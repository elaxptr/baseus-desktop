/**
 * protocol-dump.js — Targeted Baseus protocol extraction
 *
 * Attach to the already-running app (past login):
 *   frida -D 127.0.0.1:16384 -F -l docs/frida/protocol-dump.js
 *
 * This script:
 *   1. Dumps all methods + static fields of the 4 CCSDK protocol classes
 *      (c4, b4, n3, o3) — the obfuscated packet-builder layer
 *   2. Hooks every byte[] returning method in those classes to capture live packets
 *   3. Hooks HomeBleDataResolvePresenter to see parsed incoming data
 *   4. Hooks BLE GATT TX/RX (both legacy and API 33+)
 *   5. Hooks ClassicBT send/receive if present
 *
 * Then in the app: tap EVERY button — ANC, EQ, gestures, battery, etc.
 */

'use strict';

// ── helpers ────────────────────────────────────────────────────────────────

function hex(bytes) {
    if (!bytes) return '(null)';
    var arr = bytes instanceof Array ? bytes : Array.from(bytes);
    if (arr.length === 0) return '(empty)';
    return arr.map(function(b) {
        b = b & 0xff;
        return (b < 16 ? '0' : '') + b.toString(16).toUpperCase();
    }).join(' ');
}

function ts() { return new Date().toISOString().slice(11, 23); }

function decodeFrame(arr) {
    if (!arr || arr.length < 2) return '';
    if ((arr[0] & 0xff) !== 0xAA) return '(not AA frame)';
    var cmd = (arr[1] & 0xff).toString(16).toUpperCase().padStart(2, '0');
    var payload = arr.length > 2 ? ' payload=[' + hex(arr.slice(2)) + ']' : '';
    return 'cmd=0x' + cmd + payload;
}

function shortStack() {
    try {
        var lines = Java.use('android.util.Log')
            .getStackTraceString(Java.use('java.lang.Exception').$new())
            .split('\n');
        return lines.filter(function(l) {
            l = l.trim();
            return l.startsWith('at ') &&
                !l.startsWith('at android.') && !l.startsWith('at java.') &&
                !l.startsWith('at kotlin.') && !l.startsWith('at com.android.');
        }).slice(0, 5).join('\n        ');
    } catch (e) { return ''; }
}

// ── dump a class's methods and static fields (no Java.use inside loops) ────

function dumpClass(className) {
    try {
        var clazz = Java.use(className);
        console.log('\n╔══ CLASS: ' + className + ' ══╗');

        // Static fields
        var fields = clazz.class.getDeclaredFields();
        for (var i = 0; i < fields.length; i++) {
            try {
                var f = fields[i];
                f.setAccessible(true);
                var mods = f.getModifiers();
                var isStatic = (mods & 8) !== 0;
                if (!isStatic) continue;
                var val = f.get(null);
                var valStr;
                if (val === null) {
                    valStr = 'null';
                } else {
                    try {
                        var cn = val.getClass().getName();
                        if (cn === '[B') {
                            valStr = 'byte[] ' + hex(Array.from(val));
                        } else {
                            valStr = String(val);
                        }
                    } catch (e2) { valStr = String(val); }
                }
                console.log('  STATIC_FIELD ' + f.getName() + ' : ' + f.getType().getName() + ' = ' + valStr);
            } catch (e) {}
        }

        // All methods
        var methods = clazz.class.getDeclaredMethods();
        for (var j = 0; j < methods.length; j++) {
            try {
                var m = methods[j];
                var ptypes = m.getParameterTypes();
                var paramNames = [];
                for (var k = 0; k < ptypes.length; k++) {
                    paramNames.push(ptypes[k].getName());
                }
                var mods2 = m.getModifiers();
                var isStatic2 = (mods2 & 8) !== 0 ? 'static ' : '';
                console.log('  METHOD ' + isStatic2 + m.getName() +
                    '(' + paramNames.join(', ') + ') -> ' + m.getReturnType().getName());
            } catch (e) {}
        }
    } catch (e) {
        console.log('  [!] Could not load ' + className + ': ' + e);
    }
}

// ── hook all byte[] returning methods in a class ────────────────────────────

function hookByteArrayMethods(className) {
    try {
        var clazz = Java.use(className);
        var methods = clazz.class.getDeclaredMethods();
        var hooked = 0;
        for (var i = 0; i < methods.length; i++) {
            try {
                var m = methods[i];
                if (m.getReturnType().getName() !== '[B') continue;
                var methodName = m.getName();
                (function(cn, mn) {
                    try {
                        clazz[mn].implementation = function() {
                            var result = this[mn].apply(this, arguments);
                            if (result && result.length > 0) {
                                var bytes = Array.from(result);
                                console.log('[' + ts() + '] BYTE[] ' + cn + '.' + mn + '()');
                                console.log('    HEX: ' + hex(bytes));
                                console.log('    DEC: ' + decodeFrame(bytes));
                                console.log('    STACK: ' + shortStack());
                            }
                            return result;
                        };
                        hooked++;
                    } catch (e) {}
                })(className, methodName);
            } catch (e) {}
        }
        if (hooked > 0) console.log('  [hook] ' + className + ': hooked ' + hooked + ' byte[] method(s)');
    } catch (e) {
        console.log('  [!] hookByteArrayMethods failed for ' + className + ': ' + e);
    }
}

// ── hook all methods of a class (log every call + args) ────────────────────

function hookAllMethods(className) {
    try {
        var clazz = Java.use(className);
        var methods = clazz.class.getDeclaredMethods();
        var hooked = 0;
        for (var i = 0; i < methods.length; i++) {
            try {
                var m = methods[i];
                var mn = m.getName();
                var retType = m.getReturnType().getName();
                (function(cn, name, ret) {
                    try {
                        clazz[name].implementation = function() {
                            var argStrs = [];
                            for (var a = 0; a < arguments.length; a++) {
                                try {
                                    var arg = arguments[a];
                                    if (arg === null || arg === undefined) {
                                        argStrs.push('null');
                                    } else if (arg.getClass && arg.getClass().getName() === '[B') {
                                        argStrs.push('byte[' + arg.length + ']=' + hex(Array.from(arg)));
                                    } else {
                                        argStrs.push(String(arg));
                                    }
                                } catch (e) { argStrs.push('?'); }
                            }
                            var result = this[name].apply(this, arguments);
                            var retStr = '';
                            if (ret === '[B' && result) {
                                retStr = ' => byte[]=' + hex(Array.from(result));
                            } else if (result !== undefined && result !== null) {
                                try { retStr = ' => ' + String(result); } catch (e) {}
                            }
                            console.log('[' + ts() + '] CALL ' + cn + '.' + name +
                                '(' + argStrs.join(', ') + ')' + retStr);
                            return result;
                        };
                        hooked++;
                    } catch (e) {}
                })(className, mn, retType);
            } catch (e) {}
        }
        if (hooked > 0) console.log('  [hook] ' + className + ': hooked ' + hooked + ' method(s)');
    } catch (e) {
        console.log('  [!] hookAllMethods failed for ' + className + ': ' + e);
    }
}

// ── main ───────────────────────────────────────────────────────────────────

Java.perform(function() {
    console.log('\n[*] protocol-dump.js attached at ' + new Date().toISOString());

    // ── 1. Dump + hook CCSDK protocol classes ──────────────────────────────
    var CCSDK_CLASSES = [
        'com.bluetrum.ccsdk.c4',
        'com.bluetrum.ccsdk.b4',
        'com.bluetrum.ccsdk.n3',
        'com.bluetrum.ccsdk.o3',
    ];

    console.log('\n=== PHASE 1: CCSDK class dump ===');
    CCSDK_CLASSES.forEach(function(cn) { dumpClass(cn); });

    console.log('\n=== PHASE 2: Hook CCSDK byte[] methods ===');
    CCSDK_CLASSES.forEach(function(cn) { hookByteArrayMethods(cn); });

    // ── 2. Hook HomeBleDataResolvePresenter (incoming data parser) ─────────
    console.log('\n=== PHASE 3: Hook HomeBleDataResolvePresenter ===');
    hookAllMethods('com.control_center.intelligent.view.presenter.HomeBleDataResolvePresenter');

    // ── 3. Hook BleEnhancedApi / BleApi (outgoing commands) ───────────────
    console.log('\n=== PHASE 4: Hook BLE API layer ===');
    hookByteArrayMethods('com.baseus.ble.api.BleEnhancedApi');
    hookByteArrayMethods('com.baseus.ble.api.BleApi');
    hookAllMethods('com.baseus.ble.manager.BleManager');

    // ── 4. BLE GATT hooks (TX/RX at Android framework level) ──────────────
    console.log('\n=== PHASE 5: BLE GATT TX/RX ===');
    var BluetoothGatt = Java.use('android.bluetooth.BluetoothGatt');

    try {
        BluetoothGatt.writeCharacteristic
            .overload('android.bluetooth.BluetoothGattCharacteristic')
            .implementation = function(ch) {
                var val = Array.from(ch.getValue() || []);
                console.log('[' + ts() + '] GATT-TX uuid=' + ch.getUuid());
                console.log('    HEX: ' + hex(val));
                console.log('    DEC: ' + decodeFrame(val));
                console.log('    STACK: ' + shortStack());
                return this.writeCharacteristic(ch);
            };
        console.log('  [hook] GATT writeCharacteristic (legacy)');
    } catch (e) { console.log('  [skip] GATT writeCharacteristic legacy: ' + e); }

    try {
        BluetoothGatt.writeCharacteristic
            .overload('android.bluetooth.BluetoothGattCharacteristic', '[B', 'int')
            .implementation = function(ch, value, writeType) {
                var val = Array.from(value || []);
                console.log('[' + ts() + '] GATT-TX uuid=' + ch.getUuid());
                console.log('    HEX: ' + hex(val));
                console.log('    DEC: ' + decodeFrame(val));
                console.log('    STACK: ' + shortStack());
                return this.writeCharacteristic(ch, value, writeType);
            };
        console.log('  [hook] GATT writeCharacteristic (API33+)');
    } catch (e) { console.log('  [skip] GATT writeCharacteristic API33+: ' + e); }

    // RX: onCharacteristicChanged — catch incoming notifications
    try {
        Java.use('android.bluetooth.BluetoothGattCallback')
            .onCharacteristicChanged
            .overload('android.bluetooth.BluetoothGatt', 'android.bluetooth.BluetoothGattCharacteristic')
            .implementation = function(gatt, ch) {
                var val = Array.from(ch.getValue() || []);
                console.log('[' + ts() + '] GATT-RX uuid=' + ch.getUuid());
                console.log('    HEX: ' + hex(val));
                console.log('    DEC: ' + decodeFrame(val));
                return this.onCharacteristicChanged(gatt, ch);
            };
        console.log('  [hook] GATT onCharacteristicChanged (legacy)');
    } catch (e) { console.log('  [skip] GATT onCharacteristicChanged legacy: ' + e); }

    try {
        Java.use('android.bluetooth.BluetoothGattCallback')
            .onCharacteristicChanged
            .overload('android.bluetooth.BluetoothGatt', 'android.bluetooth.BluetoothGattCharacteristic', '[B')
            .implementation = function(gatt, ch, value) {
                var val = Array.from(value || []);
                console.log('[' + ts() + '] GATT-RX uuid=' + ch.getUuid());
                console.log('    HEX: ' + hex(val));
                console.log('    DEC: ' + decodeFrame(val));
                return this.onCharacteristicChanged(gatt, ch, value);
            };
        console.log('  [hook] GATT onCharacteristicChanged (API33+)');
    } catch (e) { console.log('  [skip] GATT onCharacteristicChanged API33+: ' + e); }

    // Service discovery
    try {
        Java.use('android.bluetooth.BluetoothGattCallback')
            .onServicesDiscovered
            .overload('android.bluetooth.BluetoothGatt', 'int')
            .implementation = function(gatt, status) {
                console.log('[' + ts() + '] GATT SERVICES (status=' + status + ')');
                var svcs = gatt.getServices();
                for (var i = 0; i < svcs.size(); i++) {
                    var svc = svcs.get(i);
                    console.log('  SVC ' + svc.getUuid());
                    var chars = svc.getCharacteristics();
                    for (var j = 0; j < chars.size(); j++) {
                        var c = chars.get(j);
                        var p = c.getProperties();
                        var props = [];
                        if (p & 0x02) props.push('READ');
                        if (p & 0x04) props.push('WRITE_NO_RSP');
                        if (p & 0x08) props.push('WRITE');
                        if (p & 0x10) props.push('NOTIFY');
                        if (p & 0x20) props.push('INDICATE');
                        console.log('    CHAR ' + c.getUuid() + ' [' + props.join('|') + ']');
                    }
                }
                return this.onServicesDiscovered(gatt, status);
            };
        console.log('  [hook] GATT onServicesDiscovered');
    } catch (e) { console.log('  [skip] GATT onServicesDiscovered: ' + e); }

    // ── 5. Classic BT send hook ────────────────────────────────────────────
    console.log('\n=== PHASE 6: Classic BT hooks ===');
    try {
        var ClassicBtApi = Java.use('com.baseus.classicbluetoothsdk.api.ClassicBtApi');
        hookByteArrayMethods('com.baseus.classicbluetoothsdk.api.ClassicBtApi');
        hookAllMethods('com.baseus.classicbluetoothsdk.bluetooth.presenter.ClassicBluetoothPresenter');
        console.log('  [hook] ClassicBtApi done');
    } catch (e) { console.log('  [skip] ClassicBtApi: ' + e); }

    console.log('\n[*] All hooks installed.');
    console.log('[*] NOW: navigate to your earphone in the app and tap ANC / EQ / gestures / battery.');
    console.log('[*] Watch this output for BYTE[] and GATT-TX lines.');
});
