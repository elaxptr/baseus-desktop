# Baseus Inspire XH1 — Packet Table (APK-DERIVED, UNVERIFIED)

> **Status:** Experimental — all opcodes and UUIDs extracted from static APK analysis.
> No physical device has been used to verify this table. Fields marked ⚠ need live confirmation.
> See "How to verify" at the bottom if you own an XH1.

## Device identification

| Field | Value |
|---|---|
| App product name | "Baseus Inspire XH1" (seen in APK bytecode-dump2.txt) |
| Chip vendor | Unknown — likely Qualcomm or Beken (⚠ unverified) |
| BT profiles | BLE GATT (control); Classic A2DP + HFP (audio) |
| App package | `com.baseus.intelligent` |
| Control protocol | Proprietary BLE GATT |
| BLE device name | `Baseus Inspire XH1` (from `EarFunctionManager2` dispatch key) |
| APK dispatch class | `com.control_center.intelligent.view.activity.headphones.manager.EarFunctionManager2` |

## BLE GATT endpoints ⚠ UNVERIFIED

Candidate UUIDs from `apk-analysis.log` classes6.dex (`0000ae0x` family).
classes6.dex also contains `00001101`/`0000110b`/`0000111e` Classic BT profiles, consistent
with a headphone that does BLE control + Classic audio.

| Role | UUID (candidate) | Source |
|---|---|---|
| Service | `0000ae00-0000-1000-8000-00805f9b34fb` | APK classes6.dex |
| Write | `0000ae01-0000-1000-8000-00805f9b34fb` | APK classes6.dex |
| Notify | `0000ae02-0000-1000-8000-00805f9b34fb` | APK classes6.dex |

Alternative UUID family if ae00 fails — classes4.dex (`0000fae0` family):

| Role | UUID (alt candidate) |
|---|---|
| Service | `0000fae0-0000-1000-8000-00805f9b34fb` |
| Write | `0000fae1-0000-1000-8000-00805f9b34fb` |
| Notify | `0000fae2-0000-1000-8000-00805f9b34fb` |

**To confirm:** Connect with nRF Connect, tap the XH1 device, list services. Report which UUID family is present.

## Frame format ⚠ assumed

Assumed same as BP1 Pro ANC (shared Baseus protocol base):

```
[ 0xAA ] [ CMD ] [ payload... ]   ← device → app (notify char)
[ 0xBA ] [ CMD ] [ payload... ]   ← app → device (write char)
```

No length or CRC in BLE GATT PDU; frame boundary = GATT notification/write boundary.

## Battery ⚠ UNVERIFIED

Assumed CMD `0x02` (same opcode as BP1). XH1 is over-ear, single unit — payload may differ.

Hypothesis A (single battery, 2-byte payload):
```
AA 02 [pct: u8] [charging_flag: u8]
```

Hypothesis B (L/R headband drivers, same format as earbuds):
```
AA 02 [left_driver_pct: u8] 0x00 [right_driver_pct: u8] 0x01
```

To confirm: pair XH1, open nRF Connect, subscribe to notify characteristic, charge/discharge while observing `0xAA 0x02` frames.

## APK feature opcode lists (NOT wire bytes)

These are app-internal feature-type integers from `EarFunctionManager2.f("Baseus Inspire XH1")`.
They identify which UI panels to show — they are **not** BLE wire opcodes.

```
f() returns: [0x65=101, 0x6B=107, 0x00, 0x3E3=995, 0x6A=106, 0x68=104, 0x0A=10]
i() returns: [0x3E2=994, 0x3E6=998, 0x07, 0x03, 0x6A=106, 0x68=104]
```

Source: `bytecode-dump2.txt` lines 4079–4100 (method `f`) and 4163–4179 (method `i`).

The values 0x68 (104) and 0x6A (106) appear in both lists and are small enough to be single-byte
BLE opcodes. **Candidates only — must be confirmed with hardware.**

## ANC modes ⚠ UNVERIFIED

XH1 has 5 adaptive noise modes (not BP1's 3-mode Off/ANC/Transparency).
Mode labels and internal IDs from `NoiseTypeBean` constructors in `EarFunctionManager2`
(`bytecode-dump2.txt` lines 5699–5751):

| Mode label | Internal ID | Wire byte (⚠ candidate) |
|---|---|---|
| Self (own hearing profile) | unknown | unknown |
| Indoor | `0x0A` (10) | `0x0A`? |
| Outdoor | `0x09` (9) | `0x09`? |
| Commute | `0x08` (8) | `0x08`? |
| Custom | variable (from `NoiseReduceDataModel.b()`) | unknown |

**Hypothesis:** The mode IDs from `NoiseTypeBean` (8/9/10) may be the wire payload bytes for
an ANC SET command with format `BA [anc_opcode] [mode_id]`. The ANC opcode candidate is `0x68`
(appears in both feature and query lists).

ANC RX (device → app notification): `AA 68 [mode_id]`? (⚠ all unverified).

To confirm: hook `OutputStream.write` via `docs/frida/socket-trace.js` while toggling
noise modes in the Baseus app.

## Sibling models (same Inspire family, untested)

| Model | Notes |
|---|---|
| Baseus Inspire XP1 | Appears in same dispatch branches; may share protocol |
| Baseus Inspire XC1 | Appears in same dispatch branches; unknown feature delta |

## How to verify (if you own an XH1)

1. Install the app, let it connect to your XH1.
2. Open nRF Connect → find `Baseus Inspire XH1` device.
3. List GATT services — identify the service, write, and notify UUIDs. Update this table.
4. Subscribe to notify characteristic. Toggle noise mode in the Baseus app and capture the hex bytes.
5. Use nRF Connect's write UI to send `AA 02` manually and observe the device response.
6. Run `docs/frida/socket-trace.js` via Frida for write-side captures.
7. Open a PR updating this table and `crates/baseus-protocol/src/models/inspire_xh1.rs`.

## Source files

- APK dump: `docs/protocol/captures/bytecode-dump.txt`, `bytecode-dump2.txt`
- UUID inventory: `docs/protocol/captures/apk-analysis.log`
- Extraction script: `tools/extract_apk_model.py --name "Baseus Inspire XH1"`
