#!/usr/bin/env python3
"""
Extract btsnoop_hci.log from an adb bugreport zip and copy it to
docs/protocol/captures/btsnoop_hci.log.

Usage:
    python docs/frida/extract_btsnoop.py <bugreport.zip> [output_path]
"""

import sys
import zipfile
import pathlib
import struct

DEFAULT_OUT = pathlib.Path("docs/protocol/captures/btsnoop_hci.log")

BTSNOOP_MAGIC = b"btsnoop\x00"


def find_btsnoop(zf: zipfile.ZipFile) -> str | None:
    candidates = [
        n for n in zf.namelist()
        if "btsnoop_hci" in n.lower() and n.endswith(".log")
    ]
    if not candidates:
        # Broader search
        candidates = [n for n in zf.namelist() if "btsnoop" in n.lower()]
    return candidates[0] if candidates else None


def summarise(path: pathlib.Path) -> None:
    data = path.read_bytes()
    if not data.startswith(BTSNOOP_MAGIC):
        print(f"  WARNING: file does not start with btsnoop magic — may be corrupt")
        return

    # BTSnoop file header: 8 magic + 4 version + 4 datalink
    version, datalink = struct.unpack_from(">II", data, 8)
    print(f"  BTSnoop version={version}, datalink={datalink}")

    # Count records: each record = 4+4+4+8 = 24 byte header + payload
    offset = 16
    records = 0
    while offset + 24 <= len(data):
        orig_len, incl_len, flags, drops = struct.unpack_from(">IIII", data, offset)
        offset += 24 + incl_len
        records += 1

    print(f"  {records} HCI records, {len(data):,} bytes total")
    print(f"  Saved to {path}")


def main():
    if len(sys.argv) < 2:
        print("Usage: extract_btsnoop.py <bugreport.zip> [output_path]")
        sys.exit(1)

    zip_path = pathlib.Path(sys.argv[1])
    out_path = pathlib.Path(sys.argv[2]) if len(sys.argv) > 2 else DEFAULT_OUT

    if not zip_path.exists():
        print(f"ERROR: {zip_path} not found")
        sys.exit(1)

    out_path.parent.mkdir(parents=True, exist_ok=True)

    with zipfile.ZipFile(zip_path) as zf:
        name = find_btsnoop(zf)
        if name is None:
            print("ERROR: no btsnoop_hci.log found in the bugreport zip")
            print("Files containing 'log' in the zip:")
            for n in zf.namelist():
                if n.endswith(".log"):
                    print(f"  {n}")
            sys.exit(1)

        print(f"  Extracting: {name}")
        data = zf.read(name)

    out_path.write_bytes(data)
    summarise(out_path)


if __name__ == "__main__":
    main()
