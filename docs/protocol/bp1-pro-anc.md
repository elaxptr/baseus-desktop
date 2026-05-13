# Baseus Bass BP1 Pro ANC — Packet Table

## Device identification

| Field | Value |
|---|---|
| App product name | "BP1 Ultra" (seen in APK strings) |
| Chip vendor | Bluetrum (蓝特无线), CCSDK v? |
| Secondary OTA chip | JieLi (杰理) via jl_bt_ota SDK |
| BT profiles | BLE GATT (control) + Classic SPP (OTA only) |
| App package | `com.baseus.intelligent` |
| Control protocol | Bluetrum CCCOMM over BLE GATT |

## BLE GATT endpoints

| Role | UUID |
|---|---|
| Service | `02F00000-0000-0000-0000-00000000FE00` |
| Write | `02F00000-0000-0000-0000-00000000FF01` |
| Notify | `02F00000-0000-0000-0000-00000000FF02` |

See `framing.md` for general framing format.

## Battery / Power state

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

Command to query battery: `get_device_power_volume`
Notification event: `DEVICE_POWER_VOLUME_RESP` / `DEVICE_POWER_VOLUME`

**Wire bytes**: TODO — requires live BLE capture.

Likely response structure (TBD):
```
[ CMD_CATEGORY ][ CMD_ID ][ LEN_L ][ LEN_H ]
[ left_level : u8 ][ left_charging : u8 ]
[ right_level : u8 ][ right_charging : u8 ]
[ case_level  : u8 ][ case_charging  : u8 ]
```

## ANC mode

SDK enum: `com.bluetrum.cccomm.data.api.AncMode`

The `BleCommandUtil.resolveModify0C` method (class name suffix `0C` = command byte 0x0C)
processes ANC-related responses. Modes visible in UI strings: OFF, ANC (active noise
cancellation), TRANSPARENCY.

**Wire bytes**: TODO — requires live BLE capture.

## Gesture / key configuration

SDK: `com.bluetrum.cccomm.data.api.KeyType`, `KeyFunction`
Log string: `ear_detection_switch_set_key`, `ear_factory_restoration_set_key`

## In-case detection

SDK: `InCaseStatus(leftSideIn=..., rightSideIn=...)`

## EQ

Not yet analyzed.

## How to fill in TODO sections

1. Capture a live BLE session using Android HCI snoop log or Wireshark + BLE sniffer.
2. Pair BP1 Pro ANC to an Android phone running the Baseus app.
3. Enable "Bluetooth HCI snoop log" in Developer Options.
4. Open Baseus app, open case lid (triggers battery notification), toggle ANC.
5. Pull `btsnoop_hci.log` and filter in Wireshark for ATT writes to handle for `FF01`
   and ATT notifications from handle for `FF02`.
6. Record exact byte sequences and fill in the wire byte sections above.
7. Add golden round-trip test cases to `crates/baseus-protocol/src/models/bp1_pro_anc.rs`.
