# Baseus Bass BP1 Pro ANC — Packet Table

## Device identification

| Field | Value |
|---|---|
| App product name | "BP1 Ultra" (seen in APK strings) |
| Chip vendor | Bluetrum (蓝特无线), CCSDK v? |
| Secondary OTA chip | JieLi (杰理) via jl_bt_ota SDK |
| BT profiles | BLE GATT (control) + Classic SPP (OTA only) |
| App package | `com.baseus.intelligent` |
| Control protocol | Proprietary BLE GATT (not Bluetrum CCCOMM 02F0 UUIDs) |
| BLE device name | `Bass BP1 Pro` |
| BLE MAC (test unit) | `4A:01:CE:BA:C8:03` |

## BLE GATT endpoints

Confirmed via nRF Connect on a physical BASS BP1 PRO unit.
The `02F0…` UUIDs found in APK static analysis belong to a different Bluetrum device variant.

| Role | UUID |
|---|---|
| Service | `53527aa4-29f7-ae11-4e74-997334782568` |
| Write | `ee684b1a-1e9b-ed3e-ee55-f894667e92ac` (WRITE only) |
| Notify | `654b749c-e37f-ae1f-ebab-40ca133e3690` (NOTIFY + READ) |

## Frame format

All frames (both app→device writes and device→app notifications) begin with magic byte `0xAA`.
The second byte encodes the command/event category. Remaining bytes are category-specific payload.
No fixed-length header or explicit length field observed — frame length is determined by the GATT
notification or write PDU length.

```
[ 0xAA ] [ CMD ] [ payload... ]
```

## ANC mode notifications  (device → app, notify char)

Second byte high nibble `0x3` = ANC event family. Low nibble encodes the mode.

| Captured bytes | ANC state |
|---|---|
| `AA 30 00` | ANC off (default) |
| `AA 32 02 FF` | Transparency |
| `AA 33 01 68` | ANC active |

Byte 3+ interpretation is pending further analysis. `0xFF` and `0x68` may be
strength/level parameters or checksums — need write-side captures to confirm.

## Battery notifications  (device → app, notify char)

CMD byte `0x02` = bud battery report (left + right).

```
AA 02 [left_pct: u8] 0x00 [right_pct: u8] 0x01
```

`0x00` and `0x01` are fixed bud-ID markers (left=0, right=1), **not** charging flags.
Charging state for individual buds is not present in this frame; source TBD.

Confirmed live capture (both buds in ear, 100%):
```
AA 02 64 00 64 01
```

## Case battery notification  (device → app, notify char)

CMD byte `0x27` = case battery report.

```
AA 27 [case_pct: u8] [case_charging: u8]
```

`case_charging`: `0x00` = not charging, `0x01` = charging (in charger).

Confirmed live capture (case at 50%, not plugged in):
```
AA 27 32 00
```

## Case / connection event  (device → app, notify char)

CMD byte `0x80` = case/connection event (observed on case-close).

```
AA 80 01 4A 01 A8 EF BF A9   (example — case closed)
```

Byte layout not fully decoded. Separate from battery — does not carry case_pct.

## Battery / Power state — SDK model

SDK class: `com.bluetrum.cccomm.data.api.DevicePower`

```kotlin
data class DevicePower(
    val leftSidePower:  ComponentPower,
    val rightSidePower: ComponentPower,
    val casePower:      ComponentPower,
)
data class ComponentPower(
    val powerLevel: Int,    // 0–100 (percent)
    val isCharging: Boolean,
)
```

The `AA 02` frame covers left and right buds. Case battery may arrive via the `AA 80`
frame or a separate notification not yet captured.

## Outstanding TODOs

- [x] Confirm battery % values against Baseus app display
- [x] Identify case battery notification opcode (`0x27`)
- [x] Add golden test cases to `crates/baseus-protocol/src/models/bp1_pro_anc.rs`
- [ ] Capture bud charging state — need a live frame with a bud in-case charging to identify the byte
- [ ] Decode `AA 80` case event fully (trailing bytes purpose unknown)
- [ ] Capture write-side frames (app → device) to confirm ANC set commands
- [ ] Determine checksum/trailing byte semantics in ANC frames (`0xFF`, `0x68`)

## Capture methodology

Protocol observed via nRF Connect for Mobile (free, Play Store):
1. Connect to `BASS BP1 PRO` BLE device
2. Find service `53527aa4-…`
3. Enable notifications on `654b749c-…` (NOTIFY char)
4. Interact (toggle ANC, open/close case) and read raw notification values

For write-side captures, use nRF Connect's write UI on `ee684b1a-…` or
run `docs/frida/socket-trace.js` via Frida on a real Android device.
