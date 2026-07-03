# Reverse-Engineering Methodology

## Prerequisites
- MuMuPlayer (or any Android emulator with Bluetooth passthrough)
- Frida server (arm64 build matching emulator ABI) pushed to `/data/local/tmp/frida-server`
- ADB connected: `adb devices` shows the emulator
- Baseus app installed on emulator; BP1 Pro ANC paired to host's Bluetooth adapter

## Steps

### 1. Enable Bluetooth HCI snoop log
In MuMuPlayer's Android settings: **Developer Options → Enable Bluetooth HCI snoop log**.
The log appears at `/sdcard/btsnoop_hci.log`.

Verify:
```
adb devices
adb shell getprop persist.bluetooth.btsnoopenable
# → true
```

### 2. Start Frida server
```
adb shell "chmod +x /data/local/tmp/frida-server && /data/local/tmp/frida-server &"
```

### 3. Identify Baseus app package name
```
adb shell pm list packages | grep baseus
# → com.baseus.headset  (or similar)
```

### 4. Attach Frida hook
```
frida -U -f com.baseus.headset -l docs/frida/socket-trace.js --no-pause 2>&1 | tee docs/protocol/captures/session-$(date +%Y%m%d-%H%M%S).log
```

### 5. Execute stimulus sequence (note timestamps)
In order, ~3 seconds between each action:
1. Open app cold → watch for handshake bytes
2. Open earbuds case lid → expect battery notification
3. Toggle ANC: off → on → transparency → off
4. Remove one bud from ear
5. Close case, let disconnect happen

### 6. Pull btsnoop HCI log
```
adb pull /sdcard/btsnoop_hci.log docs/protocol/captures/btsnoop-$(date +%Y%m%d).log
```
Open in Wireshark. Filter: `btrfcomm || avctp || a2dp`

### 7. Sanitise MAC addresses before committing
```powershell
# PowerShell:
(Get-Content docs\protocol\captures\session-01.log) -replace '[0-9a-fA-F]{2}(:[0-9a-fA-F]{2}){5}', 'XX:XX:XX:XX:XX:XX' | Set-Content docs\protocol\captures\session-01.log
```

## What to look for

> The confirmed BP1 Pro packet table lives in
> [`docs/protocol/bp1-pro-anc.md`](protocol/bp1-pro-anc.md) — treat that as the source of truth.
> Note the `02F0…` UUIDs that appear in APK static analysis are a **decoy**: they belong to a
> different Bluetrum device variant and are *not* what the BP1 Pro actually uses.

- **Bluetooth profile**: The BP1 Pro ANC uses **BLE GATT** for runtime control (and accepts the
  same command bytes over Classic SPP/RFCOMM — see issue #3). Filter Wireshark for `btatt`.
- **GATT service (confirmed)**: `53527aa4-29f7-ae11-4e74-997334782568`
- **Write characteristic**: `ee684b1a-1e9b-ed3e-ee55-f894667e92ac` (app → device commands)
- **Notify characteristic**: `654b749c-e37f-ae1f-ebab-40ca133e3690` (device → app events)
- **Frame shape**: `AA <cmd> <payload…>` for notifications, `BA <cmd> <payload…>` for writes —
  no length field or CRC on the BLE side.
- **Battery**: opening the case lid triggers a notify; `AA 02 <L%> 00 <R%> 01`.
- **ANC**: toggling ANC writes `BA 34 <mode> <level>`; the device acks with `AA 34 …`.
- **Frida note**: the Java bridge does not work on x86_64 Android 12 emulators with Frida 17.x.
  Use a **real Android device** for Frida captures, or use the Android HCI snoop log only.

## Adding support for a new model

**Verified-on-hardware only.** This project deliberately does not ship APK-guessed models
(the earlier Inspire XH1/XP1/XC1 drafts were removed for exactly this reason). If you own a
Baseus device that isn't the BP1 Pro:

1. Run the capture procedure above with your device paired, and confirm every value against
   real notifications — don't rely on APK candidates alone.
2. Create `docs/protocol/<your-model>.md` with the packet table and commit sanitised captures
   to `docs/protocol/captures/`.
3. Create `crates/baseus-protocol/src/models/<your_model>.rs` implementing `decode_frame`,
   with golden tests built from your real captures.
4. Register your model in `BaseusModel` (`crates/baseus-protocol/src/types.rs`) — advertising
   name(s), `gatt_uuids()`, and command encoding in `execute_command`.
5. Open a PR titled `feat: add <model> protocol`, with a screenshot of the app showing your
   device's live battery/ANC.

> **Coming soon:** an in-app **Capture Studio** (scan any device, live hex log, guided capture,
> export a shareable bundle) plus a **declarative model format** so adding a device becomes data
> + a golden test rather than a hand-written module. See `BACKLOG.md`.
