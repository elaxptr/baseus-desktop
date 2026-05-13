# baseus-desktop

Open-source Windows desktop client for Baseus earbuds.
Displays live battery (L / R / case) from your Baseus Bass BP1 Pro ANC — and eventually other models.

Built by reverse-engineering the official Baseus Android app. Protocol documentation and
Frida capture scripts live in [`docs/protocol/`](docs/protocol/).

## Status

🚧 **v0 — in development.** Not yet functional.

## Requirements

- Windows 10 1903+ (WinRT Bluetooth APIs)
- Baseus Bass BP1 Pro ANC earbuds (v1 only; architecture supports adding models)

## Building

```
cargo build
cd apps/baseus-app && pnpm install && pnpm tauri build
```

## Contributing

See [docs/re-methodology.md](docs/re-methodology.md) to add support for your Baseus model.
