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
- **Bluetooth profile**: The BP1 Pro ANC uses **BLE GATT** (not RFCOMM) for all runtime
  control. Filter Wireshark for `btatt` and look for ATT writes to the Bluetrum CCSDK
  service `02F00000-0000-0000-0000-00000000FE00`.
- **Write characteristic**: `02F00000-0000-0000-0000-00000000FF01` (app → device commands)
- **Notify characteristic**: `02F00000-0000-0000-0000-00000000FF02` (device → app events)
- **Auth handshake**: Look for the challenge-response exchange immediately after GATT
  service discovery. The pattern involves a DevAuthChallenge and DevAuthResponse.
- **Battery command**: Triggered by opening the case lid; look for notify on `FF02` with
  three power level bytes (left, right, case) and three charging-state bytes.
- **ANC command**: Toggling ANC from the UI causes a write to `FF01`; command byte appears
  to be `0x0C` based on `BleCommandUtil.resolveModify0C` method name in the APK.
- **Frida note**: Java bridge does not work on x86_64 Android 12 emulators with Frida 17.x.
  Use a **real Android device** for Frida captures, or use Android HCI snoop log only.

## Adding support for a new model

### Option A: Full RE from scratch (you own the device)
1. Run the capture procedure above with your device paired.
2. Create `docs/protocol/<your-model>.md` with the packet table.
3. Create `crates/baseus-protocol/src/models/<your_model>.rs` implementing `decode_frame`.
4. Add your model variant to `BaseusModel` in `crates/baseus-protocol/src/types.rs`.
5. Open a PR with captures committed to `docs/protocol/captures/`.

### Option B: APK-extracted draft (you own the device but skip deep RE)
1. Run `python tools/extract_apk_model.py --name "Baseus <Your Model>"` and review the output.
2. Install the app and let it connect to your device.
3. In nRF Connect, find your device and list its GATT services. Identify the service, write, and
   notify UUIDs. These override the APK candidates in `types.rs`.
4. Subscribe to the notify characteristic and interact with the device (open case, toggle ANC,
   check battery). Capture the raw hex frames.
5. Update `docs/protocol/<your-model>.md` and `crates/baseus-protocol/src/models/<your_model>.rs`
   with confirmed values.
6. Change `ModelStatus` from `Experimental` to `Verified` in `types.rs`.
7. Open a PR — the review checklist is in `docs/protocol/inspire-xh1.md` under "How to verify".

## Confirming an experimental model (e.g. Inspire XH1)

If the app ships an **experimental** model (yellow banner in the UI), here is how to promote it
to Verified:

1. **Install the app** and let it attempt to connect to your device.
2. **Check the logs** (`RUST_LOG=debug` in the environment) for connection attempts — you'll see
   which advertising name it scanned for and whether the GATT characteristics were found.
3. **Run nRF Connect** (free, Google Play) on an Android phone with your device paired.
   List services → note which UUID family is actually present. Update `types.rs::gatt_uuids()`.
4. **Subscribe to the notify characteristic** in nRF Connect and interact with the device:
   - Toggle ANC modes in the Baseus app → capture the `0xAA 0x??` notifications.
   - Open/close case or check battery in the Baseus app → capture battery notifications.
5. **Update `docs/protocol/<model>.md`** with confirmed byte values and remove ⚠ markers.
6. **Update `crates/baseus-protocol/src/models/<model>.rs`**:
   - Replace candidate opcodes with confirmed values.
   - Remove `// APK-EXTRACTED, UNVERIFIED` comments from confirmed lines.
   - Remove `#[ignore]` from hardware test placeholders and fill in real captures.
7. **Change `ModelStatus` to `Verified`** in `types.rs::status()`.
8. **Open a PR** — title: `feat: verify <model> protocol`. Include a screenshot of the app
   showing your device's battery/ANC with confirmed data.
