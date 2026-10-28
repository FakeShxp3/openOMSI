# Optional onward passenger connections — experimental

MIT-licensed source reference: Mayconrib808/OpenOmsi-BBS 0.3.45-coop.15-2.2.1.8 (credit retained).

Implementation:
- `passenger_connections=0` by default; `passenger_transfer_percent=20` is inert until enabled.
- Uses the original passenger id and changes the task to walk to an onward stop (no new passenger spawn).
- Up to two transfers per person, 80m candidate radius, original bus excluded from immediate re-boarding.
- Only single-player is enabled. **LAN clients and servers are explicitly held at 0%.** This avoids Maycon's documented missing client-owned passenger authority handback. Multiplayer must be reviewed and implemented separately.
- Validation required: map path reachability beyond direct collision ray, real map compatibility, Rust tests, renderer test, performance and reconnect ownership. This branch is not merge-ready.
