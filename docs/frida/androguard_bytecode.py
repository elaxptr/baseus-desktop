"""
androguard_bytecode.py — Dump bytecode of specific protocol methods.

Run: python docs/frida/androguard_bytecode.py

Targets:
  - HomeBleDataResolvePresenter$2.e([B)  — incoming BLE notification handler
  - HomeBleDataResolvePresenter.f/g/h    — BLE data parsers
  - BleEnhancedApi (all methods)         — outgoing command builders
  - BleApi                               — outgoing command builders
  - NoiseReduceManger.k                  — ANC mode setter
  - EarFunctionManager.g                 — earphone function manager
  - HeadPhoneDataResolveManager          — main BLE packet dispatcher
  - NoiseReduceDataModel                 — ANC mode command sender (.l method)
  - BatteryHolder                        — battery data structure
  - BletoothImpl / BleManagerImpl        — actual BleApi implementation
  - BleManager                           — BLE session manager
"""

import sys, re
from pathlib import Path

try:
    from androguard.misc import AnalyzeAPK
except ImportError:
    print("pip install androguard")
    sys.exit(1)

APK_PATH = Path("docs/protocol/captures/baseus.apk")
print(f"[*] Loading APK...")
a, d_list, dx = AnalyzeAPK(str(APK_PATH))
print(f"[*] Loaded.")

TARGETS = [
    # (class_fragment, method_name_fragment, desc_fragment)
    ("HomeBleDataResolvePresenter$2", "e", "[B"),
    ("HomeBleDataResolvePresenter$2", "b", None),
    ("HomeBleDataResolvePresenter$2", "d", None),
    ("HomeBleDataResolvePresenter", "f", "[B"),
    ("HomeBleDataResolvePresenter", "g", "[B"),
    ("HomeBleDataResolvePresenter", "h", "[B"),
    ("BleEnhancedApi", None, None),
    ("BleApi", None, None),
    ("NoiseReduceManger", "k", None),
    ("NoiseReduceManger", "b", None),
    ("NoiseReduceManger2", "b", None),
    ("NoiseReduceManger2", "k", None),
    ("EarFunctionManager", "g", None),
    ("EarFunctionManager2", "f", None),
    ("EarFunctionManager2", "i", None),
    ("SimpleBleData", None, None),
    ("BleDataBean", None, None),
    ("EarPhoneActivity", None, None),
    ("EarPodNewActivity", None, "[B"),
    # Additional targets for protocol decoding
    ("HeadPhoneDataResolveManager", None, None),
    ("NoiseReduceDataModel", None, None),
    ("BatteryHolder", None, None),
    ("BletoothImpl", None, None),
    ("BleManagerImpl", None, None),
    ("BleManager", None, None),
    ("BleUtils", None, None),
]

def decode_operands(insn):
    ops = insn.get_operands()
    parts = []
    for op in ops:
        if not isinstance(op, tuple):
            parts.append(str(op))
            continue
        if len(op) == 3 and op[0] == 2:  # string ref
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

OUT = open("docs/protocol/captures/bytecode-dump2.txt", "w", encoding="utf-8")

def write(s):
    print(s)
    OUT.write(s + "\n")

for class_analysis in dx.get_classes():
    vm_class = class_analysis.get_vm_class()
    cls_name = vm_class.get_name()
    cls_pretty = cls_name.replace("/", ".").strip("L;")

    for (cls_frag, method_frag, desc_frag) in TARGETS:
        if cls_frag not in cls_pretty:
            continue

        for m in vm_class.get_methods():
            mname = m.get_name()
            mdesc = m.get_descriptor()

            if method_frag and method_frag != mname:
                continue
            if desc_frag and desc_frag not in mdesc:
                continue

            code = m.get_code()
            write(f"\n{'='*72}")
            write(f"CLASS:  {cls_pretty}")
            write(f"METHOD: {mname}{mdesc}")
            write("")

            if code is None:
                write("  (no code — abstract/native)")
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
write("\n[*] Saved to docs/protocol/captures/bytecode-dump2.txt")
