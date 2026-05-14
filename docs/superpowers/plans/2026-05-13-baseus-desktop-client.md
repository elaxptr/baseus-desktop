# Baseus Desktop Client — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an open-source Windows desktop app (Rust + Tauri + SolidJS) that displays live L/R/case battery for the Baseus Bass BP1 Pro ANC earbuds, derived from reverse-engineering the Android app's Bluetooth protocol.

**Architecture:** Rust workspace with three crates: `baseus-protocol` (pure codec), `baseus-transport` (Windows WinRT Bluetooth I/O), and `apps/baseus-app` (Tauri shell). SolidJS+Tailwind frontend communicates exclusively via Tauri commands/events. Phase 0 (RE work) must complete before the protocol crate gets golden tests; all other scaffolding can proceed in parallel.

**Tech Stack:** Rust (stable 1.75+), Tauri v2, SolidJS, Tailwind CSS, Vite, `windows-rs` (WinRT Bluetooth APIs), `tokio`, `thiserror`, `tracing`, `pnpm`.

---

## File Structure

```
baseus_rebuild/
├── Cargo.toml                             # workspace manifest
├── rust-toolchain.toml                    # pin stable
├── LICENSE                                # MIT
├── README.md
├── BACKLOG.md
├── crates/
│   ├── baseus-protocol/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                     # re-exports
│   │       ├── framing.rs                 # Frame struct, encode/decode, CRC-8
│   │       ├── types.rs                   # BatteryState, AncMode, BaseusModel, DeviceEvent
│   │       └── models/
│   │           ├── mod.rs                 # dispatch by BaseusModel variant
│   │           └── bp1_pro_anc.rs         # per-model decode/encode impl
│   └── baseus-transport/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs                     # BluetoothTransport trait, TransportError, MockTransport
│           └── win/
│               ├── mod.rs
│               └── rfcomm.rs              # WinRT RfcommDeviceService wrapper
├── apps/
│   └── baseus-app/
│       ├── src-tauri/                     # Tauri 2 convention: Rust lives here
│       │   ├── Cargo.toml
│       │   ├── tauri.conf.json
│       │   ├── capabilities/
│       │   │   └── default.json
│       │   └── src/
│       │       ├── main.rs                # entry point (calls lib::run())
│       │       ├── lib.rs                 # tauri::Builder, plugin wiring, AppState
│       │       ├── commands.rs            # #[tauri::command] fns
│       │       ├── device.rs              # Device<T> state machine + event loop
│       │       └── tray.rs                # tray icon + menu build
│       ├── src/                           # SolidJS frontend
│       │   ├── index.tsx
│       │   ├── App.tsx
│       │   ├── components/
│       │   │   ├── BatteryCard.tsx
│       │   │   └── ConnectionCard.tsx
│       │   └── lib/
│       │       └── tauri.ts               # typed invoke/listen wrappers
│       ├── index.html
│       ├── package.json
│       ├── tsconfig.json
│       ├── vite.config.ts
│       └── tailwind.config.ts
└── docs/
    ├── re-methodology.md                  # reproduce setup: MuMu + Frida + adb
    ├── frida/
    │   └── socket-trace.js                # main Frida hook script
    └── protocol/
        ├── framing.md                     # general Baseus framing spec
        ├── bp1-pro-anc.md                 # per-model packet table (filled in Phase 0)
        └── captures/                      # sanitised btsnoop + Frida traces
            └── .gitkeep
```

---

## Phase 0 — Reverse Engineering (must complete before Task 6)

> These tasks are research, not code. TDD doesn't apply; there's nothing to test until the protocol is known. Complete these before writing the protocol golden tests in Task 6.

### Task 1: Prepare Frida hook script + enable HCI logging

**Files:**
- Create: `docs/frida/socket-trace.js`
- Create: `docs/re-methodology.md`

- [ ] **Step 1: Enable Bluetooth HCI snooping on the MuMuPlayer emulator**

  In MuMuPlayer's Android settings: **Developer Options → Enable Bluetooth HCI snoop log**.
  The log will appear at `/sdcard/btsnoop_hci.log` (or `/data/misc/bluetooth/logs/btsnoop_hci.log` on some images — check both paths with `adb shell ls /sdcard/btsnoop_hci.log /data/misc/bluetooth/logs/btsnoop_hci.log`).

  Verify:
  ```
  adb devices
  # → emulator-5554  device   (or similar)
  adb shell getprop persist.bluetooth.btsnoopenable
  # → true  (if HCI logging is on)
  ```

- [ ] **Step 2: Write the Frida socket-trace hook script**

  Create `docs/frida/socket-trace.js`:
  ```javascript
  // Hooks Android BluetoothSocket I/O to log every byte in hex + timestamp.
  // Works on both Classic BT (RFCOMM) and BLE GATT via the Java I/O layer.
  Java.perform(function () {
    var OutputStream = Java.use('java.io.OutputStream');
    var InputStream  = Java.use('java.io.InputStream');

    function toHex(buf, offset, len) {
      var out = [];
      for (var i = 0; i < len; i++) {
        var b = buf[offset + i] & 0xff;
        out.push((b < 16 ? '0' : '') + b.toString(16));
      }
      return out.join(' ');
    }

    // Log outgoing bytes (app → earbuds)
    OutputStream.write.overload('[B', 'int', 'int').implementation = function (buf, off, len) {
      console.log('[TX ' + new Date().toISOString() + '] ' + toHex(buf, off, len));
      return this.write(buf, off, len);
    };

    // Log incoming bytes (earbuds → app)
    InputStream.read.overload('[B', 'int', 'int').implementation = function (buf, off, len) {
      var n = this.read(buf, off, len);
      if (n > 0) console.log('[RX ' + new Date().toISOString() + '] ' + toHex(buf, off, n));
      return n;
    };

    console.log('[socket-trace] hooks installed — waiting for Bluetooth I/O...');
  });
  ```

- [ ] **Step 3: Write the RE methodology doc**

  Create `docs/re-methodology.md` with the following content:
  ```markdown
  # Reverse-Engineering Methodology

  ## Prerequisites
  - MuMuPlayer (or any Android emulator with Bluetooth passthrough)
  - Frida server (arm64 build matching emulator ABI) pushed to `/data/local/tmp/frida-server`
  - ADB connected: `adb devices` shows the emulator
  - Baseus app installed on emulator; BP1 Pro ANC paired to host's Bluetooth adapter

  ## Steps

  ### 1. Start Frida server
  ```
  adb shell "chmod +x /data/local/tmp/frida-server && /data/local/tmp/frida-server &"
  ```

  ### 2. Identify Baseus app package name
  ```
  adb shell pm list packages | grep baseus
  # → com.baseus.headset  (or similar)
  ```

  ### 3. Attach Frida hook
  ```
  frida -U -f com.baseus.headset -l docs/frida/socket-trace.js --no-pause 2>&1 | tee docs/protocol/captures/session-$(date +%Y%m%d-%H%M%S).log
  ```

  ### 4. Execute stimulus sequence (with timestamps noted in the log)
  - Open app cold
  - Open earbuds case lid
  - Toggle ANC: off → on → transparency → off
  - Remove one earbud from ear
  - Close case

  ### 5. Pull btsnoop HCI log
  ```
  adb pull /sdcard/btsnoop_hci.log docs/protocol/captures/btsnoop-$(date +%Y%m%d).log
  ```
  Open in Wireshark. Filter: `btrfcomm || avctp || a2dp`

  ## What to look for
  - Which Bluetooth profile is used? Check btsnoop for RFCOMM vs ATT/GATT traffic.
  - What is the service UUID? Visible in the SDP query or GATT service discovery.
  - What is the framing format? Look for repeating byte patterns at packet boundaries.
  - Sanitise captures before committing: `sed -i 's/[0-9A-F]\{2\}:[0-9A-F]\{2\}:[0-9A-F]\{2\}:[0-9A-F]\{2\}:[0-9A-F]\{2\}:[0-9A-F]\{2\}/XX:XX:XX:XX:XX:XX/g'`
  ```

- [ ] **Step 4: Commit**

  ```
  git add docs/frida/socket-trace.js docs/re-methodology.md
  git commit -m "docs: add Frida socket-trace hook + RE methodology"
  ```

---

### Task 2: Run capture session

**Files:**
- Create: `docs/protocol/captures/<session>.log` (Frida output)
- Create: `docs/protocol/captures/btsnoop-<date>.log`

- [ ] **Step 1: Start Frida server on emulator**
  ```
  adb shell "chmod +x /data/local/tmp/frida-server && /data/local/tmp/frida-server &"
  ```
  Wait 2 seconds, then verify:
  ```
  frida-ps -U | head -5
  ```

- [ ] **Step 2: Attach hook, pipe to file**
  ```
  frida -U -f com.baseus.headset -l docs/frida/socket-trace.js --no-pause 2>&1 | tee docs/protocol/captures/session-01.log
  ```
  Leave this running. Keep the terminal open.

- [ ] **Step 3: Execute stimulus in the Baseus app** (describe each action in comments in the log by pressing Enter to add a newline)

  In order, with ~3 seconds between each action:
  1. Open app cold → observe handshake bytes in log
  2. Open earbuds case lid → expect battery notification
  3. Toggle ANC: off → on → transparency → off
  4. Remove one bud from ear
  5. Close case, let disconnect happen

- [ ] **Step 4: Pull btsnoop log**
  ```
  adb pull /sdcard/btsnoop_hci.log docs/protocol/captures/btsnoop-session-01.log
  ```

- [ ] **Step 5: Sanitise MAC addresses in both log files**
  ```
  # In PowerShell:
  (Get-Content docs\protocol\captures\session-01.log) -replace '[0-9a-fA-F]{2}(:[0-9a-fA-F]{2}){5}', 'XX:XX:XX:XX:XX:XX' | Set-Content docs\protocol\captures\session-01.log
  (Get-Content docs\protocol\captures\btsnoop-session-01.log) -replace '[0-9a-fA-F]{2}(:[0-9a-fA-F]{2}){5}', 'XX:XX:XX:XX:XX:XX' | Set-Content docs\protocol\captures\btsnoop-session-01.log
  ```

- [ ] **Step 6: Commit captures**
  ```
  git add docs/protocol/captures/
  git commit -m "docs: add Phase 0 capture session 01"
  ```

---

### Task 3: Analyse captures and write protocol documentation

**Files:**
- Create: `docs/protocol/framing.md`
- Create: `docs/protocol/bp1-pro-anc.md`

- [ ] **Step 1: Open Frida log and identify packet boundaries**

  In `session-01.log`, look for `[TX ...]` and `[RX ...]` lines.
  Each line is one `write()` or `read()` call — one logical packet per line (usually; if not, look for length-prefix framing where the first 1-2 bytes declare the packet size).

- [ ] **Step 2: Open btsnoop in Wireshark, filter to app traffic**

  Filter: `btrfcomm.frame_type == 0x03` (UIH frames carry application data over RFCOMM). This confirms Classic BT RFCOMM vs BLE ATT. Note the RFCOMM channel number — needed for the transport implementation.

- [ ] **Step 3: Identify magic bytes and framing format**

  Compare the first 2-4 bytes across all packets. Most TWS protocols use one of:
  - `AA BB <len> <cmd> <payload> <crc8>` (most common)
  - `5A A5 <len_hi> <len_lo> <cmd> <payload> <crc16>` (16-bit length variant)

  Note what you find and write to `docs/protocol/framing.md`.

  Template for `docs/protocol/framing.md`:
  ```markdown
  # Baseus Protocol Framing

  ## Transport
  - **Protocol**: [RFCOMM / BLE GATT — fill in from btsnoop]
  - **RFCOMM channel**: [fill in from SDP exchange in btsnoop]
  - **Service UUID**: [fill in from SDP / GATT service discovery]

  ## Frame format

  | Offset | Size | Field   | Description                          |
  |--------|------|---------|--------------------------------------|
  | 0      | 1    | magic0  | Always `0x??` — fill in              |
  | 1      | 1    | magic1  | Always `0x??` — fill in              |
  | 2      | 1    | len     | Payload length in bytes              |
  | 3      | 1    | cmd     | Command / opcode byte                |
  | 4      | len  | payload | Variable-length payload              |
  | 4+len  | 1    | crc     | CRC-8 or XOR checksum over bytes 0..(4+len-1) |

  ## Checksum algorithm
  [Fill in: CRC-8 (poly 0x07), XOR of all bytes, or other]
  ```

- [ ] **Step 4: Map stimulus → packet**

  For each stimulus action (open case, battery, ANC toggle), find the corresponding `[RX ...]` line in the Frida log. Extract the opcode byte and payload, and write to `docs/protocol/bp1-pro-anc.md`.

  Template for `docs/protocol/bp1-pro-anc.md`:
  ```markdown
  # Baseus Bass BP1 Pro ANC — Packet Reference

  Derived from capture session 01. See `docs/protocol/captures/session-01.log`.

  ## Battery notification (device → host)

  **Opcode**: `0x??` (fill in)

  | Payload byte | Field         | Notes                    |
  |--------------|---------------|--------------------------|
  | 0            | left_pct      | 0–100                    |
  | 1            | right_pct     | 0–100                    |
  | 2            | case_pct      | 0–100                    |
  | 3            | status_flags  | bit0=left_charging, bit1=right_charging, bit2=case_charging |

  **Example capture**: `AA 03 04 XX 64 5A 78 00 YY`

  ## Handshake

  **Opcode**: `0x??` (fill in)

  [describe handshake sequence — typically app sends a probe, device responds with model ID + firmware version]

  ## ANC mode command (host → device) — v1.1

  **Opcode**: `0x??` (fill in)

  | Value | Mode          |
  |-------|---------------|
  | 0x00  | ANC off       |
  | 0x01  | ANC on        |
  | 0x02  | Transparency  |
  ```

  **Extract actual bytes from your captures and fill in all `0x??` and `fill in` fields.**

- [ ] **Step 5: Extract 3 raw binary captures for golden tests**

  From the Frida log, copy the hex for:
  - One battery notification packet
  - One handshake response packet
  - One ANC response packet (even if not v1, useful for future)

  Write them as binary files:
  ```powershell
  # Example for a battery packet "aa 03 04 01 64 5a 78 00 cb":
  $bytes = [byte[]](0xaa, 0x03, 0x04, 0x01, 0x64, 0x5a, 0x78, 0x00, 0xcb)
  [System.IO.File]::WriteAllBytes("docs\protocol\captures\battery.bin", $bytes)

  # handshake response:
  $bytes = [byte[]]( ... fill in ... )
  [System.IO.File]::WriteAllBytes("docs\protocol\captures\handshake-resp.bin", $bytes)
  ```
  Replace the example bytes with your actual capture values.

- [ ] **Step 6: Commit protocol docs**
  ```
  git add docs/protocol/framing.md docs/protocol/bp1-pro-anc.md docs/protocol/captures/
  git commit -m "docs: add Phase 0 protocol analysis — framing + BP1 Pro ANC packet table"
  ```

---

## Phase 1 — Workspace & Protocol Crate

### Task 4: Initialize git repo + Rust workspace

**Files:**
- Create: `Cargo.toml`
- Create: `rust-toolchain.toml`
- Create: `LICENSE`
- Create: `README.md`
- Create: `BACKLOG.md`

- [ ] **Step 1: Initialize git repository**
  ```
  cd C:\Users\elaxptr\Documents\work\personal\baseus_rebuild
  git init
  git branch -M main
  ```

- [ ] **Step 2: Create workspace `Cargo.toml`**

  Create `Cargo.toml`:
  ```toml
  [workspace]
  members = [
      "crates/baseus-protocol",
      "crates/baseus-transport",
      "apps/baseus-app/src-tauri",
  ]
  resolver = "2"

  [workspace.dependencies]
  tokio       = { version = "1", features = ["full"] }
  thiserror   = "2"
  tracing     = "0.1"
  serde       = { version = "1", features = ["derive"] }
  serde_json  = "1"
  ```

- [ ] **Step 3: Pin stable Rust toolchain**

  Create `rust-toolchain.toml`:
  ```toml
  [toolchain]
  channel = "stable"
  ```

- [ ] **Step 4: Create MIT LICENSE**

  Create `LICENSE`:
  ```
  MIT License

  Copyright (c) 2026 baseus_rebuild contributors

  Permission is hereby granted, free of charge, to any person obtaining a copy
  of this software and associated documentation files (the "Software"), to deal
  in the Software without restriction, including without limitation the rights
  to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
  copies of the Software, and to permit persons to whom the Software is
  furnished to do so, subject to the following conditions:

  The above copyright notice and this permission notice shall be included in all
  copies or substantial portions of the Software.

  THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
  IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
  FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
  AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
  LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
  OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
  SOFTWARE.
  ```

- [ ] **Step 5: Create README.md**

  Create `README.md`:
  ```markdown
  # baseus-desktop

  Open-source Windows desktop client for Baseus earbuds.
  Displays live battery (L / R / case) from your Baseus Bass BP1 Pro ANC — and eventually other models.

  Built by reverse-engineering the official Baseus Android app. Protocol documentation and
  Frida capture scripts live in [`docs/protocol/`](docs/protocol/).

  ## Status

  🚧 **v0 — in development.** Not yet functional.

  ## Requirements

  - Windows 10 1903+ (WinRT Bluetooth APIs)
  - Baseus Bass BP1 Pro ANC earbuds (v1 only; architecture supports adding models)

  ## Building

  ```
  cargo build
  cd apps/baseus-app && pnpm install && pnpm tauri build
  ```

  ## Contributing

  See [docs/re-methodology.md](docs/re-methodology.md) to add support for your Baseus model.
  ```

- [ ] **Step 6: Create BACKLOG.md**

  Create `BACKLOG.md`:
  ```markdown
  # Backlog

  These are post-v1 features, intentionally deferred.

  - [ ] ANC mode switching (off / ANC / transparency)
  - [ ] EQ presets + custom EQ
  - [ ] Touch gesture remapping
  - [ ] Find-my-buds (play beep)
  - [ ] Firmware version display
  - [ ] Multi-device fleet support
  - [ ] Linux support (BlueZ transport impl)
  - [ ] macOS support (IOBluetooth transport impl)
  - [ ] Auto-start on Windows login
  - [ ] Windows notifications on low battery
  ```

- [ ] **Step 7: Create .gitignore**

  Create `.gitignore`:
  ```
  /target/
  node_modules/
  dist/
  .DS_Store
  Thumbs.db
  *.local
  ```

- [ ] **Step 8: First commit**
  ```
  git add Cargo.toml rust-toolchain.toml LICENSE README.md BACKLOG.md .gitignore
  git commit -m "chore: initialize workspace"
  ```

---

### Task 5: Scaffold `baseus-protocol` crate — types and skeleton

**Files:**
- Create: `crates/baseus-protocol/Cargo.toml`
- Create: `crates/baseus-protocol/src/lib.rs`
- Create: `crates/baseus-protocol/src/types.rs`
- Create: `crates/baseus-protocol/src/framing.rs`
- Create: `crates/baseus-protocol/src/models/mod.rs`
- Create: `crates/baseus-protocol/src/models/bp1_pro_anc.rs`

- [ ] **Step 1: Create crate directory structure**
  ```
  New-Item -ItemType Directory -Force crates\baseus-protocol\src\models
  ```

- [ ] **Step 2: Create `crates/baseus-protocol/Cargo.toml`**
  ```toml
  [package]
  name    = "baseus-protocol"
  version = "0.1.0"
  edition = "2021"

  [dependencies]
  thiserror = { workspace = true }
  serde     = { workspace = true }

  [dev-dependencies]
  # no extra deps — golden tests use include_bytes! from captured files
  ```

- [ ] **Step 3: Create `crates/baseus-protocol/src/types.rs`**
  ```rust
  use serde::{Deserialize, Serialize};

  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
  pub struct BatteryState {
      pub left_pct:        u8,
      pub right_pct:       u8,
      pub case_pct:        u8,
      pub left_charging:   bool,
      pub right_charging:  bool,
      pub case_charging:   bool,
  }

  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
  pub enum AncMode {
      Off,
      Anc,
      Transparency,
  }

  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
  pub enum DeviceEvent {
      BatteryUpdate(BatteryState),
      AncModeUpdate(AncMode),
      Connected,
      Disconnected,
  }

  #[derive(Debug, Clone, Copy, PartialEq)]
  pub enum BaseusModel {
      Bp1ProAnc,
  }
  ```

- [ ] **Step 4: Create `crates/baseus-protocol/src/framing.rs` — stub**

  This is a stub; the codec is filled in Task 6 once the framing format is confirmed from Phase 0.
  ```rust
  use thiserror::Error;

  #[derive(Debug, Error)]
  pub enum FrameError {
      #[error("buffer too short: need at least {need} bytes, got {got}")]
      TooShort { need: usize, got: usize },
      #[error("bad magic bytes: expected {expected:#04x} {expected2:#04x}, got {got:#04x} {got2:#04x}")]
      BadMagic { expected: u8, expected2: u8, got: u8, got2: u8 },
      #[error("checksum mismatch: computed {computed:#04x}, packet has {packet:#04x}")]
      ChecksumMismatch { computed: u8, packet: u8 },
  }

  /// Raw Baseus protocol frame: [magic0, magic1, len, cmd, payload..., checksum]
  /// Magic bytes and checksum algorithm are confirmed in Phase 0 and filled in below.
  #[derive(Debug, Clone, PartialEq)]
  pub struct Frame {
      pub cmd:     u8,
      pub payload: Vec<u8>,
  }

  impl Frame {
      /// Encode this frame to bytes. Fill in magic + checksum once Phase 0 is done.
      pub fn encode(&self) -> Vec<u8> {
          todo!("fill in after Phase 0 — see docs/protocol/framing.md")
      }

      /// Decode a frame from a raw byte slice.
      pub fn decode(buf: &[u8]) -> Result<Self, FrameError> {
          todo!("fill in after Phase 0 — see docs/protocol/framing.md")
      }
  }
  ```

- [ ] **Step 5: Create `crates/baseus-protocol/src/models/mod.rs`**
  ```rust
  use crate::{types::DeviceEvent, Frame};
  use thiserror::Error;

  #[derive(Debug, Error)]
  pub enum DecodeError {
      #[error("unknown opcode {0:#04x}")]
      UnknownOpcode(u8),
      #[error("payload too short for opcode {opcode:#04x}: need {need}, got {got}")]
      PayloadTooShort { opcode: u8, need: usize, got: usize },
  }

  pub mod bp1_pro_anc;

  pub use bp1_pro_anc::Bp1ProAnc;
  ```

- [ ] **Step 6: Create `crates/baseus-protocol/src/models/bp1_pro_anc.rs` — stub**
  ```rust
  use crate::{models::DecodeError, types::DeviceEvent, Frame};

  pub struct Bp1ProAnc;

  impl Bp1ProAnc {
      /// Decode a raw frame into a DeviceEvent. Fill in opcodes from Phase 0 packet table.
      pub fn decode_frame(frame: &Frame) -> Result<DeviceEvent, DecodeError> {
          todo!("fill in opcodes from docs/protocol/bp1-pro-anc.md after Phase 0")
      }
  }
  ```

- [ ] **Step 7: Create `crates/baseus-protocol/src/lib.rs`**
  ```rust
  pub mod framing;
  pub mod models;
  pub mod types;

  pub use framing::Frame;
  pub use types::{AncMode, BaseusModel, BatteryState, DeviceEvent};
  ```

- [ ] **Step 8: Verify the workspace compiles (stubs are ok)**
  ```
  cargo check -p baseus-protocol
  ```
  Expected: warnings about `todo!()` stubs but no errors.

- [ ] **Step 9: Commit**
  ```
  git add crates/baseus-protocol/
  git commit -m "feat(protocol): scaffold types + framing stub (Phase 0 pending)"
  ```

---

### Task 6: Implement `baseus-protocol` framing codec (TDD)

> **Prerequisite: Phase 0 Tasks 1–3 must be complete.** You need the actual magic bytes, framing format, and checksum algorithm from `docs/protocol/framing.md` to implement this task.

**Files:**
- Modify: `crates/baseus-protocol/src/framing.rs`

The instructions below use `0xAA 0x03` as the example magic bytes and CRC-8 (poly=0x07, init=0x00) as the example checksum. **Replace these with the actual values from your Phase 0 docs** before writing the tests.

- [ ] **Step 1: Write the failing tests first**

  Add a `#[cfg(test)]` module at the bottom of `crates/baseus-protocol/src/framing.rs`:
  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;

      // Synthetic test vector: cmd=0x01, payload=[0x64, 0x5a, 0x78, 0x00]
      // Replace magic bytes (0xaa, 0x03) and checksum (last byte) with real values from framing.md.
      const SYNTH_FRAME: &[u8] = &[0xaa, 0x03, 0x04, 0x01, 0x64, 0x5a, 0x78, 0x00, 0xcb];
      //                                  ^^^^ magic  ^^^^ len  ^^^^ cmd  ^^^^^^^^^^^^^^^^^^ payload  ^^^^ crc

      #[test]
      fn decode_synth_frame_gives_correct_cmd_and_payload() {
          let f = Frame::decode(SYNTH_FRAME).expect("should decode");
          assert_eq!(f.cmd, 0x01);
          assert_eq!(f.payload, &[0x64, 0x5a, 0x78, 0x00]);
      }

      #[test]
      fn encode_then_decode_round_trips() {
          let original = Frame { cmd: 0x01, payload: vec![0x64, 0x5a, 0x78, 0x00] };
          let bytes = original.encode();
          let decoded = Frame::decode(&bytes).expect("should decode");
          assert_eq!(decoded, original);
      }

      #[test]
      fn decode_rejects_bad_magic() {
          let mut bad = SYNTH_FRAME.to_vec();
          bad[0] = 0x00;
          assert!(matches!(Frame::decode(&bad), Err(FrameError::BadMagic { .. })));
      }

      #[test]
      fn decode_rejects_checksum_mismatch() {
          let mut bad = SYNTH_FRAME.to_vec();
          *bad.last_mut().unwrap() ^= 0xff;
          assert!(matches!(Frame::decode(&bad), Err(FrameError::ChecksumMismatch { .. })));
      }

      #[test]
      fn decode_rejects_buffer_too_short() {
          assert!(matches!(Frame::decode(&[0xaa]), Err(FrameError::TooShort { .. })));
      }
  }
  ```

- [ ] **Step 2: Run tests — confirm they all fail**
  ```
  cargo test -p baseus-protocol -- framing::tests 2>&1
  ```
  Expected: all 5 tests FAIL with `not yet implemented` panics.

- [ ] **Step 3: Implement CRC-8 (poly=0x07)**

  Add to the top of `crates/baseus-protocol/src/framing.rs`:
  ```rust
  // CRC-8, polynomial 0x07, init 0x00. Replace with XOR-sum if Phase 0 shows a simpler checksum.
  pub(crate) fn crc8(data: &[u8]) -> u8 {
      let mut crc: u8 = 0x00;
      for &byte in data {
          crc ^= byte;
          for _ in 0..8 {
              if crc & 0x80 != 0 {
                  crc = (crc << 1) ^ 0x07;
              } else {
                  crc <<= 1;
              }
          }
      }
      crc
  }
  ```
  If Phase 0 reveals the checksum is a simple XOR of all bytes, replace the function body with:
  ```rust
  pub(crate) fn crc8(data: &[u8]) -> u8 { data.iter().fold(0u8, |acc, &b| acc ^ b) }
  ```

- [ ] **Step 4: Implement `Frame::encode`**

  Replace the `todo!()` in `encode`:
  ```rust
  pub fn encode(&self) -> Vec<u8> {
      // Replace 0xAA, 0x03 with actual magic bytes from docs/protocol/framing.md
      const MAGIC: [u8; 2] = [0xAA, 0x03];
      let len = self.payload.len() as u8;
      let mut out = Vec::with_capacity(5 + self.payload.len());
      out.extend_from_slice(&MAGIC);
      out.push(len);
      out.push(self.cmd);
      out.extend_from_slice(&self.payload);
      let chk = crc8(&out);
      out.push(chk);
      out
  }
  ```

- [ ] **Step 5: Implement `Frame::decode`**

  Replace the `todo!()` in `decode`:
  ```rust
  pub fn decode(buf: &[u8]) -> Result<Self, FrameError> {
      const MAGIC: [u8; 2] = [0xAA, 0x03];
      const MIN_LEN: usize = 5; // magic(2) + len(1) + cmd(1) + crc(1)
      if buf.len() < MIN_LEN {
          return Err(FrameError::TooShort { need: MIN_LEN, got: buf.len() });
      }
      if buf[0] != MAGIC[0] || buf[1] != MAGIC[1] {
          return Err(FrameError::BadMagic {
              expected: MAGIC[0], expected2: MAGIC[1],
              got: buf[0], got2: buf[1],
          });
      }
      let payload_len = buf[2] as usize;
      let total = MIN_LEN + payload_len;
      if buf.len() < total {
          return Err(FrameError::TooShort { need: total, got: buf.len() });
      }
      let cmd = buf[3];
      let payload = buf[4..4 + payload_len].to_vec();
      let computed = crc8(&buf[..total - 1]);
      let packet   = buf[total - 1];
      if computed != packet {
          return Err(FrameError::ChecksumMismatch { computed, packet });
      }
      Ok(Self { cmd, payload })
  }
  ```

- [ ] **Step 6: Run tests — all should pass**
  ```
  cargo test -p baseus-protocol -- framing::tests 2>&1
  ```
  Expected: `test result: ok. 5 passed; 0 failed`

- [ ] **Step 7: Commit**
  ```
  git add crates/baseus-protocol/src/framing.rs
  git commit -m "feat(protocol): implement framing codec + CRC-8 with tests"
  ```

---

### Task 7: Implement BP1 Pro ANC model decode (TDD with real captures)

> **Prerequisite: Phase 0 Task 3 must be complete** — you need `docs/protocol/captures/battery.bin` and `docs/protocol/captures/handshake-resp.bin`.

**Files:**
- Modify: `crates/baseus-protocol/src/models/bp1_pro_anc.rs`

- [ ] **Step 1: Write failing golden tests**

  Create `crates/baseus-protocol/tests/golden.rs`:
  ```rust
  use baseus_protocol::{
      framing::Frame,
      models::Bp1ProAnc,
      types::{BatteryState, DeviceEvent},
  };

  /// Raw battery notification packet captured from the real device.
  /// If this file doesn't exist yet, complete Phase 0 Task 3 first.
  const BATTERY_PACKET: &[u8] = include_bytes!("../../../docs/protocol/captures/battery.bin");
  const HANDSHAKE_RESP: &[u8] = include_bytes!("../../../docs/protocol/captures/handshake-resp.bin");

  #[test]
  fn battery_packet_decodes_to_valid_percentages() {
      let frame = Frame::decode(BATTERY_PACKET).expect("battery.bin should be a valid frame");
      let event = Bp1ProAnc::decode_frame(&frame).expect("should decode to DeviceEvent");
      match event {
          DeviceEvent::BatteryUpdate(b) => {
              assert!(b.left_pct  <= 100, "left_pct={} out of range",  b.left_pct);
              assert!(b.right_pct <= 100, "right_pct={} out of range", b.right_pct);
              assert!(b.case_pct  <= 100, "case_pct={} out of range",  b.case_pct);
          }
          other => panic!("expected BatteryUpdate, got {:?}", other),
      }
  }

  #[test]
  fn handshake_response_decodes_without_panic() {
      let frame = Frame::decode(HANDSHAKE_RESP).expect("handshake-resp.bin should be a valid frame");
      // Just verify it doesn't panic — we may emit Connected or a model-specific event
      let _ = Bp1ProAnc::decode_frame(&frame);
  }
  ```

- [ ] **Step 2: Run — confirm they fail**
  ```
  cargo test -p baseus-protocol 2>&1
  ```
  Expected: `battery_packet_decodes_to_valid_percentages` FAILs with `not yet implemented`.

- [ ] **Step 3: Implement `Bp1ProAnc::decode_frame`**

  Fill in `crates/baseus-protocol/src/models/bp1_pro_anc.rs` with actual opcode values from `docs/protocol/bp1-pro-anc.md`:
  ```rust
  use crate::{models::DecodeError, types::{AncMode, BatteryState, DeviceEvent}, Frame};

  // ─── Opcodes — fill in from docs/protocol/bp1-pro-anc.md ─────────────────────
  // CHANGE ME: replace 0x00 placeholders with the actual bytes from Phase 0 captures.
  // Run Phase 0 Tasks 1–3 first, then update these constants.
  const OPCODE_BATTERY:   u8 = 0x00;   // battery notification from device — CHANGE ME
  const OPCODE_HANDSHAKE: u8 = 0x01;   // handshake response from device  — CHANGE ME
  const OPCODE_ANC:       u8 = 0x02;   // ANC mode event from device       — CHANGE ME
  // ─────────────────────────────────────────────────────────────────────────────

  pub struct Bp1ProAnc;

  impl Bp1ProAnc {
      pub fn decode_frame(frame: &Frame) -> Result<DeviceEvent, DecodeError> {
          match frame.cmd {
              OPCODE_BATTERY => Self::decode_battery(frame),
              OPCODE_HANDSHAKE => Ok(DeviceEvent::Connected),
              OPCODE_ANC => Self::decode_anc(frame),
              other => Err(DecodeError::UnknownOpcode(other)),
          }
      }

      fn decode_battery(frame: &Frame) -> Result<DeviceEvent, DecodeError> {
          // Battery payload: [left_pct, right_pct, case_pct, status_flags]
          // Adjust offsets if your captures show a different layout.
          if frame.payload.len() < 4 {
              return Err(DecodeError::PayloadTooShort {
                  opcode: frame.cmd,
                  need:   4,
                  got:    frame.payload.len(),
              });
          }
          let flags = frame.payload[3];
          Ok(DeviceEvent::BatteryUpdate(BatteryState {
              left_pct:       frame.payload[0].min(100),
              right_pct:      frame.payload[1].min(100),
              case_pct:       frame.payload[2].min(100),
              left_charging:  flags & 0x01 != 0,
              right_charging: flags & 0x02 != 0,
              case_charging:  flags & 0x04 != 0,
          }))
      }

      fn decode_anc(frame: &Frame) -> Result<DeviceEvent, DecodeError> {
          if frame.payload.is_empty() {
              return Err(DecodeError::PayloadTooShort { opcode: frame.cmd, need: 1, got: 0 });
          }
          let mode = match frame.payload[0] {
              0x00 => AncMode::Off,
              0x01 => AncMode::Anc,
              0x02 => AncMode::Transparency,
              _    => AncMode::Off,
          };
          Ok(DeviceEvent::AncModeUpdate(mode))
      }
  }
  ```

- [ ] **Step 4: Run golden tests — all must pass**
  ```
  cargo test -p baseus-protocol 2>&1
  ```
  Expected: `test result: ok. 7 passed; 0 failed` (5 framing + 2 golden).

- [ ] **Step 5: Commit**
  ```
  git add crates/baseus-protocol/src/models/ crates/baseus-protocol/tests/
  git commit -m "feat(protocol): implement BP1 Pro ANC battery decode with golden tests"
  ```

---

## Phase 2 — Transport Crate

### Task 8: `baseus-transport` trait + MockTransport

**Files:**
- Create: `crates/baseus-transport/Cargo.toml`
- Create: `crates/baseus-transport/src/lib.rs`

- [ ] **Step 1: Create directory**
  ```
  New-Item -ItemType Directory -Force crates\baseus-transport\src\win
  ```

- [ ] **Step 2: Create `crates/baseus-transport/Cargo.toml`**
  ```toml
  [package]
  name    = "baseus-transport"
  version = "0.1.0"
  edition = "2021"

  [dependencies]
  thiserror = { workspace = true }
  tokio     = { workspace = true }
  tracing   = { workspace = true }

  [target.'cfg(windows)'.dependencies]
  windows = { version = "0.61", features = [
      "Devices_Bluetooth",
      "Devices_Bluetooth_Rfcomm",
      "Devices_Enumeration",
      "Foundation",
      "Networking_Sockets",
      "Storage_Streams",
  ] }
  ```

- [ ] **Step 3: Create `crates/baseus-transport/src/lib.rs`**
  ```rust
  use thiserror::Error;

  #[derive(Debug, Error)]
  pub enum TransportError {
      #[error("connection failed: {0}")]
      ConnectionFailed(String),
      #[error("device not found for address {0}")]
      DeviceNotFound(u64),
      #[error("Bluetooth service not found on device")]
      ServiceNotFound,
      #[error("I/O error: {0}")]
      Io(String),
      #[error("disconnected")]
      Disconnected,
  }

  /// Abstraction over a bidirectional Bluetooth byte stream.
  /// The Windows (WinRT) implementation is in `win::rfcomm`.
  /// `MockTransport` is available for unit-testing the device event loop.
  pub trait BluetoothTransport: Send + 'static {
      async fn connect(addr: u64) -> Result<Self, TransportError>
      where
          Self: Sized;
      async fn send(&mut self, data: &[u8]) -> Result<(), TransportError>;
      /// Read the next packet from the device. Blocks until a packet arrives.
      async fn recv(&mut self, buf: &mut [u8]) -> Result<usize, TransportError>;
      async fn disconnect(&mut self) -> Result<(), TransportError>;
  }

  /// In-process mock for testing. Pre-load `recv_queue` with raw packet bytes.
  /// `send_log` records every outgoing packet for assertion.
  pub struct MockTransport {
      pub recv_queue: std::collections::VecDeque<Vec<u8>>,
      pub send_log:   Vec<Vec<u8>>,
  }

  impl MockTransport {
      pub fn new() -> Self {
          Self { recv_queue: Default::default(), send_log: Vec::new() }
      }

      pub fn push_rx(&mut self, packet: Vec<u8>) {
          self.recv_queue.push_back(packet);
      }
  }

  impl BluetoothTransport for MockTransport {
      async fn connect(_addr: u64) -> Result<Self, TransportError> {
          Ok(Self::new())
      }
      async fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
          self.send_log.push(data.to_vec());
          Ok(())
      }
      async fn recv(&mut self, buf: &mut [u8]) -> Result<usize, TransportError> {
          if let Some(packet) = self.recv_queue.pop_front() {
              let n = packet.len().min(buf.len());
              buf[..n].copy_from_slice(&packet[..n]);
              Ok(n)
          } else {
              // Signal EOF / disconnect once queue is drained
              Err(TransportError::Disconnected)
          }
      }
      async fn disconnect(&mut self) -> Result<(), TransportError> {
          Ok(())
      }
  }

  #[cfg(windows)]
  pub mod win;
  ```

- [ ] **Step 4: Create stub `crates/baseus-transport/src/win/mod.rs`**
  ```rust
  pub mod rfcomm;
  ```

- [ ] **Step 5: Create stub `crates/baseus-transport/src/win/rfcomm.rs`**
  ```rust
  use crate::{BluetoothTransport, TransportError};

  pub struct RfcommTransport {
      // Windows WinRT streams — filled in Task 9
      _private: (),
  }

  impl BluetoothTransport for RfcommTransport {
      async fn connect(_addr: u64) -> Result<Self, TransportError> {
          todo!("WinRT RFCOMM — implemented in Task 9")
      }
      async fn send(&mut self, _data: &[u8]) -> Result<(), TransportError> {
          todo!("WinRT RFCOMM — implemented in Task 9")
      }
      async fn recv(&mut self, _buf: &mut [u8]) -> Result<usize, TransportError> {
          todo!("WinRT RFCOMM — implemented in Task 9")
      }
      async fn disconnect(&mut self) -> Result<(), TransportError> {
          todo!("WinRT RFCOMM — implemented in Task 9")
      }
  }
  ```

- [ ] **Step 6: Verify compilation**
  ```
  cargo check -p baseus-transport 2>&1
  ```
  Expected: no errors (stubs compile).

- [ ] **Step 7: Commit**
  ```
  git add crates/baseus-transport/
  git commit -m "feat(transport): add BluetoothTransport trait + MockTransport"
  ```

---

### Task 9: Implement WinRT RFCOMM transport

> **Prerequisite:** Phase 0 must identify the Bluetooth service UUID and confirm RFCOMM (not BLE GATT). If Phase 0 reveals BLE GATT, implement BLE instead — the same `BluetoothTransport` trait applies; just wire `btleplug::api::Peripheral` instead.

**Files:**
- Modify: `crates/baseus-transport/src/win/rfcomm.rs`

- [ ] **Step 1: Add `windows-rs` features to Cargo.toml** (if not already added in Task 8)

  Confirm `crates/baseus-transport/Cargo.toml` has:
  ```toml
  [target.'cfg(windows)'.dependencies]
  windows = { version = "0.61", features = [
      "Devices_Bluetooth",
      "Devices_Bluetooth_Rfcomm",
      "Devices_Enumeration",
      "Foundation",
      "Networking_Sockets",
      "Storage_Streams",
  ] }
  ```

- [ ] **Step 2: Write the full WinRT RFCOMM transport implementation**

  Replace `crates/baseus-transport/src/win/rfcomm.rs` with:
  ```rust
  use crate::{BluetoothTransport, TransportError};
  use windows::{
      core::HSTRING,
      Devices::Bluetooth::{BluetoothDevice, Rfcomm::RfcommDeviceService},
      Networking::Sockets::StreamSocket,
      Storage::Streams::{DataReader, DataWriter, InputStreamOptions},
  };

  // Standard SPP service ID (UUID 0x1101). If Phase 0 reveals a custom UUID, replace
  // SerialPortServiceId() with RfcommServiceId::FromUuid(windows::core::GUID { ... }).
  // The custom GUID fields (data1/data2/data3/data4) come from the btsnoop SDP exchange.
  fn spp_service_id() -> windows::core::Result<windows::Devices::Bluetooth::Rfcomm::RfcommServiceId> {
      windows::Devices::Bluetooth::Rfcomm::RfcommServiceId::SerialPortServiceId()
  }

  pub struct RfcommTransport {
      reader: DataReader,
      writer: DataWriter,
  }

  impl BluetoothTransport for RfcommTransport {
      async fn connect(addr: u64) -> Result<Self, TransportError> {
          // addr is a 48-bit MAC address encoded as u64 (e.g., 0x001122334455)
          let device = BluetoothDevice::FromBluetoothAddressAsync(addr)
              .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?
              .await
              .map_err(|_| TransportError::DeviceNotFound(addr))?;

          let service_id = spp_service_id()
              .map_err(|e| TransportError::ServiceNotFound)?;

          let services_result = device
              .GetRfcommServicesForIdAsync(&service_id)
              .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?
              .await
              .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

          let services = services_result
              .Services()
              .map_err(|e| TransportError::ServiceNotFound)?;

          if services.Size().unwrap_or(0) == 0 {
              return Err(TransportError::ServiceNotFound);
          }

          let svc = services.GetAt(0).map_err(|e| TransportError::ServiceNotFound)?;

          let socket = StreamSocket::new()
              .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

          socket
              .ConnectAsync(
                  &svc.ConnectionHostName()
                      .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?,
                  &svc.ConnectionServiceName()
                      .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?,
              )
              .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?
              .await
              .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

          let writer = DataWriter::CreateDataWriter(
              &socket.OutputStream()
                  .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?,
          )
          .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

          let reader = DataReader::CreateDataReader(
              &socket.InputStream()
                  .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?,
          )
          .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

          reader
              .SetInputStreamOptions(InputStreamOptions::Partial)
              .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

          tracing::info!("RFCOMM connected to {:#014x}", addr);
          Ok(Self { reader, writer })
      }

      async fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
          self.writer
              .WriteBytes(data)
              .map_err(|e| TransportError::Io(e.to_string()))?;
          self.writer
              .StoreAsync()
              .map_err(|e| TransportError::Io(e.to_string()))?
              .await
              .map_err(|e| TransportError::Io(e.to_string()))?;
          Ok(())
      }

      async fn recv(&mut self, buf: &mut [u8]) -> Result<usize, TransportError> {
          // Load as many bytes as available (Partial mode)
          let loaded = self.reader
              .LoadAsync(buf.len() as u32)
              .map_err(|e| TransportError::Io(e.to_string()))?
              .await
              .map_err(|e| TransportError::Disconnected)? as usize;

          if loaded == 0 {
              return Err(TransportError::Disconnected);
          }
          self.reader
              .ReadBytes(&mut buf[..loaded])
              .map_err(|e| TransportError::Io(e.to_string()))?;
          Ok(loaded)
      }

      async fn disconnect(&mut self) -> Result<(), TransportError> {
          // Drop DataWriter gracefully (flushes pending writes)
          let _ = self.writer.FlushAsync();
          Ok(())
      }
  }
  ```

- [ ] **Step 3: Check compile on Windows**
  ```
  cargo check -p baseus-transport --target x86_64-pc-windows-msvc 2>&1
  ```
  Expected: no errors. If `windows::core::GUID` parsing fails, use `windows::core::GUID::from` or the `GUID!` macro.

- [ ] **Step 4: Manual integration test** (requires physical earbuds + Windows)

  Write a small test binary `crates/baseus-transport/examples/connect.rs`:
  ```rust
  // Usage: cargo run --example connect -- <bluetooth-address-as-hex>
  // e.g.: cargo run --example connect -- 0x001122334455
  #[tokio::main]
  async fn main() {
      tracing_subscriber::fmt::init();
      let addr_str = std::env::args().nth(1).expect("pass BT address as 0x<hex>");
      let addr = u64::from_str_radix(addr_str.trim_start_matches("0x"), 16)
          .expect("invalid hex address");
      use baseus_transport::{win::rfcomm::RfcommTransport, BluetoothTransport};
      match RfcommTransport::connect(addr).await {
          Ok(mut t) => {
              println!("Connected!");
              let mut buf = [0u8; 512];
              for _ in 0..5 {
                  match t.recv(&mut buf).await {
                      Ok(n) => println!("RX: {:?}", &buf[..n]),
                      Err(e) => { println!("Error: {e}"); break; }
                  }
              }
          }
          Err(e) => eprintln!("Connection failed: {e}"),
      }
  }
  ```
  Run with the earbuds powered on and paired to your Windows laptop:
  ```
  cargo run -p baseus-transport --example connect -- 0x<your-bt-addr>
  ```
  Expected: `Connected!` followed by raw RX bytes. If frames arrive, note them — they should match Phase 0 captures.

- [ ] **Step 5: Commit**
  ```
  git add crates/baseus-transport/src/win/rfcomm.rs crates/baseus-transport/examples/
  git commit -m "feat(transport): implement WinRT RFCOMM transport"
  ```

---

## Phase 3 — Tauri App

### Task 10: Scaffold Tauri v2 app + SolidJS frontend

**Files:** `apps/baseus-app/` (all new)

- [ ] **Step 1: Install Tauri CLI and create app**
  ```
  pnpm add -g @tauri-apps/cli
  cd apps
  pnpm create tauri-app@latest baseus-app -- --template solid-ts --manager pnpm --identifier com.baseus.desktop
  cd baseus-app
  pnpm install
  ```
  This scaffolds `src-tauri/` (Rust) and `src/` (SolidJS) automatically.

- [ ] **Step 2: Add the Tauri app to the workspace**

  Edit the root `Cargo.toml`, add to the `members` array:
  ```toml
  members = [
      "crates/baseus-protocol",
      "crates/baseus-transport",
      "apps/baseus-app/src-tauri",   # ← add this
  ]
  ```

- [ ] **Step 3: Add workspace dependencies to `apps/baseus-app/src-tauri/Cargo.toml`**

  Open the generated `apps/baseus-app/src-tauri/Cargo.toml`. Add:
  ```toml
  [dependencies]
  # ... existing tauri deps ...
  baseus-protocol  = { path = "../../../crates/baseus-protocol" }
  baseus-transport = { path = "../../../crates/baseus-transport" }
  tokio    = { workspace = true }
  tracing  = { workspace = true }
  serde    = { workspace = true }
  serde_json = { workspace = true }
  tracing-subscriber = { version = "0.3", features = ["env-filter"] }
  ```

- [ ] **Step 4: Install Tailwind CSS**
  ```
  cd apps/baseus-app
  pnpm add -D tailwindcss @tailwindcss/vite
  ```

  Create `apps/baseus-app/tailwind.config.ts`:
  ```ts
  import type { Config } from 'tailwindcss';
  export default {
    content: ['./index.html', './src/**/*.{ts,tsx}'],
    theme: { extend: {} },
    plugins: [],
  } satisfies Config;
  ```

  Edit `apps/baseus-app/vite.config.ts` to add the Tailwind plugin:
  ```ts
  import { defineConfig } from 'vite';
  import solid from 'vite-plugin-solid';
  import tailwindcss from '@tailwindcss/vite';

  export default defineConfig({
    plugins: [tailwindcss(), solid()],
    clearScreen: false,
    server: { port: 1420, strictPort: true, watch: { ignored: ['**/src-tauri/**'] } },
  });
  ```

  Edit `apps/baseus-app/src/index.css` (create if absent):
  ```css
  @import "tailwindcss";

  html, body, #root {
    height: 100%;
    margin: 0;
    background: #0a0a0a;
    color: #f5f5f5;
    font-family: system-ui, -apple-system, sans-serif;
  }
  ```

- [ ] **Step 5: Verify the scaffold runs**
  ```
  cd apps/baseus-app
  pnpm tauri dev
  ```
  Expected: Tauri window opens with the default SolidJS counter. Close it when confirmed.

- [ ] **Step 6: Commit**
  ```
  git add apps/baseus-app/ Cargo.toml
  git commit -m "feat(app): scaffold Tauri v2 + SolidJS + Tailwind"
  ```

---

### Task 11: Implement `Device` state machine (Rust backend)

**Files:**
- Create: `apps/baseus-app/src-tauri/src/device.rs`

The `Device` struct owns the transport, runs an async event loop, and broadcasts `DeviceEvent` values to all listeners (Tauri commands subscribe via a `broadcast::Receiver`).

- [ ] **Step 1: Write a failing test for device event forwarding**

  Create `apps/baseus-app/src-tauri/src/device.rs`:
  ```rust
  // tests are at the bottom of this file
  ```
  At the bottom of the file (in the same file for now), add:
  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;
      use baseus_protocol::{framing::Frame, types::{BatteryState, DeviceEvent}};
      use baseus_transport::MockTransport;

      // Build a real battery packet using the framing codec so the test uses the same bytes as production.
      fn make_battery_packet(l: u8, r: u8, c: u8) -> Vec<u8> {
          // Opcode must match OPCODE_BATTERY in bp1_pro_anc.rs — fill in after Task 7.
          // Replace 0x01 with the real battery opcode.
          Frame { cmd: 0x01, payload: vec![l, r, c, 0x00] }.encode()
      }

      #[tokio::test]
      async fn battery_event_forwarded_to_subscriber() {
          let mut mock = MockTransport::new();
          mock.push_rx(make_battery_packet(80, 75, 60));

          let (device, mut rx) = Device::new(mock, baseus_protocol::types::BaseusModel::Bp1ProAnc);
          tokio::spawn(device.run());

          let event = tokio::time::timeout(
              std::time::Duration::from_secs(1),
              async { loop { if let Ok(e) = rx.recv().await { return e; } } }
          ).await.expect("event within 1s");

          assert!(matches!(event, DeviceEvent::BatteryUpdate(BatteryState { left_pct: 80, right_pct: 75, case_pct: 60, .. })));
      }

      #[tokio::test]
      async fn disconnect_event_emitted_on_transport_error() {
          let mock = MockTransport::new(); // empty queue → recv returns Disconnected immediately
          let (device, mut rx) = Device::new(mock, baseus_protocol::types::BaseusModel::Bp1ProAnc);
          tokio::spawn(device.run());

          let event = tokio::time::timeout(
              std::time::Duration::from_secs(1),
              async { loop { if let Ok(e) = rx.recv().await { return e; } } }
          ).await.expect("disconnect event within 1s");

          assert!(matches!(event, DeviceEvent::Disconnected));
      }
  }
  ```

- [ ] **Step 2: Run tests — confirm they fail**
  ```
  cargo test -p baseus-app 2>&1
  ```
  Expected: compile error (`Device` not found).

- [ ] **Step 3: Implement `Device`**

  Fill in `device.rs` before the `#[cfg(test)]` block:
  ```rust
  use baseus_protocol::{
      framing::Frame,
      models::{bp1_pro_anc::Bp1ProAnc, DecodeError},
      types::{BaseusModel, DeviceEvent},
  };
  use baseus_transport::{BluetoothTransport, TransportError};
  use tokio::sync::broadcast;
  use tracing::{error, info, warn};

  const EVENT_CHANNEL_CAP: usize = 64;

  pub struct Device<T: BluetoothTransport> {
      transport: T,
      model:     BaseusModel,
      event_tx:  broadcast::Sender<DeviceEvent>,
  }

  impl<T: BluetoothTransport> Device<T> {
      pub fn new(transport: T, model: BaseusModel) -> (Self, broadcast::Receiver<DeviceEvent>) {
          let (tx, rx) = broadcast::channel(EVENT_CHANNEL_CAP);
          (Self { transport, model, event_tx: tx }, rx)
      }

      pub async fn run(mut self) {
          info!("device event loop started");
          let mut buf = vec![0u8; 1024];
          loop {
              match self.transport.recv(&mut buf).await {
                  Ok(n) => {
                      match Frame::decode(&buf[..n]) {
                          Ok(frame) => {
                              let result = match self.model {
                                  BaseusModel::Bp1ProAnc => Bp1ProAnc::decode_frame(&frame),
                              };
                              match result {
                                  Ok(event) => { let _ = self.event_tx.send(event); }
                                  Err(DecodeError::UnknownOpcode(op)) => {
                                      warn!("unknown opcode {op:#04x} — ignoring");
                                  }
                                  Err(e) => error!("decode error: {e}"),
                              }
                          }
                          Err(e) => warn!("framing error: {e}"),
                      }
                  }
                  Err(TransportError::Disconnected) => {
                      info!("device disconnected");
                      let _ = self.event_tx.send(DeviceEvent::Disconnected);
                      break;
                  }
                  Err(e) => {
                      error!("transport error: {e}");
                      let _ = self.event_tx.send(DeviceEvent::Disconnected);
                      break;
                  }
              }
          }
      }
  }
  ```

- [ ] **Step 4: Run tests — all should pass**
  ```
  cargo test -p baseus-app 2>&1
  ```
  Expected: `test result: ok. 2 passed; 0 failed`

- [ ] **Step 5: Commit**
  ```
  git add apps/baseus-app/src-tauri/src/device.rs
  git commit -m "feat(app): implement Device state machine with broadcast event loop"
  ```

---

### Task 12: Tauri commands + app state

**Files:**
- Modify: `apps/baseus-app/src-tauri/src/lib.rs`
- Create: `apps/baseus-app/src-tauri/src/commands.rs`

- [ ] **Step 1: Create `commands.rs`**

  Create `apps/baseus-app/src-tauri/src/commands.rs`:
  ```rust
  use baseus_protocol::types::{BatteryState, DeviceEvent};
  use serde::{Deserialize, Serialize};
  use std::sync::Arc;
  use tauri::{AppHandle, Emitter, State};
  use tokio::sync::{broadcast, Mutex};

  #[derive(Debug, Clone, Serialize, Deserialize)]
  #[serde(tag = "type", rename_all = "snake_case")]
  pub enum FrontendEvent {
      BatteryUpdate { left_pct: u8, right_pct: u8, case_pct: u8, left_charging: bool, right_charging: bool, case_charging: bool },
      Connected,
      Disconnected,
  }

  impl From<DeviceEvent> for Option<FrontendEvent> {
      fn from(e: DeviceEvent) -> Self {
          match e {
              DeviceEvent::BatteryUpdate(b) => Some(FrontendEvent::BatteryUpdate {
                  left_pct:       b.left_pct,
                  right_pct:      b.right_pct,
                  case_pct:       b.case_pct,
                  left_charging:  b.left_charging,
                  right_charging: b.right_charging,
                  case_charging:  b.case_charging,
              }),
              DeviceEvent::Connected    => Some(FrontendEvent::Connected),
              DeviceEvent::Disconnected => Some(FrontendEvent::Disconnected),
              _                         => None,
          }
      }
  }

  /// Called by the frontend to start a connection to the earbuds.
  /// `addr` is the 48-bit Bluetooth address encoded as a u64 (decimal string from JS).
  #[tauri::command]
  pub async fn connect(app: AppHandle, addr: u64) -> Result<(), String> {
      use baseus_transport::win::rfcomm::RfcommTransport;
      use baseus_protocol::types::BaseusModel;
      use crate::device::Device;

      let transport = RfcommTransport::connect(addr).await.map_err(|e| e.to_string())?;
      let (device, mut rx) = Device::new(transport, BaseusModel::Bp1ProAnc);

      tokio::spawn(device.run());

      // Forward device events to the JS frontend as Tauri events
      tokio::spawn(async move {
          loop {
              match rx.recv().await {
                  Ok(event) => {
                      if let Some(fe) = Option::<FrontendEvent>::from(event) {
                          let _ = app.emit("device-event", &fe);
                      }
                  }
                  Err(broadcast::error::RecvError::Closed) => break,
                  Err(broadcast::error::RecvError::Lagged(n)) => {
                      tracing::warn!("event channel lagged by {n}");
                  }
              }
          }
      });

      Ok(())
  }
  ```

- [ ] **Step 2: Wire commands into `lib.rs`**

  Replace the contents of `apps/baseus-app/src-tauri/src/lib.rs` with:
  ```rust
  mod commands;
  mod device;

  pub fn run() {
      tracing_subscriber::fmt()
          .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
          .init();

      tauri::Builder::default()
          .invoke_handler(tauri::generate_handler![commands::connect])
          .run(tauri::generate_context!())
          .expect("error while running tauri application");
  }
  ```

- [ ] **Step 3: Verify compile**
  ```
  cargo check -p baseus-app 2>&1
  ```
  Expected: no errors.

- [ ] **Step 4: Commit**
  ```
  git add apps/baseus-app/src-tauri/src/commands.rs apps/baseus-app/src-tauri/src/lib.rs
  git commit -m "feat(app): add connect command + device-event forwarding to frontend"
  ```

---

### Task 13: SolidJS UI components

**Files:**
- Modify: `apps/baseus-app/src/App.tsx`
- Create: `apps/baseus-app/src/components/BatteryCard.tsx`
- Create: `apps/baseus-app/src/components/ConnectionCard.tsx`
- Create: `apps/baseus-app/src/lib/tauri.ts`

- [ ] **Step 1: Create typed Tauri wrappers**

  Create `apps/baseus-app/src/lib/tauri.ts`:
  ```ts
  import { invoke } from '@tauri-apps/api/core';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';

  export interface BatteryState {
    left_pct: number; right_pct: number; case_pct: number;
    left_charging: boolean; right_charging: boolean; case_charging: boolean;
  }

  export type DeviceEvent =
    | { type: 'battery_update' } & BatteryState
    | { type: 'connected' }
    | { type: 'disconnected' };

  export function connectDevice(addr: bigint): Promise<void> {
    return invoke('connect', { addr: Number(addr) });
  }

  export function onDeviceEvent(cb: (e: DeviceEvent) => void): Promise<UnlistenFn> {
    return listen<DeviceEvent>('device-event', (event) => cb(event.payload));
  }
  ```

- [ ] **Step 2: Create `BatteryCard.tsx`**

  Create `apps/baseus-app/src/components/BatteryCard.tsx`:
  ```tsx
  import { Component } from 'solid-js';

  interface Props {
    label:    string;
    pct:      number;
    charging: boolean;
  }

  const BatteryCard: Component<Props> = (props) => {
    const RADIUS = 38;
    const CIRC   = 2 * Math.PI * RADIUS;
    const offset = () => CIRC - (Math.max(0, Math.min(100, props.pct)) / 100) * CIRC;
    const color  = () => props.pct > 20 ? '#22c55e' : '#ef4444';

    return (
      <div class="flex flex-col items-center gap-3 rounded-2xl bg-neutral-900 p-6 w-40">
        <div class="relative w-24 h-24">
          <svg class="absolute inset-0 -rotate-90" viewBox="0 0 88 88">
            <circle cx="44" cy="44" r={RADIUS} fill="none" stroke="#262626" stroke-width="8" />
            <circle
              cx="44" cy="44" r={RADIUS}
              fill="none" stroke={color()} stroke-width="8"
              stroke-dasharray={CIRC} stroke-dashoffset={offset()}
              stroke-linecap="round"
              style="transition: stroke-dashoffset 0.5s ease, stroke 0.3s ease"
            />
          </svg>
          <span class="absolute inset-0 flex items-center justify-center text-2xl font-bold tabular-nums">
            {props.pct}
          </span>
        </div>
        <span class="text-sm text-neutral-400 font-medium">{props.label}</span>
        {props.charging && (
          <span class="text-xs text-yellow-400 font-semibold tracking-wide">CHARGING</span>
        )}
      </div>
    );
  };

  export default BatteryCard;
  ```

- [ ] **Step 3: Create `ConnectionCard.tsx`**

  Create `apps/baseus-app/src/components/ConnectionCard.tsx`:
  ```tsx
  import { Component } from 'solid-js';

  type Status = 'connected' | 'connecting' | 'disconnected';

  interface Props { status: Status; lastUpdated: string | null }

  const STATUS_COLOR: Record<Status, string> = {
    connected:    'bg-green-500',
    connecting:   'bg-yellow-500 animate-pulse',
    disconnected: 'bg-neutral-600',
  };

  const STATUS_LABEL: Record<Status, string> = {
    connected:    'Connected',
    connecting:   'Connecting…',
    disconnected: 'Disconnected',
  };

  const ConnectionCard: Component<Props> = (props) => (
    <div class="flex items-center gap-3 rounded-2xl bg-neutral-900 px-5 py-3 w-full max-w-sm">
      <span class={`w-3 h-3 rounded-full flex-shrink-0 ${STATUS_COLOR[props.status]}`} />
      <div class="flex flex-col min-w-0">
        <span class="text-sm font-semibold">{STATUS_LABEL[props.status]}</span>
        {props.lastUpdated && (
          <span class="text-xs text-neutral-500 truncate">Updated {props.lastUpdated}</span>
        )}
      </div>
    </div>
  );

  export default ConnectionCard;
  ```

- [ ] **Step 4: Rewrite `App.tsx`**

  Replace `apps/baseus-app/src/App.tsx` with:
  ```tsx
  import { createSignal, onCleanup, onMount } from 'solid-js';
  import BatteryCard from './components/BatteryCard';
  import ConnectionCard from './components/ConnectionCard';
  import { BatteryState, connectDevice, onDeviceEvent } from './lib/tauri';

  // The Bluetooth address of the BP1 Pro ANC.
  // Hardcode for v1 — the user pairs the device first in Windows BT settings.
  // In a future version, enumerate paired devices via a Tauri command.
  const DEVICE_ADDR = BigInt('0x' + (import.meta.env.VITE_BT_ADDR ?? '000000000000'));

  type ConnStatus = 'connected' | 'connecting' | 'disconnected';

  export default function App() {
    const [status, setStatus]   = createSignal<ConnStatus>('connecting');
    const [battery, setBattery] = createSignal<BatteryState | null>(null);
    const [lastUpd, setLastUpd] = createSignal<string | null>(null);

    onMount(async () => {
      const unlisten = await onDeviceEvent((e) => {
        if (e.type === 'battery_update') {
          setBattery(e);
          setStatus('connected');
          setLastUpd(new Date().toLocaleTimeString());
        } else if (e.type === 'connected') {
          setStatus('connected');
        } else if (e.type === 'disconnected') {
          setStatus('disconnected');
        }
      });
      onCleanup(unlisten);

      try {
        await connectDevice(DEVICE_ADDR);
      } catch (err) {
        console.error('connect failed:', err);
        setStatus('disconnected');
      }
    });

    return (
      <div class="min-h-screen bg-neutral-950 flex flex-col items-center justify-center gap-6 p-6">
        <h1 class="text-lg font-semibold text-neutral-200 tracking-tight">Baseus Desktop</h1>
        <ConnectionCard status={status()} lastUpdated={lastUpd()} />
        <div class="flex gap-4 flex-wrap justify-center">
          <BatteryCard label="Left"  pct={battery()?.left_pct  ?? 0} charging={battery()?.left_charging  ?? false} />
          <BatteryCard label="Right" pct={battery()?.right_pct ?? 0} charging={battery()?.right_charging ?? false} />
          <BatteryCard label="Case"  pct={battery()?.case_pct  ?? 0} charging={battery()?.case_charging  ?? false} />
        </div>
        <p class="text-xs text-neutral-600">
          {status() === 'disconnected' ? 'Open the case to reconnect.' : 'Showing live battery readings.'}
        </p>
      </div>
    );
  }
  ```

- [ ] **Step 5: Add `VITE_BT_ADDR` to `.env.local`**

  Create `apps/baseus-app/.env.local`:
  ```
  # 48-bit Bluetooth address of your BP1 Pro ANC (no colons, hex digits only)
  # Find it in Windows Settings → Bluetooth → Devices → your earbuds → Properties
  VITE_BT_ADDR=AABBCCDDEEFF
  ```

- [ ] **Step 6: Start dev server and verify UI renders**
  ```
  cd apps/baseus-app
  pnpm tauri dev
  ```
  Expected: dark window with "Baseus Desktop" heading, connection status card, and three battery cards showing 0%. Connection will fail because the earbuds aren't connected yet — that's fine. Verify no TypeScript errors in the console.

- [ ] **Step 7: Commit**
  ```
  git add apps/baseus-app/src/ apps/baseus-app/.env.local
  git commit -m "feat(ui): add BatteryCard + ConnectionCard + App with live event subscription"
  ```

---

### Task 14: Tray icon

**Files:**
- Modify: `apps/baseus-app/src-tauri/src/lib.rs`
- Create: `apps/baseus-app/src-tauri/src/tray.rs`

- [ ] **Step 1: Add tray plugin to `src-tauri/Cargo.toml`**
  ```toml
  tauri-plugin-tray-icon = "2"
  ```

- [ ] **Step 2: Create `tray.rs`**

  Create `apps/baseus-app/src-tauri/src/tray.rs`:
  ```rust
  use tauri::{
      tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
      AppHandle, Manager,
  };

  pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
      TrayIconBuilder::new()
          .icon(app.default_window_icon().unwrap().clone())
          .tooltip("Baseus Desktop")
          .on_tray_icon_event(|tray, event| {
              if let TrayIconEvent::Click {
                  button: MouseButton::Left,
                  button_state: MouseButtonState::Up,
                  ..
              } = event
              {
                  let app = tray.app_handle();
                  if let Some(window) = app.get_webview_window("main") {
                      let _ = window.show();
                      let _ = window.set_focus();
                  }
              }
          })
          .build(app)?;
      Ok(())
  }

  /// Update the tray tooltip to show current battery percentage.
  pub fn update_tray_battery(app: &AppHandle, pct: u8) {
      // Get the first tray icon (we only have one)
      if let Some(tray) = app.tray_by_id("default").or_else(|| {
          app.tray_by_id("1") // fallback id
      }) {
          let _ = tray.set_tooltip(Some(&format!("Baseus — {}%", pct)));
      }
  }
  ```

- [ ] **Step 3: Wire tray into `lib.rs`**

  Update `apps/baseus-app/src-tauri/src/lib.rs`:
  ```rust
  mod commands;
  mod device;
  mod tray;

  pub fn run() {
      tracing_subscriber::fmt()
          .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
          .init();

      tauri::Builder::default()
          .plugin(tauri_plugin_tray_icon::init())
          .setup(|app| {
              tray::setup_tray(&app.handle())?;
              // Hide the window from the taskbar on startup (tray-only until clicked)
              if let Some(window) = app.get_webview_window("main") {
                  let _ = window.hide();
              }
              Ok(())
          })
          .invoke_handler(tauri::generate_handler![commands::connect])
          .run(tauri::generate_context!())
          .expect("error while running tauri application");
  }
  ```

- [ ] **Step 4: Update the Tauri capabilities file to allow tray**

  Edit `apps/baseus-app/src-tauri/capabilities/default.json`:
  ```json
  {
    "$schema": "../gen/schemas/desktop-schema.json",
    "identifier": "default",
    "description": "Default capabilities",
    "windows": ["main"],
    "permissions": [
      "core:default",
      "tray-icon:default"
    ]
  }
  ```

- [ ] **Step 5: Verify tray appears on `pnpm tauri dev`**
  ```
  cd apps/baseus-app && pnpm tauri dev
  ```
  Expected: tray icon appears in the system tray. Clicking it shows/focuses the main window. Window starts hidden (you only see it via tray click).

- [ ] **Step 6: Commit**
  ```
  git add apps/baseus-app/src-tauri/src/tray.rs apps/baseus-app/src-tauri/src/lib.rs apps/baseus-app/src-tauri/capabilities/
  git commit -m "feat(app): add system tray icon with click-to-show behaviour"
  ```

---

### Task 15: CI, final checks, GitHub push

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Create GitHub Actions workflow**

  Create `.github/workflows/ci.yml`:
  ```yaml
  name: CI

  on:
    push:
      branches: [main]
    pull_request:
      branches: [main]

  jobs:
    rust:
      runs-on: windows-latest
      steps:
        - uses: actions/checkout@v4
        - uses: dtolnay/rust-toolchain@stable
        - uses: Swatinem/rust-cache@v2
        - name: Check
          run: cargo check --workspace
        - name: Clippy
          run: cargo clippy --workspace -- -D warnings
        - name: Format
          run: cargo fmt --all -- --check
        - name: Tests
          run: cargo test -p baseus-protocol

    frontend:
      runs-on: windows-latest
      steps:
        - uses: actions/checkout@v4
        - uses: pnpm/action-setup@v4
          with: { version: latest }
        - uses: actions/setup-node@v4
          with: { node-version: '20', cache: 'pnpm', cache-dependency-path: 'apps/baseus-app/pnpm-lock.yaml' }
        - name: Install
          run: pnpm install
          working-directory: apps/baseus-app
        - name: Type-check
          run: pnpm tsc --noEmit
          working-directory: apps/baseus-app
  ```

- [ ] **Step 2: Run CI checks locally before pushing**
  ```
  cargo fmt --all -- --check
  cargo clippy --workspace -- -D warnings
  cargo test -p baseus-protocol
  cd apps/baseus-app && pnpm tsc --noEmit
  ```
  Fix any warnings before proceeding.

- [ ] **Step 3: Create GitHub repo and push**
  ```
  gh repo create baseus-desktop --public --description "Open-source Windows desktop client for Baseus earbuds"
  git remote add origin https://github.com/<your-user>/baseus-desktop.git
  git push -u origin main
  ```

- [ ] **Step 4: Final commit (CI file)**
  ```
  git add .github/
  git commit -m "ci: add GitHub Actions workflow for Rust + frontend checks"
  git push
  ```

---

## Verification Checklist

After completing all tasks:

1. `cargo test -p baseus-protocol` — all framing + golden tests pass.
2. `cargo clippy --workspace -- -D warnings` — zero warnings.
3. `pnpm tauri dev` — app starts, tray icon appears, clicking tray shows main window.
4. **Hardware test** — with BP1 Pro ANC powered on and paired, open the case: battery cards update within 5s.
5. **Case charge test** — plug case into USB: charging flag appears on case card.
6. **Disconnect test** — close case: connection card transitions to Disconnected within 10s.
7. GitHub Actions CI passes on push to `main`.
