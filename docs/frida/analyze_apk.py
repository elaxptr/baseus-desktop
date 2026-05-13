"""
Extract readable strings from Baseus APK DEX files using a strings-like scan.
No DEX format parsing — raw printable ASCII scan, then filter by keywords.
"""
import zipfile, re, os, struct

APK = r"docs\protocol\captures\baseus.apk"
OUT = r"docs\protocol\captures\apk-analysis.log"

lines = []
def log(s):
    lines.append(s)
    print(s)

def extract_strings(data, min_len=5):
    """Scan binary blob for printable ASCII runs (like `strings` command)."""
    pattern = re.compile(rb'[ -~\t]{%d,}' % min_len)
    return [m.group().decode('ascii', errors='replace') for m in pattern.finditer(data)]

proto_keywords = [
    'bluetooth', 'rfcomm', 'socket', 'protocol', 'packet', 'frame',
    'opcode', 'command', 'headset', 'earphone', 'earbud', 'bp1',
    'battery', 'magic', 'header', 'anc', 'noise', 'serial',
    'connect', 'gatt', 'spp', '0xaa', '0xab', '0xac', 'cmd',
    'baseus', 'uuid', 'handshake', 'checksum', 'crc',
]

with zipfile.ZipFile(APK) as z:
    dex_files = sorted([n for n in z.namelist() if re.match(r'classes\d*\.dex', n)])
    log(f"DEX files: {dex_files}")

    all_strings = set()

    for dex_name in dex_files:
        log(f"\n=== {dex_name} ===")
        data = z.read(dex_name)
        log(f"Size: {len(data):,} bytes")

        strings = extract_strings(data, min_len=5)
        log(f"Strings found: {len(strings):,}")
        all_strings.update(strings)

        uuid_hits = [s for s in strings if re.search(
            r'[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}', s)]
        if uuid_hits:
            log(f"\n--- UUIDs ({len(uuid_hits)}) ---")
            for s in sorted(set(uuid_hits)):
                log(f"  {s}")

        # Raw 0xAA-prefixed sequences (potential protocol frame headers)
        raw_aa = []
        for i in range(len(data) - 12):
            if data[i] == 0xAA:
                chunk = data[i:i+16]
                raw_aa.append(chunk.hex())
        if raw_aa:
            unique_aa = list(dict.fromkeys(raw_aa))
            log(f"\n--- Raw 0xAA sequences (first 30 unique) ---")
            for h in unique_aa[:30]:
                log(f"  {h}")

    log(f"\n\n=== PROTOCOL KEYWORD HITS (all DEX) ===")
    hits = [s for s in sorted(all_strings) if any(k in s.lower() for k in proto_keywords)]
    log(f"Total keyword hits: {len(hits)}")
    for s in hits[:1000]:
        log(f"  {repr(s)}")

    log(f"\n\n=== HEX CONSTANTS (all DEX) ===")
    hex_hits = [s for s in sorted(all_strings)
                if re.search(r'0[xX][0-9a-fA-F]{2,}', s)
                or re.match(r'^[0-9a-fA-F]{8,}$', s)]
    log(f"Total: {len(hex_hits)}")
    for s in sorted(set(hex_hits))[:300]:
        log(f"  {repr(s)}")

    log(f"\n\n=== CLASS NAMES (baseus / protocol / bluetooth) ===")
    class_keywords = ['baseus', 'bluetooth', 'rfcomm', 'protocol', 'packet',
                      'frame', 'opcode', 'headset', 'earphone', 'earbud',
                      'battery', 'anc', 'spp', 'gatt', 'serial', 'command']
    class_hits = [s for s in sorted(all_strings)
                  if '.' in s and any(k in s.lower() for k in class_keywords)]
    log(f"Total: {len(class_hits)}")
    for s in class_hits[:500]:
        log(f"  {s}")

with open(OUT, 'w', encoding='utf-8') as f:
    f.write('\n'.join(lines))
log(f"\nSaved to {OUT}")
