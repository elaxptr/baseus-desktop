# Baseus Earbuds — General Protocol Framing

## Overview

Baseus earbuds that use the in-house Bluetrum CCSDK (Control Center SDK) communicate with Android
over **BLE GATT** for all runtime control functions (battery, ANC, EQ, gestures). Classic
Bluetooth SPP/RFCOMM is present in the APK only for JieLi-chip OTA firmware updates.

## Transport: BLE GATT

| Role | UUID |
|---|---|
| Service | `02F00000-0000-0000-0000-00000000FE00` |
| Write (app → device) | `02F00000-0000-0000-0000-00000000FF01` |
| Notify (device → app) | `02F00000-0000-0000-0000-00000000FF02` |
| OTA / extra | `02F00000-0000-0000-0000-00000000FF05` |

These UUIDs are used by the Bluetrum CCSDK (`com.bluetrum.ccsdk`), confirmed from APK static
analysis (classes2.dex).

## Packet Format

The on-wire packet format is proprietary to the Bluetrum CCSDK and its inner classes are heavily
obfuscated (single-letter names `a0`, `b1`, …). The exact byte layout **requires a live BLE
capture** to confirm.

### Known packet structure (inferred from SDK API and log strings)

```
[ CMD_CATEGORY : 1 ]
[ CMD_ID       : 1 ]
[ DATA_LEN     : 2 ] (little-endian)
[ PAYLOAD      : DATA_LEN bytes ]
```

The SDK internally builds packets in `CCSdkApi` and dispatches them to the write characteristic.
Response notifications on `FF02` are parsed back by the SDK into high-level API objects.

### Authentication / Handshake

Before any command can be sent, the CCSDK performs a challenge-response handshake:

```
App → Device : DevAuthChallenge (random nonce)
Device → App : DevAuthResponse  (HMAC of nonce + shared secret)
App → Device : AppAuthResponse  (app's reply)
Device → App : AppAuthResult    (auth OK / FAILED)
```

This is implemented via `com.bluetrum.cccomm.auth.IAuthenticator`. The `ExampleAuthenticator`
in the CC SDK demo implements the simplest form. The shared secret is likely derived from the
device's MAC address or serial number.

**TODO**: Capture the auth bytes from a live session.

## SPP / RFCOMM (OTA only)

Classic Bluetooth SPP UUID `00001101-0000-1000-8000-00805F9B34FB` is present in the APK and
handled exclusively by the JieLi OTA SDK (`com.jieli.jl_bt_ota`, package `com.jieli.otasdk`).
The JieLi RCSP protocol is used for firmware file transfer and is NOT part of the normal
control path.

## How to capture live traffic

Option A — Android HCI snoop log:
1. Enable developer mode on Android.
2. Enable **Bluetooth HCI snoop log** in Developer Options.
3. Open Baseus app, pair earbuds, perform operations.
4. `adb pull /sdcard/btsnoop_hci.log`
5. Open in Wireshark; filter `btle` and look for writes to characteristic `FF01`.

Option B — Frida socket-trace on real device (not emulator):
- Use `docs/frida/socket-trace.js` on a real Android device with USB debugging.
- Frida 17.x Java bridge works on real devices; it does not work reliably on x86_64
  Android 12 emulators (confirmed limitation).
