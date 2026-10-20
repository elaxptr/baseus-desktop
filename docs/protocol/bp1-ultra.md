# Baseus Bass BP1 Ultra — Packet Table

## Device identification

| Field | Value |
|---|---|
| Product name (official app) | "Baseus Bass BP1 Ultra" |
| Firmware tested | V1.0.26 (left and right buds) |
| Official app | `com.baseus.intelligent` 2.17.0.1 |
| BLE advertising name | `BP1 Ultra` |
| BT profiles | BLE GATT (control) + Classic SPP (control, used by the official app) |
| Control protocol | Same opcode family as the BP1 Pro ANC, wrapped in a CRC frame |

The BP1 Ultra is a separate product from the BP1 Pro ANC, even though the older
`bp1-pro-anc.md` notes the "BP1 Ultra" string in the APK. They share the GATT service but
**not** the frame format, and several opcodes changed meaning.

## How this was captured and verified

1. Android 11 test phone (rooted) with the Bluetooth HCI snoop log in `full` mode, official
   Baseus app logged in as guest, every screen driven over `adb` while timestamps were logged.
2. The snoop log shows the official app talking to the buds over **SPP (RFCOMM channel 1)**,
   not BLE. The RFCOMM payloads were extracted and decoded.
3. Every frame type was then **replayed over BLE GATT from a Windows PC** (WinRT) against the
   same physical unit, and the device's acks and state reports were read back.
4. Audible checks (buds worn, owner listening): ANC muffles room noise, Transparency opens
   it up, Normal sits in between; switching mode plays the buds' confirmation tone.

## BLE GATT endpoints

Identical to the BP1 Pro ANC (WinRT GATT dump of a physical unit):

| Role | UUID |
|---|---|
| Service | `53527aa4-29f7-ae11-4e74-997334782568` |
| Write | `ee684b1a-1e9b-ed3e-ee55-f894667e92ac` (WRITE only) |
| Notify | `654b749c-e37f-ae1f-ebab-40ca133e3690` (NOTIFY + READ) |

The buds also expose Google Fast Pair (`0xFE2C`) and advertise a second, separate
peripheral named `BP1 Ultra` that carries the Fast Pair data but **not** the control
service. Matching by name alone can therefore pick the wrong peripheral; the client
matches on the service UUID.

The control service is only advertised while no other control session is open: while the
official app is connected over SPP, the buds drop it from their advertisement. The BLE
address is random and rotates, so it must not be cached.

### Telling the BP1 Pro and BP1 Ultra apart

Both advertise the same service UUID, so the scan cannot distinguish them. Right after the
notify subscription the BP1 Ultra pushes, unprompted:

```
78 9C 00 0E 02 01 05 02 64 00 64 01 2C 56   battery L 100 %, R 100 %
78 9C 00 0C 02 01 03 27 64 00 6D 60         case 100 %, not charging
78 9C 00 0B 02 01 02 30 00 4C CD            EQ preset 0
78 9C 00 0B 02 01 02 90 02 8D 34            (unknown)
```

The BP1 Pro sends bare `AA …` frames instead. The client reads the first notification
and picks the model from its leading magic (`WireFormat::detect`).

## Frame format

```
78 9C | LEN (u16 BE) | 02 | KIND | N | CMD PAYLOAD… (N bytes) | CRC (u16 BE)
```

| Field | Meaning |
|---|---|
| `78 9C` | Magic |
| `LEN` | Length of the whole frame, magic and CRC included |
| `02` | Constant |
| `KIND` | App → device: `01` read/query, `03` write. Device → app: always `01` |
| `N` | Length of `CMD PAYLOAD…` |
| `CRC` | CRC16/MODBUS (reflected poly `0xA001`, init `0xFFFF`) over every byte before it |

The same frames are used over BLE and SPP. The bare BP1 Pro frames (`BA 34 01 68`, …)
are silently ignored.

Responses:

- A write is acked with `<CMD> 01`. The device then reports the new state on the matching
  read opcode (e.g. `34 …` → `34 01`, then `33 …`).
- `<CMD> 0B` means the write was rejected (see low-latency mode below).
- Unknown opcodes get no answer at all.
- The device occasionally drops a write without acking it; re-reading the state and
  re-sending fixes it. The desktop client re-queries the state 1.5 s after every write.

## Opcodes used by the desktop app

### Battery

```
← 02 L 00 R 01                       left / right percent; 00 and 01 are bud markers
← 27 PCT CHARGING                    case percent, charging flag
→ 02                                 (read) request a battery report
```

| Frame | Meaning |
|---|---|
| `78 9C 00 0E 02 01 05 02 64 00 64 01 2C 56` | L 100 %, R 100 % |
| `78 9C 00 0E 02 01 05 02 5A 00 64 01 C4 5B` | L 90 %, R 100 % |
| `78 9C 00 0C 02 01 03 27 64 00 6D 60` | case 100 %, not charging |

### Noise control

```
→ 34 MODE PARAM                      (write) set mode
← 34 01                              flat ack, carries no state
← 33 MODE ANC_SELECTION TRANSPARENCY_SELECTION CUSTOM_LEVEL
→ 33                                 (read) query state
```

| `MODE` | `PARAM` written by the official app |
|---|---|
| `00` Normal | `FF` |
| `01` Noise cancellation | `01`–`05` custom strength; scenes `65` indoor, `66` outdoor, `67` commuting, `6C` traveling, `68` adaptive |
| `02` Transparency | `01` full, `02` voice mode |

The state frame remembers each selection independently of the active mode:

| State frame payload | Meaning |
|---|---|
| `33 00 05 01 05` | Normal (last ANC selection level 5, transparency full, custom level 5) |
| `33 01 05 01 05` | Noise cancellation, level 5 |
| `33 01 66 01 05` | Noise cancellation, "outdoor" scene, custom level still 5 |
| `33 01 01 01 01` | Noise cancellation, custom level 1 |
| `33 02 05 02 05` | Transparency, voice mode |

Example writes (byte-exact with the official app):

```
78 9C 00 0C 02 03 03 34 01 05 3B 03   ANC level 5
78 9C 00 0C 02 03 03 34 02 01 08 02   Transparency (full)
78 9C 00 0C 02 03 03 34 00 FF E8 82   Normal
```

Wind-noise reduction: `52 00/01` (write, acked `52 01`), state `51 ON`. Not exposed in the
desktop app.

### EQ

```
← 30 IDX                             active preset index (also pushed on connect)
→ 31 IDX F1 … F8                     (write) apply a curve, acked with 31 01
```

The device does not store preset curves: the official app streams the whole curve every
time. Each `F` is 8 bytes, four little-endian u16 values: centre frequency (Hz), gain, Q,
filter type. The desktop app sends the official app's curves byte for byte (see
`crates/baseus-protocol/src/models/bp1_ultra.rs`) for these four presets:

| Desktop preset | Official preset | `IDX` | Official write CRC |
|---|---|---|---|
| Balanced | Baseus Classic | `00` | `EE 3E` |
| Bass Boost | Powerful Bass | `01` | `B0 BC` |
| Voice | Podcast | `10` | `5D 1D` |
| Clear | Clear Treble | `09` | `B0 3E` |

The official app has 30 presets (`IDX` `00`–`1D`) and a custom 8-band EQ (`IDX` `65`,
bands 65 / 125 / 250 / 500 / 1k / 2k / 5k / 12k Hz, filter type 1, gain = 120 + 10 × dB).

### Low-latency (game) mode

```
→ 24 ON                              (write)
← 24 01                              ack, then 23 ON
← 24 0B                              rejected
→ 23                                 (read) query state
```

Accepted earlier in the same session (`24 01` → `23 01` / `23 00`), later rejected with
`24 0B` — **including by the official app** — while the phone was on LDAC and the buds were
connected to two devices. The client always re-reads `23` after a write, so the toggle
reflects what the device actually did.

## Other opcodes (captured, not used by the desktop app)

| Function | Write → | ← State |
|---|---|---|
| Spatial audio | `43 00/01` (ack `43 01`) | `42 ON 01` |
| Bass boost | `54 ON LEVEL` (ack `54 01`), level 1–3 | `53 ON LEVEL` |
| Volume limiter | `94 ON DB` (ack `94 01`), dB as decimal (`4B` = 75 … `64` = 100) | `93 ON DB` |
| Sleep mode | `79 ON SOUND` (ack `79 01`) | `78 ON SOUND` |
| Gestures | `8D SIDE GESTURE FUNCTION` (ack `8D 01`) | `8C SIDE FF` → `8C SIDE g f g f g f g f` |
| Ambient-sound cycle | `8F SIDE GESTURE 06 N m1 … mN` (ack `8F 01`) | `8E SIDE GESTURE 06` → same layout |
| Firmware version | — | `19` → `19 xx 10 1A 10 1A` (1.0.26 / 1.0.26) |
| Dual connection | — | `3B` → `3B ON CONNECTED`; `3E` → one frame per connected device |
| LDAC | not toggled (restarts the buds) | `74` → `74 ON` |
| State dump | `FF` (read) | `23`, `74`, … |

Gestures: SIDE `00` left / `01` right; GESTURE `00` double tap, `01` triple tap,
`02` tap and hold, `03` single tap; FUNCTION `00` none, `01` play/pause, `02` previous,
`03` next, `06` ambient-sound cycle, `0B` volume up, `0C` volume down, `12` remote camera.
Ambient-cycle modes: `01` Normal, `02` Transparency, `03` Noise cancellation.

`80` and `3D` (connection events) and `3E` carry Bluetooth addresses and device names;
they are not documented byte by byte here.

## Not captured

- **Find my earbuds** — the official app only offers it to logged-in accounts. The desktop
  app hides the button for this model.
- Answer without a known meaning: `36`, `37`, `3C`, `44`, `45`, `46` (echo), `4E`, `4F`,
  `5B`, `5C`, `5D`, `75`, `8B`, `90`, `96`, `97` (`97 0B`).
- No answer to a read: `25`, `35`, `41`, `50`, `55`–`58`, `5A`, `76`, `77`, `7A`, `7B`,
  `7C`, `91`, `92`, `95`.

## Raw captures

`docs/protocol/captures/bp1-ultra-*.txt` hold the decoded frames of the sessions above,
with Bluetooth addresses and device names removed. The raw `btsnoop_hci.log` files are
not committed: they carry the addresses of every device involved.
