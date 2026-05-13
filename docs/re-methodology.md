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
- **Bluetooth profile**: Check btsnoop for RFCOMM (UIH frames) vs ATT/GATT traffic.
- **Service UUID**: Visible in the SDP query or GATT service discovery exchange.
- **Framing format**: Look for repeating byte patterns at packet starts. Most TWS protocols: `<magic0> <magic1> <len> <cmd> <payload...> <crc>`.
- **Checksum**: CRC-8 (poly 0x07), XOR of all preceding bytes, or other.

## Adding support for a new model
1. Run the capture procedure above with your device paired.
2. Create `docs/protocol/<your-model>.md` with the packet table.
3. Create `crates/baseus-protocol/src/models/<your_model>.rs` implementing `decode_frame`.
4. Add your model variant to `BaseusModel` in `crates/baseus-protocol/src/types.rs`.
5. Open a PR with captures committed to `docs/protocol/captures/`.
