# Baseus Bass BP1 Pro ANC — Packet Table

## Device identification

| Field | Value |
|---|---|
| App product name | "BP1 Ultra" (seen in APK strings) |
| Chip vendor | Bluetrum (蓝特无线), CCSDK v? |
| Secondary OTA chip | JieLi (杰理) via jl_bt_ota SDK |
| BT profiles | BLE GATT (control) + Classic SPP (control + OTA — same command bytes on both, see issue #3) |
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
A bud reports **0%** when seated in the case (device provides power, bud goes passive).

Live captures:

| Situation | Frame |
|---|---|
| Both buds in ear, 100% | `AA 02 64 00 64 01` |
| Left in case, right in ear 100% | `AA 02 00 00 64 01` |

Bud charging detection: a bud is in-case (and likely charging) when `pct == 0`
while the case itself is present and reporting via `AA 27`.

## Device identity frame  (device → app, notify char)

CMD byte `0x12` = device identity, sent once on connect.

```
AA 12 [mac_byte5..mac_byte0: 6 bytes] 00 00 00 00 00 00 01
```

MAC bytes are in reverse order. Example:
```
AA 12 03 C8 BA CE 01 4A …   →   MAC 4A:01:CE:BA:C8:03
```

Trailing bytes purpose unknown. Not decoded by the app.

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

## EQ preset commands  (app → device, write char)

CMD byte `0x43` = set EQ preset. CMD byte `0x42` = query current preset.

Source: btsnoop RFCOMM captures from bugreport ZIPs (`bugreport-baseus.zip`,
`bugreport-baseus2.zip`). The official app frames RFCOMM traffic as
`BA BA opcode payload CRC`; BLE GATT format drops the repeated magic and
trailing CRC. The device also *accepts* the bare BLE-style frames
(`BA opcode payload`, no CRC) over RFCOMM channel 1 — independently confirmed
in [issue #3](https://github.com/elaxptr/baseus-desktop/issues/3), where the
exact same command bytes worked unmodified over both transports.

```
BA 43 [preset: u8]     ← set preset
BA 42                  ← query current preset (no payload)
```

Device acknowledges with the same opcode:
```
AA 43 [preset: u8]     ← ack / notification after set
AA 42 [preset: u8]     ← query response
```

Preset values seen in captures (values 0x00–0x02 confirmed; 0x03 extrapolated):

| Value | Label |
|---|---|
| `0x00` | Balanced |
| `0x01` | Bass Boost |
| `0x02` | Voice |
| `0x03` | Clear (not yet captured — extrapolated from app UI order) |

## ANC write commands  (app → device, write char)

Confirmed from same btsnoop RFCOMM captures.

```
BA 34 [mode: u8] [level: u8]
```

| `mode` | ANC state |
|---|---|
| `0x00` | Off (`level = 0xFF`) |
| `0x01` | ANC active (`level = 0x68` default, `0x10`–`0xFF` range) |
| `0x02` | Transparency (`level = 0xFF`) |

Device responds on notify char with an `AA 34` ack. **Payload semantics vary by
firmware revision** ([issue #3](https://github.com/elaxptr/baseus-desktop/issues/3)):

- Some units echo the mode: `AA 34 00` = off, non-zero = active. Mode-specific
  `AA 33 [mode] [level]` notifications have also been observed in app captures.
- Other units answer **every** ANC command — including Off — with a flat
  `AA 34 01`, never `AA 32`/`AA 33`. On these units the payload byte carries no
  mode information.

Because of this, the client decoder treats a zero ack payload as Off and a
non-zero payload as confirming the *last commanded* mode (see
`Bp1ProAnc::resolve_anc_ack`), instead of trusting the ack byte as a mode value.

## Game / low-latency mode  (bidirectional)

Community-verified over both SPP and BLE
([issue #3](https://github.com/elaxptr/baseus-desktop/issues/3) — HCI snoop of
the official app, then confirmed live against a physical BP1 Pro).
Independent toggle — not a mutually-exclusive ANC state.

```
BA 24 [on: u8]     ← set game mode (0x01 = on, 0x00 = off)
```

Device responds with two notifications:

| Action | Outgoing | Ack(s) |
|---|---|---|
| Game Mode ON | `BA 24 01` | `AA 24 01`, then `AA 23 01` |
| Game Mode OFF | `BA 24 00` | `AA 24 01`, then `AA 23 00` |

`AA 24 01` is a flat "command received" ack (payload is `01` regardless of
on/off — mirroring the flat ANC ack convention above) and carries no state.
`AA 23 [00|01]` is the actual state confirmation and is what the client decodes
(`DeviceEvent::GameModeUpdate`).

## Outstanding TODOs

- [x] Confirm battery % values against Baseus app display
- [x] Identify case battery notification opcode (`0x27`)
- [x] Add golden test cases to `crates/baseus-protocol/src/models/bp1_pro_anc.rs`
- [x] Capture write-side frames (app → device) — confirmed via btsnoop RFCOMM
- [x] Determine ANC level semantics (`0x68` default, `0x10`–`0xFF` range)
- [x] Identify EQ preset opcode (`0x43` set, `0x42` query)
- [x] Identify game mode opcodes (`0x24` set, `0x23` state — issue #3, community-verified)
- [ ] Capture bud charging state — need a live frame with a bud in-case charging
- [ ] Decode `AA 80` case event fully (trailing bytes purpose unknown)
- [ ] Confirm EQ preset `0x03` (Clear) — not yet observed in captures
- [ ] Identify gesture command opcode (`0x92` likely gesture-related, values 01–02 seen)

## Capture methodology

Protocol observed via nRF Connect for Mobile (free, Play Store):
1. Connect to `BASS BP1 PRO` BLE device
2. Find service `53527aa4-…`
3. Enable notifications on `654b749c-…` (NOTIFY char)
4. Interact (toggle ANC, open/close case) and read raw notification values

For write-side captures, use nRF Connect's write UI on `ee684b1a-…` or
run `docs/frida/socket-trace.js` via Frida on a real Android device.
