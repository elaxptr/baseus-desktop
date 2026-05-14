"""
androguard_analyze.py — Find protocol command bytes in Baseus APK.

Run from repo root:
  python docs/frida/androguard_analyze.py

Searches for methods that reference key strings like:
  buildBatteryNotification, queryBattery, opCode, filterNonofifyCmdMessage
Then prints their bytecode so we can find command byte constants.
"""

import sys, re
from pathlib import Path

try:
    from androguard.misc import AnalyzeAPK
except ImportError:
    print("pip install androguard")
    sys.exit(1)

APK_PATH = Path("docs/protocol/captures/baseus.apk")
if not APK_PATH.exists():
    print(f"APK not found: {APK_PATH}")
    sys.exit(1)

print(f"[*] Loading APK (may take 60s)...")
a, d_list, dx = AnalyzeAPK(str(APK_PATH))
print(f"[*] Loaded. {len(d_list)} DEX files.")

TARGET_STRINGS = [
    "buildBatteryNotification",
    "queryBattery",
    "filterNonofifyCmdMessage",
    "filterNonofifyCmd",
    "onSoundChannelBalance",
    "send data timeout",
    "opCode",
    "HomeBleDataResolve",
    "synchronizeAccountToDevice",
    "decviceType",
    "TWS_CONNECT",
    "BASS_BP1",
    "BassBP1",
    "bass_bp1",
    "BP1",
    "noise_mode",
    "eqDataAnc",
]

# ─── 1. Find methods that reference target strings ──────────────────────────

print("\n=== Methods referencing key strings ===\n")
method_hits = {}  # MethodAnalysis -> set of matched strings

for string_analysis in dx.get_strings():
    s = str(string_analysis.get_value())
    matched = [t for t in TARGET_STRINGS if t.lower() in s.lower()]
    if not matched:
        continue
    for _, method in string_analysis.get_xref_from():
        m = method.get_method()
        key = (m.get_class_name(), m.get_name(), m.get_descriptor())
        if key not in method_hits:
            method_hits[key] = (method, set())
        method_hits[key][1].update(matched)

for (cls, name, desc), (m_analysis, keywords) in sorted(method_hits.items()):
    print(f"  [{', '.join(sorted(keywords))}]")
    print(f"  {cls.replace('/', '.').strip('L;')}.{name}{desc}")
    print()

# ─── 2. Print bytecode for the most interesting methods ─────────────────────

PRIORITY = ["buildBatteryNotification", "queryBattery", "filterNonofifyCmdMessage",
            "HomeBleDataResolve", "synchronizeAccountToDevice", "opCode"]

print("\n=== Bytecode of high-priority methods ===\n")

for (cls, name, desc), (m_analysis, keywords) in sorted(method_hits.items()):
    if not any(p.lower() in kw.lower() for p in PRIORITY for kw in keywords):
        continue
    m = m_analysis.get_method()
    code = m.get_code()
    if code is None:
        continue

    print(f"\n{'='*70}")
    print(f"CLASS: {cls.replace('/', '.').strip('L;')}")
    print(f"METHOD: {name}{desc}")
    print(f"STRINGS: {', '.join(sorted(keywords))}")
    print()

    # Print Dalvik bytecode
    bc = code.get_bc()
    for insn in bc.get_instructions():
        try:
            ops = insn.get_operands()
            op_strs = []
            for op in ops:
                if isinstance(op, tuple):
                    if len(op) == 3 and op[0] == 2:  # string ref
                        op_strs.append(f'"{op[2]}"')
                    elif len(op) == 3:
                        op_strs.append(str(op[2]))
                    elif len(op) == 2:
                        v = op[1]
                        op_strs.append(f"0x{v:02X}({v})" if isinstance(v, int) else str(v))
                    else:
                        op_strs.append(str(op))
                else:
                    op_strs.append(str(op))
            print(f"  {insn.get_name():30s} {', '.join(op_strs)}")
        except Exception as e:
            print(f"  {insn.get_name()} [err: {e}]")

# ─── 3. Find all classes that contain "HomeBleDataResolvePresenter" ──────────

print("\n\n=== HomeBleDataResolvePresenter class ===\n")
for class_analysis in dx.get_classes():
    vm_class = class_analysis.get_vm_class()
    cls_name = vm_class.get_name()
    if 'HomeBleDataResolve' not in cls_name and 'HomeBleData' not in cls_name:
        continue
    print(f"Class: {cls_name.replace('/', '.').strip('L;')}")
    for m in vm_class.get_methods():
        print(f"  {m.get_name()}{m.get_descriptor()}")

# ─── 4. Find classes with command byte constants (short arrays with 0xAA) ────

print("\n\n=== Classes with new-array byte[] in baseus/bluetrum packages ===\n")
for class_analysis in dx.get_classes():
    vm_class = class_analysis.get_vm_class()
    cls_name = vm_class.get_name()
    if not any(p in cls_name for p in ['baseus', 'bluetrum', 'control_center', 'ccsdk']):
        continue

    for m in vm_class.get_methods():
        code = m.get_code()
        if code is None:
            continue
        bc = code.get_bc()
        insns = list(bc.get_instructions())

        # Find const-16 or const/4 or const with 0xAA(-86) near new-array
        has_aa = False
        for i, insn in enumerate(insns):
            if insn.get_name() not in ('const/4', 'const/16', 'const', 'const-wide'):
                continue
            try:
                ops = insn.get_operands()
                for op in ops:
                    if isinstance(op, tuple) and len(op) >= 2:
                        v = op[-1]
                        if isinstance(v, int) and (v == 0xAA or v == -86 or v == 170):
                            has_aa = True
                            break
            except Exception:
                pass

        if has_aa:
            pretty_cls = cls_name.replace('/', '.').strip('L;')
            print(f"  {pretty_cls}.{m.get_name()}{m.get_descriptor()}")

print("\n[*] Done.")
