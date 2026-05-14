"""
find_device_battery_sender.py — Find the method that sends the "device_battery"
Intent broadcast, revealing the BLE notification opcode for battery data.

Run: python docs/frida/find_device_battery_sender.py
"""

import sys
from pathlib import Path

try:
    from androguard.misc import AnalyzeAPK
except ImportError:
    print("pip install androguard")
    sys.exit(1)

APK_PATH = Path("docs/protocol/captures/baseus.apk")
print("[*] Loading APK...")
a, d_list, dx = AnalyzeAPK(str(APK_PATH))
print("[*] Loaded.")

OUT = open("docs/protocol/captures/battery-sender.txt", "w", encoding="utf-8", errors="replace")

def write(s):
    print(s)
    OUT.write(s + "\n")

def decode_operands(insn):
    ops = insn.get_operands()
    parts = []
    for op in ops:
        if not isinstance(op, tuple):
            parts.append(str(op))
            continue
        if len(op) == 3 and op[0] == 2:
            parts.append(f'"{op[2]}"')
        elif len(op) == 3:
            v = op[2]
            if isinstance(v, int):
                parts.append(f"0x{v:02X}" if abs(v) > 9 else str(v))
            else:
                parts.append(str(v))
        elif len(op) == 2:
            v = op[1]
            if isinstance(v, int):
                parts.append(f"0x{v:02X}" if abs(v) > 9 else str(v))
            else:
                parts.append(str(v))
        else:
            parts.append(str(op))
    return ", ".join(parts)

# Find all methods that reference "device_battery" string
TARGET_STRINGS = ["device_battery", "send_device_msg"]

write("=" * 72)
write("Methods referencing 'device_battery' / 'send_device_msg'")
write("=" * 72)

battery_senders = {}
for string_analysis in dx.get_strings():
    s = str(string_analysis.get_value())
    if any(t in s for t in TARGET_STRINGS):
        for _, method in string_analysis.get_xref_from():
            m = method.get_method()
            key = (m.get_class_name(), m.get_name(), m.get_descriptor())
            if key not in battery_senders:
                battery_senders[key] = (method, set())
            battery_senders[key][1].add(s)

for (cls, name, desc), (m_analysis, strings) in sorted(battery_senders.items()):
    write(f"  [{', '.join(sorted(strings))}]")
    write(f"  {cls.replace('/', '.').strip('L;')}.{name}{desc}")
    write("")

# Dump bytecode of each method
write("\n" + "=" * 72)
write("Bytecode of battery-sender methods")
write("=" * 72)

for (cls_name, meth_name, meth_desc), (m_analysis, strings) in sorted(battery_senders.items()):
    m = m_analysis.get_method()
    code = m.get_code()
    cls_pretty = cls_name.replace("/", ".").strip("L;")
    write(f"\n{'='*72}")
    write(f"CLASS:  {cls_pretty}")
    write(f"METHOD: {meth_name}{meth_desc}")
    write(f"STRINGS: {', '.join(sorted(strings))}")
    write("")
    if code is None:
        write("  (no code)")
        continue
    bc = code.get_bc()
    for insn in bc.get_instructions():
        try:
            name = insn.get_name()
            ops = decode_operands(insn)
            write(f"  {name:<28} {ops}")
        except Exception as e:
            write(f"  {insn.get_name()} [err: {e}]")

OUT.close()
print("\n[*] Saved to docs/protocol/captures/battery-sender.txt")
