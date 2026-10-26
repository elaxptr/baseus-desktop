# Changelog

All notable changes to this project are documented here. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/); versions match the git tags.

## [Unreleased]

### Added
- Tray menu shows the connected model and battery levels and switches noise control
  (Off / Noise cancellation / Transparency); the tray tooltip shows the same summary.

### Fixed
- Changes made while the earbuds were disconnected were queued and replayed on the next
  connection. They're now refused, and the controls are disabled until the earbuds connect.
- The Home tab no longer shows 0 % batteries while disconnected.
- A change the earbuds didn't take now shows an error and reverts the control instead of
  failing silently.
- Removed the decorative macOS-style window buttons from the title bar.

## [0.4.1] — 2026-10-26

### Fixed
- Releases now include the signed `latest.json` that the in-app updater reads. No
  earlier release had it, so the updater never found an update; versions up to 0.4.0
  have to install this one by hand.

## [0.4.0] — 2026-10-26

First public release since 0.2.1: 0.3.0 was never published as a release, so its
changes (below) also ship with this version.

### Added
- **Bass BP1 Ultra support** (firmware 1.0.26): battery (L/R/case), noise control
  (Normal / ANC with 5 levels / Transparency), 4 EQ presets and low-latency mode.
  Captured from the official app's SPP traffic and verified over BLE on a physical unit
  — see `docs/protocol/bp1-ultra.md`.
- CRC frame support in `baseus-protocol` (`framing::crc`: `78 9C` envelope with a
  CRC16/MODBUS trailer) and a `WireFormat` per model.
- The BP1 Pro and BP1 Ultra share a GATT service, so the connected model is now
  identified from the first notification's envelope instead of the scan entry.
- Per-model capabilities in the `model-info` event: discrete ANC levels, which modes take a
  strength, find-my-earbuds availability, device preset names.
- **Connection help on the home tab** while no earbuds are connected. It says why the
  last attempt failed instead of sitting on "Connecting…": Bluetooth off, nothing found,
  earbuds found but the control channel refused (phone app or dual-device mode holding
  it, [#8](https://github.com/elaxptr/baseus-desktop/issues/8)), or a nearby Baseus
  model that isn't supported yet, with links to help add it
  ([#10](https://github.com/elaxptr/baseus-desktop/issues/10),
  [#2](https://github.com/elaxptr/baseus-desktop/issues/2),
  [#5](https://github.com/elaxptr/baseus-desktop/issues/5)). Idea from
  [#7](https://github.com/elaxptr/baseus-desktop/pull/7).
- [docs/capture-your-model.md](docs/capture-your-model.md): a no-root, no-PC guide for
  owners of unsupported models to record their earbuds' protocol on an Android phone. The
  app's "Help add this model" button opens it.

### Fixed
- The window stayed on "Connecting…" when the earbuds connected before the webview
  subscribed to events (a cached peripheral can connect within a second of launch). The
  backend now keeps a snapshot that the frontend reads after subscribing.
- A manual launch now opens the window. Only launch-at-login (`--minimized`) starts in
  the tray; before, every launch went straight to the tray and looked like the app had
  closed itself ([#1](https://github.com/elaxptr/baseus-desktop/issues/1)).
- Closing the window hides it to the tray instead of quitting, so the BLE link and
  low-battery alerts keep running; use the tray menu's **Quit** to exit.
- The title bar no longer claims "Bass BP1 Pro ANC" before any earbuds are connected.
- Update check and install failures are now shown in Settings, with a retry button.
  Before, a failed check silently reset the panel. The install panel also stays visible
  while installing instead of disappearing as soon as you click Install
  ([#7](https://github.com/elaxptr/baseus-desktop/pull/7)).
- More BP1 Pro name variants are recognized ("Baseus Bass BP1 Pro", "… ANC"), and name
  matching ignores surrounding whitespace ([#7]).

## 0.3.0 — 2026-07-03 (not released separately)

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

[0.4.0]: https://github.com/elaxptr/baseus-desktop/releases/tag/v0.4.0

## [0.2.1] and earlier

See the git history and release tags (`v0.2.1`, `v0.2.0`, `v0.1.0`). This changelog
was introduced in 0.3.0.
