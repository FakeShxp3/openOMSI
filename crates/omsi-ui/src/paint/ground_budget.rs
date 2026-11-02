//! A dense map loses detail uniformly, not everything drawn after the first full layer.
//! Exact vertices are kept until the budget is exceeded. Then all triangles, old and new,
//! are composited at the view's scale. Equal cells become rectangles; only a grid whose
//! rectangles exceed the vertex budget is coarsened. No GPU or readback needed.

use super::Vertex;
use glam::Vec2;

pub(super) struct GroundBudget {
    limit: usize,
    reach: f32,
    mpp: f32,
    grid: Option<Grid>,
}

impl GroundBudget {
    pub fn new(limit: usize, reach: f32, mpp: f32) -> Self {
        assert!(limit >= 6, "a ground budget must hold one quad");
        Self {
            limit,
            reach: if reach.is_finite() && reach > 0.0 {
                reach
            } else {
                1.0
            },
            mpp: if mpp.is_finite() { mpp.max(0.0) } else { 0.0 },
            grid: None,
        }
    }

    pub fn reduced(&self) -> bool {
        self.grid.is_some()
    }

    pub fn clear(&mut self) {
        self.grid = None;
    }

    pub fn push(&mut self, exact: &mut Vec<Vertex>, vertices: &[Vertex]) {
        debug_assert_eq!(vertices.len() % 3, 0);
        if self.grid.is_none() && exact.len().saturating_add(vertices.len()) <= self.limit {
            exact.extend_from_slice(vertices);
            return;
        }
        let grid = self
            .grid
            .get_or_insert_with(|| Grid::new(self.limit, self.reach, self.mpp));
        for t in exact.chunks_exact(3) {
            grid.triangle(t);
        }
        *exact = Vec::new();
        for t in vertices.chunks_exact(3) {
            grid.triangle(t);
        }
    }

    pub fn finish(self, exact: Vec<Vertex>) -> Vec<Vertex> {
        match self.grid {
            Some(grid) => {
                drop(exact);
                grid.vertices()
            }
            None => exact,
        }
    }
}

struct Grid {
    limit: usize,
    side: usize,
    reach: f32,
    mpp: f32,
    /// sRGB, straight RGBA. Even a 4096-square build uses at most 64 MiB on the worker.
    cells: Vec<[u8; 4]>,
    /// Land classes and road passes repeat one colour over many thousands of triangles.
    last_color: Option<([[f32; 4]; 3], [f32; 4], [u8; 4])>,
}

struct Rectangle {
    left: usize,
    right: usize,
    top: usize,
    bottom: usize,
    color: [u8; 4],
}

impl Grid {
    fn new(limit: usize, reach: f32, mpp: f32) -> Self {
        let pixels = if mpp > 0.0 {
            2.0 * reach / mpp
        } else {
            ((limit / 6) as f32).sqrt().min(512.0)
        };
        let side = (pixels.ceil().clamp(1.0, 4096.0) as usize).next_power_of_two();
        Self {
            limit,
            side,
            reach,
            mpp,
            cells: vec![[0; 4]; side * side],
            last_color: None,
        }
    }

    fn triangle(&mut self, t: &[Vertex]) {
        let p: [Vec2; 3] = std::array::from_fn(|i| {
            let v = t[i];
            let at = Vec2::new(v.pos[0], v.pos[1])
                + Vec2::from(v.ext) * v.width[0].max(v.width[1] * self.mpp);
            (at / self.reach + Vec2::ONE) * (self.side as f32 * 0.5)
        });
        if p.iter().any(|p| !p.is_finite())
            || t.iter().any(|v| v.color.iter().any(|c| !c.is_finite()))
        {
            return;
        }
        if (p[1] - p[0]).perp_dot(p[2] - p[0]).abs() < 1e-10 {
            return;
        }
        let low = p
            .iter()
            .map(|p| p.y)
            .fold(f32::INFINITY, f32::min)
            .floor()
            .max(0.0) as usize;
        let high = p
            .iter()
            .map(|p| p.y)
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            .clamp(0.0, self.side as f32) as usize;
        let key = [t[0].color, t[1].color, t[2].color];
        let (color, packed) = self
            .last_color
            .filter(|(old, _, _)| *old == key)
            .map(|(_, color, packed)| (color, packed))
            .unwrap_or_else(|| {
                let color: [f32; 4] = std::array::from_fn(|k| {
                    t.iter()
                        .map(|v| {
                            if k == 3 {
                                v.color[k].clamp(0.0, 1.0)
                            } else {
                                linear(v.color[k])
                            }
                        })
                        .sum::<f32>()
                        / 3.0
                });
                let premultiplied = [
                    color[0] * color[3],
                    color[1] * color[3],
                    color[2] * color[3],
                    color[3],
                ];
                (color, pack(premultiplied))
            });
        self.last_color = Some((key, color, packed));
        // Conservative coverage keeps a narrow road even when no cell centre falls in it.
        // Clip each triangle to a row's slab before finding its x span, not its whole box.
        for y in low..high {
            let Some((a, b)) = row_span(p, y as f32, (y + 1) as f32) else {
                continue;
            };
            let a = a.floor().clamp(0.0, self.side as f32) as usize;
            let b = b.ceil().clamp(0.0, self.side as f32) as usize;
            let cells = &mut self.cells[y * self.side + a..y * self.side + b];
            if color[3] == 1.0 {
                cells.fill(packed);
                continue;
            }
            for dst in cells {
                let mut blended = unpack(*dst);
                for k in 0..3 {
                    blended[k] = color[k] * color[3] + blended[k] * (1.0 - color[3]);
                }
                blended[3] = color[3] + blended[3] * (1.0 - color[3]);
                *dst = pack(blended);
            }
        }
    }

    fn rectangles(&self, mut visit: impl FnMut(Rectangle)) {
        let mut active: Vec<Rectangle> = Vec::new();
        let mut next = Vec::new();
        for y in 0..self.side {
            let row = &self.cells[y * self.side..(y + 1) * self.side];
            let mut previous = active.drain(..).peekable();
            let mut x = 0;
            while x < self.side {
                let color = row[x];
                let first = x;
                x += 1;
                while x < self.side && row[x] == color {
                    x += 1;
                }
                if color[3] == 0 {
                    continue;
                }
                while previous.peek().is_some_and(|r| r.left < first) {
                    visit(previous.next().unwrap());
                }
                if previous.peek().is_some_and(|r| r.left == first) {
                    let mut rect = previous.next().unwrap();
                    if rect.right == x && rect.color == color {
                        rect.bottom = y + 1;
                        next.push(rect);
                        continue;
                    }
                    visit(rect);
                }
                next.push(Rectangle {
                    left: first,
                    right: x,
                    top: y,
                    bottom: y + 1,
                    color,
                });
            }
            previous.for_each(&mut visit);
            std::mem::swap(&mut active, &mut next);
        }
        active.into_iter().for_each(visit);
    }

    fn coarsen(&mut self) {
        let small = self.side / 2;
        for y in 0..small {
            for x in 0..small {
                let mut color = [0.0; 4];
                for dy in 0..2 {
                    for dx in 0..2 {
                        let c = unpack(self.cells[(2 * y + dy) * self.side + 2 * x + dx]);
                        for k in 0..4 {
                            color[k] += c[k] * 0.25;
                        }
                    }
                }
                self.cells[y * small + x] = pack(color);
            }
        }
        self.side = small;
        self.cells.truncate(small * small);
    }

    fn vertices(mut self) -> Vec<Vertex> {
        loop {
            let mut count = 0;
            self.rectangles(|_| count += 6);
            if count > self.limit {
                self.coarsen();
                continue;
            }
            let mut out = Vec::with_capacity(count);
            let coordinate = |i: usize| (2.0 * (i as f32 / self.side as f32) - 1.0) * self.reach;
            self.rectangles(|r| {
                let v = |x: usize, y: usize| Vertex {
                    pos: [coordinate(x), coordinate(y), 0.0],
                    color: r.color.map(|c| c as f32 / 255.0),
                    mode: [1.0, 0.0],
                    ..Default::default()
                };
                let (a, b, c, d) = (
                    v(r.left, r.top),
                    v(r.right, r.top),
                    v(r.right, r.bottom),
                    v(r.left, r.bottom),
                );
                out.extend([a, b, c, a, c, d]);
            });
            return out;
        }
    }
}

fn pack(c: [f32; 4]) -> [u8; 4] {
    if c[3] <= 0.0 {
        return [0; 4];
    }
    [
        srgb(c[0] / c[3]),
        srgb(c[1] / c[3]),
        srgb(c[2] / c[3]),
        c[3],
    ]
    .map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)
}

fn unpack(c: [u8; 4]) -> [f32; 4] {
    let c = c.map(|v| v as f32 / 255.0);
    [
        linear(c[0]) * c[3],
        linear(c[1]) * c[3],
        linear(c[2]) * c[3],
        c[3],
    ]
}

/// The x extent of a triangle intersecting the horizontal strip [low, high].
fn row_span(p: [Vec2; 3], low: f32, high: f32) -> Option<(f32, f32)> {
    let (mut left, mut right) = (f32::INFINITY, f32::NEG_INFINITY);
    let mut take = |x: f32| {
        left = left.min(x);
        right = right.max(x);
    };
    for i in 0..3 {
        let (a, b) = (p[i], p[(i + 1) % 3]);
        if a.y >= low && a.y <= high {
            take(a.x);
        }
        for y in [low, high] {
            if (a.y < y && b.y >= y) || (b.y < y && a.y >= y) {
                take(a.x + (b.x - a.x) * ((y - a.y) / (b.y - a.y)));
            }
        }
    }
    (left <= right).then_some((left, right))
}

fn linear(c: f32) -> f32 {
    let c = c.clamp(0.0, 1.0);
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn srgb(c: f32) -> f32 {
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

#[cfg(test)]
mod tests;
