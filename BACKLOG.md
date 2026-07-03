# Backlog

Post-v1 features, intentionally deferred.

## Protocol capture tooling (in-app Capture Studio)

The APK-derived Inspire XH1/XP1/XC1 models were removed (never hardware-verified). The
replacement is tooling that lets real owners capture and contribute *verified* support:

- [ ] **In-app Capture Studio** — a dev mode in the desktop app: scan any BLE device, auto-detect notify/write chars, live hex log, guided "toggle ANC now → captured N frames", export a shareable capture bundle. (Owns its own design spec.)
- [ ] **Declarative model format** — define models as data (UUIDs, opcode→event map, frame layout) read by a generic decoder, so adding a device is data + a golden test, not a Rust module. The Capture Studio can auto-draft it.
- [ ] Re-add community-contributed models through the above, each verified on real hardware.

## BP1 Pro ANC remaining work

## BP1 Pro ANC remaining work

- [ ] Touch gesture remapping (opcode `0x92` likely candidate — needs full protocol capture)
- [ ] Custom EQ (per-band sliders, not just presets)
- [ ] Confirm EQ preset `0x03` (Clear) — extrapolated, not yet observed in captures
- [ ] Bud charging state — no captured frame yet; currently hardcoded false

## Platform + infrastructure

- [ ] Firmware version display
- [ ] Multi-device support
- [ ] Linux support (BlueZ transport)
- [ ] macOS support (IOBluetooth transport)
