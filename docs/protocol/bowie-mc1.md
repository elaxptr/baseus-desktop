# Baseus Bowie MC1 — Packet Table

**Status: decoded from one capture, not verified over BLE, not supported by the app.**

## Device identification

| Field | Value |
|---|---|
| Classic BT name | `Baseus Bowie MC1` |
| Firmware | 2.1.21 (left and right) |
| Official app | `com.baseus.intelligent` 2.18.0 |
| BT profiles seen | Classic SPP (control, RFCOMM DLCI 2), A2DP, AVRCP, HFP |
| BLE control service | **Unknown.** The capture has no LE traffic to the buds. |
| Control protocol | The BP1 Pro ANC's plain `BA`/`AA` frames ([bp1-pro-anc.md](bp1-pro-anc.md)) |

## Source

HCI snoop log from the owner's phone, attached to
[#12](https://github.com/elaxptr/baseus-desktop/issues/12) with the list of actions
taken. The control payloads are extracted in
[captures/bowie-mc1-app-session.txt](captures/bowie-mc1-app-session.txt). The official app
talked to the buds over SPP only, so whether the buds expose the BP1 Pro's GATT service
(`53527aa4-…`) over BLE, which is all this app can use, is not known yet.

## Frames

Same envelope as the BP1 Pro ANC: `BA <cmd> <payload>` app → buds, `AA <cmd> <payload>`
buds → app, no length or CRC.

### Identical to the BP1 Pro ANC

| Frame | Meaning |
|---|---|
| `BA 02` → `AA 02 <L%> 00 <R%> 01` | Battery query / report (`AA 02 64 00 64 01`, later `AA 02 5A 00 64 01`) |
| `AA 27 <case%> <charging>` | Case battery (`AA 27 64 00`) |
| `BA 24 01` / `BA 24 00` → `AA 24 01`, then `AA 23 01` / `AA 23 00` | Low-latency on / off: flat ack, then state |
| `BA 10 00 01` / `BA 10 00 00` → `AA 10 01` | Find left: start / stop |
| `BA 10 01 01` / `BA 10 01 00` → `AA 10 01` | Find right: start / stop |
| `BA 42` → `AA 42 <preset>` | Current EQ preset |
| `BA 30` → `AA 30 00` | Noise-control query. Always `00` (Off); the MC1 has no ANC. |

### New on the MC1

**Status dump.** `BA 01` makes the buds send their whole state in one burst:

```
AA 42 00               EQ preset 0
AA 21 00 0C 0B         gesture slot 0: left 0C, right 0B
AA 02 64 00 64 01      battery
AA 23 00               low latency off
AA 27 64 00            case battery
AA 53 01               bass boost on
AA 30 00               noise control off
AA 19 01 21 15 21 15   firmware: L 0x21 0x15 = 2.1.21, R 2.1.21
AA 36 01               unknown
```

**Bass boost** (on/off only), same flat-ack-then-state pattern as low latency:

| Action | Sent | Replies |
|---|---|---|
| Off | `BA 54 00` | `AA 54 01`, `AA 53 00` |
| On | `BA 54 01` | `AA 54 01`, `AA 53 01` |

**EQ presets.** Unlike the BP1 Pro (`BA 43 <preset>`), the app sends the preset id followed
by a filter table, and the buds ack with a flat `AA 31 01`:

```
BA 31 <preset> 32 00 78 00 07 00 | 04 00 94 11 32 00 0A 00 | 01 00 E0 2E F0 00 07 00 | …
```

The table looks like 8-byte filter entries (type, frequency in Hz, gain, Q, all u16 LE:
`04 00 | 94 11 = 4500 Hz | …`), with a shared head and per-preset tail. The app would replay
each table verbatim, so its exact meaning doesn't matter. Full frames are in the capture
file. Preset ids, in the order the owner selected them:

| Preset | Id | Confidence |
|---|---|---|
| Baseus Classic | `00` | Reported by `AA 42 00` at start and re-selected last |
| Hi-Fi Live | `03` | Inferred from the order of the action list |
| Jazz | `07` | Inferred |
| Treble Boost | `09` | Inferred |
| Acoustic | `0A` | Inferred |
| Rock Classic | `0B` | Inferred |

The gaps suggest one id space shared with larger preset lists (the EP10 Pro lists eight,
see [#5](https://github.com/elaxptr/baseus-desktop/issues/5)).

**Gestures.** `BA 21 <slot>` queries a slot, answered by `AA 21 <slot> <left> <right>`.
`BA 22 <slot> <left> <right>` sets it (`FF` = leave that bud unchanged), acked with
`AA 22 01`. Slots `00`–`03` are presumably the press counts / long press; values seen are
`00`–`04`, `0B`, `0C`. Initial state: slot 0 `0C/0B`, slot 1 `04/04`, slot 2 `02/03`,
slot 3 `01/01`. Which slot and value is which action needs the app's labels; the app has
no gesture UI, so this is left undecoded.

### Unknown

- `BA 3B`: sent by the app after every unsolicited battery/case/noise-control burst. No
  reply. Probably an acknowledgement; not needed to drive the buds.
- `AA 36 01`: part of the status dump.

## Before the app can support it

1. Confirm the buds expose service `53527aa4-29f7-ae11-4e74-997334782568` over BLE, and
   under what LE name (nRF Connect on the phone, or a scan from Windows). Without it the
   app would need an SPP transport.
2. Confirm the EQ preset order.
3. The owner tests a build: battery, bass boost, EQ, low latency, find.
