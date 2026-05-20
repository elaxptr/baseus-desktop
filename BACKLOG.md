# Backlog

Post-v1 features, intentionally deferred.

## Experimental model work (Inspire XH1 pilot)

- [ ] **Verify XH1 BLE UUIDs** — candidate UUIDs from APK analysis (`0000ae0x` family); confirm with nRF Connect on real hardware
- [ ] **Verify XH1 ANC wire format** — opcode 0x68 candidate from APK; confirm mode bytes (0x08/0x09/0x0A for Commute/Outdoor/Indoor) via Frida hook
- [ ] **Implement XH1 ANC SET** — once wire format confirmed, wire `execute_command` for adaptive modes
- [ ] **XH1 HomeTab** — show single headphone battery card instead of L/R/case grid
- [ ] **Promote XH1 from Experimental → Verified** — after ANC + battery confirmed by an owner

## Other Baseus models

- [ ] Add more models using `tools/extract_apk_model.py` — the workflow is now established; any contributor can add a model as a draft PR

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
