# Optional onward passenger connections — experimental

MIT-licensed source reference: Mayconrib808/OpenOmsi-BBS 0.3.45-coop.15-2.2.1.8 (credit retained).

Implementation:
- `passenger_connections=0` by default; `passenger_transfer_percent=20` is inert until enabled.
- Uses the original passenger id and changes the task to walk to an onward stop (no new passenger spawn).
- Up to two transfers per person, 80m candidate radius, original bus excluded from immediate re-boarding.
- Only single-player is enabled. **LAN clients and servers are explicitly held at 0%.** This avoids Maycon's documented missing client-owned passenger authority handback. Multiplayer must be reviewed and implemented separately.
- Candidate walks require loaded, continuous ground sampled at up to 2 m intervals (maximum 40 samples), bounded height changes and a clear collision ray. A known due service or a compatible ready bus is required; absent service data fails closed. This is deliberately conservative, not a complete pavement route planner.
- Synthetic tests cover identity/type retention, two transfers, immediate reboarding exclusion, missing service/ground and the disabled path leaving RNG untouched. Native/launcher settings and host/join/server gates are tested separately.
- Before leaving Draft: test real maps (including overpasses, barriers and streamed tiles), Windows gameplay captures, low-end frame times/memory and all supported platforms. Multiplayer remains disabled; authority handback and synchronization are outside this PR.

The original MIT copyright and permission notice are retained in [MAYCON-LICENSE.txt](MAYCON-LICENSE.txt). Source: https://github.com/Mayconrib808/OpenOmsi-BBS.
