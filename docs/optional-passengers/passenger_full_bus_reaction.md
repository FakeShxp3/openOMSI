# Full-bus reaction (experimental)

MIT source reference: Mayconrib808/OpenOmsi-BBS, 0.3.45-coop.15-2.2.1.8.

- OFF by default and independent of opt-in hailing; original OMSI animation untouched.
- Counts usable seated AND standing [passpos] locations, excluding script-disabled places; no false full state for a zero-capacity/unknown model.
- Uses a short 1.4-second visual reaction, then suppresses repetition until the encounter ends or the bus has capacity again.
- Only affects procedural animation; boarding decisions and route tasks remain authoritative and unchanged.
- Multiplayer behavior, actual Windows graphics, performance and Cargo tests still require review. Not merge-ready.
