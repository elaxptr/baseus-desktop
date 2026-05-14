"""
find_battery_opcode.py — Find the battery notification opcode by tracing
EarBatteryHolder.Type2 / BatteryHolder.Type2 construction sites.

Run: python docs/frida/find_battery_opcode.py
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

OUT = open("docs/protocol/captures/battery-opcode.txt", "w", encoding="utf-8", errors="replace")

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

# Strategy 1: Find all methods that call EarBatteryHolder.Type2 or BatteryHolder.Type2 <init>
TARGET_CONSTRUCTORS = [
    "EarBatteryHolder$Type2",
    "BatteryHolder$Type2",
    "EarBatteryHolder$Type0",
    "BatteryHolder$Type0",
    "EarBatteryHolder$Type1",
    "BatteryHolder$Type1",
]

write("=" * 72)
write("Methods that construct BatteryHolder / EarBatteryHolder instances")
write("=" * 72)

battery_callers = set()
for class_analysis in dx.get_classes():
    vm_class = class_analysis.get_vm_class()
    cls_name = vm_class.get_name()
    for m in vm_class.get_methods():
        code = m.get_code()
        if code is None:
            continue
        bc = code.get_bc()
        for insn in bc.get_instructions():
            if insn.get_name() not in ("invoke-direct", "new-instance"):
                continue
            try:
                ops = insn.get_operands()
                for op in ops:
                    if isinstance(op, tuple) and len(op) >= 1:
                        ref = op[-1] if isinstance(op[-1], str) else None
                        if ref and any(t in ref for t in TARGET_CONSTRUCTORS):
                            key = (cls_name, m.get_name(), m.get_descriptor())
                            battery_callers.add(key)
            except Exception:
                pass

write(f"\nFound {len(battery_callers)} methods constructing BatteryHolder instances:\n")
for cls, name, desc in sorted(battery_callers):
    write(f"  {cls.replace('/', '.').strip('L;')}.{name}{desc}")

# Dump bytecode of each such method
write("\n\n" + "=" * 72)
write("Bytecode of battery-constructing methods")
write("=" * 72)

for (cls_name, meth_name, meth_desc) in sorted(battery_callers):
    for class_analysis in dx.get_classes():
        vm_class = class_analysis.get_vm_class()
        if vm_class.get_name() != cls_name:
            continue
        for m in vm_class.get_methods():
            if m.get_name() != meth_name or m.get_descriptor() != meth_desc:
                continue
            cls_pretty = cls_name.replace("/", ".").strip("L;")
            write(f"\n{'='*72}")
            write(f"CLASS:  {cls_pretty}")
            write(f"METHOD: {meth_name}{meth_desc}")
            write("")
            code = m.get_code()
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

# Strategy 2: Also dump HomeBleDataResolvePresenter.B() - the main dispatcher
write("\n\n" + "=" * 72)
write("HomeBleDataResolvePresenter non-f/g/h methods (all)")
write("=" * 72)

for class_analysis in dx.get_classes():
    vm_class = class_analysis.get_vm_class()
    cls_name = vm_class.get_name()
    if "HomeBleDataResolvePresenter" not in cls_name:
        continue
    if "$" in cls_name:
        continue
    cls_pretty = cls_name.replace("/", ".").strip("L;")
    for m in vm_class.get_methods():
        mname = m.get_name()
        if mname in ("f", "g", "h", "<init>", "<clinit>"):
            continue
        write(f"\n{'='*72}")
        write(f"CLASS:  {cls_pretty}")
        write(f"METHOD: {mname}{m.get_descriptor()}")
        write("")
        code = m.get_code()
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
print("\n[*] Saved to docs/protocol/captures/battery-opcode.txt")
