# Changelog

All notable changes to this project are documented here. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/); versions match the git tags.

## [0.3.0] — 2026-07-03

### Added
- **Game / low-latency mode** for the BP1 Pro — a toggle that sends `BA 24 01/00`
  and tracks state from the device's `AA 23` confirmation. Community-verified over
  both SPP and BLE in [#3](https://github.com/elaxptr/baseus-desktop/issues/3).
- **UX motion pass** — an animated game-mode switch (sliding knob + accent halo),
  battery rings that sweep and count up, a gliding sidebar indicator, spring tab
  transitions, and hover/press/ripple micro-interactions. All motion respects
  `prefers-reduced-motion`.

### Changed
- ANC ack handling is now firmware-tolerant: some units ack every ANC command with
  a flat `AA 34 01` (including Off), so state is resolved against the last commanded
  mode rather than trusting the ack payload as a mode value ([#3]).
- Numeric readouts (battery %, ANC strength, session timer) use a monospace face.

### Removed
- **Inspire XH1 / XP1 / XC1 support.** These were extracted from the Baseus Android
  APK and never confirmed on real hardware. Rather than ship unverified promises,
  they've been removed. The model registry is kept (BP1-only) as the extension point
  for future owner-contributed, hardware-verified models — see `BACKLOG.md` for the
  planned in-app Capture Studio and declarative model format.

[0.3.0]: https://github.com/elaxptr/baseus-desktop/releases/tag/v0.3.0

## [0.2.1] and earlier

See the git history and release tags (`v0.2.1`, `v0.2.0`, `v0.1.0`). This changelog
was introduced in 0.3.0.
