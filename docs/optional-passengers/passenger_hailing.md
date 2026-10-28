# Visual passenger hailing (experimental)

MIT source credit: Mayconrib808/OpenOmsi-BBS, 0.3.45-coop.15-2.2.1.8.

- OFF by default, no hardcoded Brazil-only map list.
- Detects a matching approaching bus within 60 m; does not change actual boarding or AI decisions.
- Enhanced/procedural human rig only, with look and wrist/arm IK; original OMSI animation is untouched.
- No new network packets or assets; default-OFF path exits without scanning buses for passengers.
- Known limitation: mirrored remote people do not get a host-transmitted service intent, so hailing is local-only until networking is reviewed. Windows gameplay / Rust tests still required.
