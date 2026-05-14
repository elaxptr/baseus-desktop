/**
 * dump-all.js — Full runtime extraction of Baseus protocol
 *
 * Runs against the Baseus app on the MuMuPlayer emulator via Frida.
 * Earbuds do NOT need to be connected — we extract from the app's own
 * class structure, static constants, and method return values at runtime.
 *
 * What this captures:
 *   1. All classes related to BLE/protocol/commands (deobfuscated names)
 *   2. All static fields on those classes (opcode constants, enums)
 *   3. All methods that return byte[] (packet builders) — hooks them to
 *      capture every byte array they produce when UI triggers them
 *   4. BLE GATT TX/RX in case a connection does happen
 *   5. Everything saved to /sdcard/baseus_dump.log
 *
 * Usage:
 *   frida -D 127.0.0.1:16384 -f com.baseus.intelligent -l docs/frida/dump-all.js --no-pause
 *
 * Then open the app and tap every button/toggle you can find.
 * Pull log: adb -s 127.0.0.1:16384 pull /sdcard/baseus_dump.log
 */

'use strict';

// ─── File output ───────────────────────────────────────────────────────────

var fw = null;

function initLog() {
    try {
        var FileWriter = Java.use('java.io.FileWriter');
        fw = FileWriter.$new('/data/local/tmp/baseus_dump.log', false);
        fw.write('=== baseus dump-all ' + new Date().toISOString() + ' ===\n\n');
        fw.flush();
        console.log('[*] logging to /data/local/tmp/baseus_dump.log');
    } catch(e) { console.log('file output failed: ' + e); }
}

function out(line) {
    console.log(line);
    if (fw) { try { fw.write(line + '\n'); fw.flush(); } catch(e) {} }
}

// ─── Helpers ───────────────────────────────────────────────────────────────

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

function decodeFrame(bytes) {
    if (!bytes || bytes.length < 2) return '';
    var b = bytes instanceof Array ? bytes : Array.from(bytes);
    if ((b[0] & 0xff) !== 0xAA) return '';
    return 'cmd=0x' + ((b[1] & 0xff).toString(16).toUpperCase().padStart(2,'0')) +
        (b.length > 2 ? ' payload=[' + hex(b.slice(2)) + ']' : '');
}

function appStack() {
    try {
        var lines = Java.use('android.util.Log')
            .getStackTraceString(Java.use('java.lang.Exception').$new())
            .split('\n');
        return lines.filter(function(l) {
            l = l.trim();
            return l.startsWith('at ') &&
                !l.startsWith('at android.') && !l.startsWith('at java.') &&
                !l.startsWith('at kotlin.') && !l.startsWith('at com.android.') &&
                !l.startsWith('at dalvik.') && !l.startsWith('at sun.');
        }).slice(0, 8).join('\n        ');
    } catch(e) { return ''; }
}

function dumpFields(clazz, className) {
    try {
        var fields = clazz.class.getDeclaredFields();
        for (var i = 0; i < fields.length; i++) {
            try {
                var f = fields[i];
                f.setAccessible(true);
                var mods = f.getModifiers();
                var isStatic = (mods & 8) !== 0;
                if (!isStatic) continue;
                var val = f.get(null);
                var valStr = val === null ? 'null' : String(val);
                // For byte arrays show hex
                if (val !== null && val.getClass && val.getClass().getName() === '[B') {
                    valStr = 'byte[] ' + hex(Array.from(val));
                }
                out('  FIELD ' + f.getName() + ' : ' + f.getType().getName() + ' = ' + valStr);
            } catch(e) {}
        }
    } catch(e) {}
}

function dumpMethods(clazz, className) {
    try {
        var methods = clazz.class.getDeclaredMethods();
        for (var i = 0; i < methods.length; i++) {
            try {
                var m = methods[i];
                var params = Java.cast(m.getParameterTypes(), Java.use('java.lang.Object[]'));
                var paramNames = [];
                for (var j = 0; j < m.getParameterTypes().length; j++) {
                    paramNames.push(m.getParameterTypes()[j].getName());
                }
                out('  METHOD ' + m.getName() + '(' + paramNames.join(', ') + ') -> ' + m.getReturnType().getName());
            } catch(e) {}
        }
    } catch(e) {}
}

// ─── Main ──────────────────────────────────────────────────────────────────

Java.perform(function() {
    initLog();
    out('[*] Frida attached, extracting Baseus protocol...\n');

    // ── 1. Enumerate all loaded classes, filter for interesting ones ───────

    // Only look at app + CCSDK classes, not Android framework
    var APP_PREFIXES = [
        'com.baseus.',
        'com.bluetrum.',
        'com.control_center.intelligent.',
        'com.jieli.',
        'com.ccsdk.',
    ];

    function isAppClass(name) {
        for (var i = 0; i < APP_PREFIXES.length; i++) {
            if (name.indexOf(APP_PREFIXES[i]) === 0) return true;
        }
        return false;
    }

    var found = {};

    out('=== PHASE 1: App class enumeration ===');
    Java.enumerateLoadedClasses({
        onMatch: function(name) {
            if (isAppClass(name) && !found[name]) {
                found[name] = true;
                out('CLASS ' + name);
            }
        },
        onComplete: function() {
            out('\n=== PHASE 2: Static field dump per class ===');

            Object.keys(found).forEach(function(className) {
                try {
                    var clazz = Java.use(className);
                    out('\n-- ' + className + ' --');
                    dumpFields(clazz, className);
                    dumpMethods(clazz, className);
                } catch(e) {}
            });

            out('\n=== PHASE 3: Hook all byte[] returning methods ===');

            // Hook every method in found classes that returns byte[]
            Object.keys(found).forEach(function(className) {
                try {
                    var clazz = Java.use(className);
                    var methods = clazz.class.getDeclaredMethods();
                    for (var i = 0; i < methods.length; i++) {
                        try {
                            var m = methods[i];
                            if (m.getReturnType().getName() !== '[B') continue;
                            var methodName = m.getName();
                            // Hook this overload
                            (function(cn, mn) {
                                try {
                                    clazz[mn].implementation = function() {
                                        var result = this[mn].apply(this, arguments);
                                        if (result && result.length > 0) {
                                            var bytes = Array.from(result);
                                            out('[' + ts() + '] PACKET-BUILD ' + cn + '.' + mn + '()');
                                            out('    HEX: ' + hex(bytes));
                                            out('    DEC: ' + decodeFrame(bytes));
                                            out('    STACK: ' + appStack());
                                        }
                                        return result;
                                    };
                                } catch(e) {}
                            })(className, methodName);
                        } catch(e) {}
                    }
                } catch(e) {}
            });

            out('[*] byte[] hooks installed\n');
        }
    });

    // ── 2. BLE GATT hooks (TX/RX if a connection does happen) ─────────────

    out('=== PHASE 4: BLE GATT TX/RX hooks ===');

    var BluetoothGatt = Java.use('android.bluetooth.BluetoothGatt');

    // Legacy writeCharacteristic
    try {
        BluetoothGatt.writeCharacteristic
            .overload('android.bluetooth.BluetoothGattCharacteristic')
            .implementation = function(ch) {
                var val = Array.from(ch.getValue() || []);
                out('[' + ts() + '] GATT-TX uuid=' + ch.getUuid());
                out('    HEX: ' + hex(val));
                out('    DEC: ' + decodeFrame(val));
                out('    STACK: ' + appStack());
                return this.writeCharacteristic(ch);
            };
    } catch(e) {}

    // API 33+ writeCharacteristic
    try {
        BluetoothGatt.writeCharacteristic
            .overload('android.bluetooth.BluetoothGattCharacteristic', '[B', 'int')
            .implementation = function(ch, value, writeType) {
                var val = Array.from(value || []);
                out('[' + ts() + '] GATT-TX uuid=' + ch.getUuid());
                out('    HEX: ' + hex(val));
                out('    DEC: ' + decodeFrame(val));
                out('    STACK: ' + appStack());
                return this.writeCharacteristic(ch, value, writeType);
            };
    } catch(e) {}

    // RX notification (legacy)
    try {
        var GattCb = Java.use('android.bluetooth.BluetoothGattCallback');
        GattCb.onCharacteristicChanged
            .overload('android.bluetooth.BluetoothGatt', 'android.bluetooth.BluetoothGattCharacteristic')
            .implementation = function(gatt, ch) {
                var val = Array.from(ch.getValue() || []);
                out('[' + ts() + '] GATT-RX uuid=' + ch.getUuid());
                out('    HEX: ' + hex(val));
                out('    DEC: ' + decodeFrame(val));
                return this.onCharacteristicChanged(gatt, ch);
            };
    } catch(e) {}

    // RX notification (API 33+)
    try {
        var GattCb2 = Java.use('android.bluetooth.BluetoothGattCallback');
        GattCb2.onCharacteristicChanged
            .overload('android.bluetooth.BluetoothGatt', 'android.bluetooth.BluetoothGattCharacteristic', '[B')
            .implementation = function(gatt, ch, value) {
                var val = Array.from(value || []);
                out('[' + ts() + '] GATT-RX uuid=' + ch.getUuid());
                out('    HEX: ' + hex(val));
                out('    DEC: ' + decodeFrame(val));
                return this.onCharacteristicChanged(gatt, ch, value);
            };
    } catch(e) {}

    // Service discovery — dump full GATT map
    try {
        Java.use('android.bluetooth.BluetoothGattCallback')
            .onServicesDiscovered
            .overload('android.bluetooth.BluetoothGatt', 'int')
            .implementation = function(gatt, status) {
                out('[' + ts() + '] GATT SERVICES (status=' + status + ')');
                var svcs = gatt.getServices();
                for (var i = 0; i < svcs.size(); i++) {
                    var svc = svcs.get(i);
                    out('  SVC ' + svc.getUuid());
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
                        out('    CHAR ' + c.getUuid() + ' [' + props.join('|') + ']');
                    }
                }
                return this.onServicesDiscovered(gatt, status);
            };
    } catch(e) {}

    // ── 3. Hook SharedPreferences to capture all stored settings ──────────

    try {
        var SPImpl = Java.use('android.app.SharedPreferencesImpl');
        ['getString','getInt','getLong','getFloat','getBoolean'].forEach(function(method) {
            try {
                var overloads = SPImpl[method].overloads;
                overloads.forEach(function(ol) {
                    ol.implementation = function() {
                        var val = ol.apply(this, arguments);
                        out('[PREF] ' + method + '("' + arguments[0] + '") = ' + val);
                        return val;
                    };
                });
            } catch(e) {}
        });
    } catch(e) {}

    // ── 4. Enumerate native .so exports ───────────────────────────────────

    out('\n=== PHASE 5: Native library exports ===');
    Process.enumerateModules().forEach(function(mod) {
        var n = mod.name.toLowerCase();
        if (n.indexOf('ccsdk') >= 0 || n.indexOf('bluetrum') >= 0 ||
            n.indexOf('baseus') >= 0 || n.indexOf('jieli') >= 0 ||
            n.indexOf('bt') >= 0 || n.indexOf('ble') >= 0) {
            out('NATIVE MODULE: ' + mod.name + ' @ ' + mod.base);
            try {
                mod.enumerateExports().forEach(function(exp) {
                    out('  EXPORT: ' + exp.name + ' @ ' + exp.address + ' type=' + exp.type);
                });
            } catch(e) {}
        }
    });

    out('\n[*] All hooks active. Now open the Baseus app and tap EVERY button you see.');
    out('[*] ANC, EQ presets, gestures, battery, wear detection, game mode — everything.');
    out('[*] Pull log when done: adb -s 127.0.0.1:16384 pull /data/local/tmp/baseus_dump.log');
});
