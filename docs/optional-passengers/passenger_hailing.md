# Passengers signal approaching buses

Status: integration scaffold on a dedicated opt-in branch.

- New setting: `passenger_hailing=0` by default; legacy settings remain disabled.
- Native and Tauri launcher controls persist the setting.
- This commit implements **configuration wiring only**. The runtime behavior, validation, and multiplayer work are still required. Do not merge this branch until those changes are reviewed and tested.
- Source reference: Mayconrib808/OpenOmsi-BBS (MIT), 0.3.45-coop.15-2.2.1.8.
