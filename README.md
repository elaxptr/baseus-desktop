# baseus-desktop

Open-source Windows desktop client for Baseus earbuds, built by reverse-engineering
the official Baseus Android app.

**Supported hardware:** Baseus Bass BP1 Pro ANC  
**Platform:** Windows 10 1903+ (WinRT Bluetooth APIs)

## Features

- Live L / R / case battery with charge state indicators
- Session timer (time since buds connected)
- ANC mode switching (Off / Active Noise Cancellation / Transparency) with strength slider
- EQ preset selection (Balanced / Bass Boost / Voice / Clear)
- Find-my-buds (plays a tone on one earbud)
- Low-battery desktop notifications
- Launch at login

## Building

```
# Prerequisites: Rust stable, Node.js, pnpm
pnpm install
pnpm tauri build
```

Or for development with hot-reload:

```
pnpm tauri dev
```

## Protocol documentation

The reverse-engineering methodology and full packet tables live in [`docs/protocol/`](docs/protocol/).
Frida hook scripts used to capture BLE writes are in [`docs/frida/`](docs/frida/).

See [`docs/re-methodology.md`](docs/re-methodology.md) to add support for a new Baseus model —
each model is one file in `crates/baseus-protocol/src/models/`.

## Architecture

```
baseus_rebuild/
├── crates/
│   ├── baseus-protocol/   # Pure Rust: packet framing, types, per-model decoders
│   └── baseus-transport/  # WinRT BLE GATT transport
├── apps/
│   └── baseus-app/        # Tauri shell + SolidJS frontend
└── docs/
    ├── protocol/          # Packet tables and framing docs
    └── frida/             # BLE capture scripts
```

## Disclaimer

This project is not affiliated with or endorsed by Baseus. All trademarks belong to their respective owners.

## License

MIT
