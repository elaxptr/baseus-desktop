/**
 * eq-gesture-dump.js — Find EQ preset and gesture config command bytes
 *
 * Strategy:
 *   1. Enumerate all loaded classes matching eq/gesture/touch key keywords
 *   2. Dump static byte[] fields from those classes (contain opcodes)
 *   3. Hook all GATT TX writes — when app sends a command, log it with stack
 *   4. Tap EQ presets + gesture dropdowns in the app; TX lines will appear
 *
 * Usage:
 *   frida -U -F -l docs/frida/eq-gesture-dump.js
 */

'use strict';

function hex(bytes) {
    if (!bytes) return '(null)';
    var arr = Array.from(bytes);
    return arr.map(function(b) { return ((b & 0xff) < 16 ? '0' : '') + (b & 0xff).toString(16).toUpperCase(); }).join(' ');
}

function ts() { return new Date().toISOString().slice(11, 23); }

function shortStack() {
    try {
        var lines = Java.use('android.util.Log')
            .getStackTraceString(Java.use('java.lang.Exception').$new())
            .split('\n');
        return lines.filter(function(l) {
            l = l.trim();
            return l.startsWith('at ') &&
                !l.startsWith('at android.') && !l.startsWith('at java.') &&
                !l.startsWith('at kotlin.') && !l.startsWith('at com.android.') &&
                !l.startsWith('at dalvik.');
        }).slice(0, 6).join('\n    ');
    } catch (e) { return '(stack unavailable)'; }
}

function dumpStaticByteFields(className) {
    try {
        var clazz = Java.use(className);
        var fields = clazz.class.getDeclaredFields();
        var found = 0;
        for (var i = 0; i < fields.length; i++) {
            try {
                var f = fields[i];
                f.setAccessible(true);
                var mods = f.getModifiers();
                if ((mods & 8) === 0) continue; // static only
                var val = f.get(null);
                if (val === null) continue;
                var cn = val.getClass().getName();
                if (cn === '[B') {
                    var bytes = Array.from(val);
                    if (bytes.length > 0 && bytes.length <= 16) {
                        console.log('  STATIC_BYTE[] ' + f.getName() + ' = ' + hex(bytes));
                        found++;
                    }
                } else if (cn === 'java.lang.Integer' || cn === 'java.lang.Byte') {
                    var v = parseInt(String(val));
                    if (v > 0) console.log('  STATIC_INT ' + f.getName() + ' = 0x' + v.toString(16).toUpperCase() + ' (' + v + ')');
                    found++;
                }
            } catch (e) {}
        }
        return found;
    } catch (e) {
        return 0;
    }
}

Java.perform(function() {
    console.log('\n[*] eq-gesture-dump.js attached — ' + new Date().toISOString());

    // ── 1. Enumerate classes matching EQ/gesture keywords ─────────────────
    var KEYWORDS = ['eq', 'equal', 'sound', 'preset', 'gesture', 'touch', 'key', 'shortcut', 'action'];
    var matched = {};

    console.log('\n=== Phase 1: Scanning loaded classes for EQ/gesture candidates ===');
    Java.enumerateLoadedClasses({
        onMatch: function(name) {
            var lname = name.toLowerCase();
            if (lname.indexOf('baseus') < 0 && lname.indexOf('bluetrum') < 0 && lname.indexOf('ccsdk') < 0) return;
            for (var i = 0; i < KEYWORDS.length; i++) {
                if (lname.indexOf(KEYWORDS[i]) >= 0) {
                    if (!matched[name]) {
                        matched[name] = true;
                        console.log('  MATCH: ' + name);
                    }
                    break;
                }
            }
        },
        onComplete: function() {
            console.log('  (scan complete)');
        }
    });

    // ── 2. Dump static byte[] fields from matched classes ─────────────────
    console.log('\n=== Phase 2: Static byte[] fields from matched classes ===');
    Object.keys(matched).forEach(function(cn) {
        var n = dumpStaticByteFields(cn);
        if (n > 0) console.log('  ^ from class: ' + cn);
    });

    // ── 3. Also dump known CCSDK protocol builder classes ─────────────────
    console.log('\n=== Phase 3: CCSDK protocol class static fields ===');
    var CCSDK = [
        'com.bluetrum.ccsdk.c4', 'com.bluetrum.ccsdk.b4',
        'com.bluetrum.ccsdk.n3', 'com.bluetrum.ccsdk.o3',
        'com.bluetrum.ccsdk.a4', 'com.bluetrum.ccsdk.d4',
        'com.bluetrum.ccsdk.e4', 'com.bluetrum.ccsdk.f4',
        'com.bluetrum.ccsdk.g4', 'com.bluetrum.ccsdk.h4',
        'com.bluetrum.ccsdk.i4', 'com.bluetrum.ccsdk.j4',
    ];
    CCSDK.forEach(function(cn) {
        var n = dumpStaticByteFields(cn);
        if (n > 0) console.log('  ^ ' + cn);
    });

    // ── 4. Hook ALL GATT TX writes — log every byte sequence sent ─────────
    console.log('\n=== Phase 4: GATT TX hook armed — tap EQ + gestures in app now ===');
    var BluetoothGatt = Java.use('android.bluetooth.BluetoothGatt');

    // Legacy API (pre-33)
    try {
        BluetoothGatt.writeCharacteristic
            .overload('android.bluetooth.BluetoothGattCharacteristic')
            .implementation = function(ch) {
                var val = Array.from(ch.getValue() || []);
                if (val.length > 0) {
                    console.log('\n[' + ts() + '] >>>TX uuid=' + ch.getUuid());
                    console.log('    HEX: ' + hex(val));
                    console.log('    STACK:\n    ' + shortStack());
                }
                return this.writeCharacteristic(ch);
            };
        console.log('  [hook] writeCharacteristic (legacy API)');
    } catch (e) { console.log('  [skip legacy] ' + e); }

    // API 33+
    try {
        BluetoothGatt.writeCharacteristic
            .overload('android.bluetooth.BluetoothGattCharacteristic', '[B', 'int')
            .implementation = function(ch, value, writeType) {
                var val = Array.from(value || []);
                if (val.length > 0) {
                    console.log('\n[' + ts() + '] >>>TX uuid=' + ch.getUuid());
                    console.log('    HEX: ' + hex(val));
                    console.log('    STACK:\n    ' + shortStack());
                }
                return this.writeCharacteristic(ch, value, writeType);
            };
        console.log('  [hook] writeCharacteristic (API 33+)');
    } catch (e) { console.log('  [skip api33] ' + e); }

    // ── 5. Hook classes that look like EQ/gesture presenters ──────────────
    console.log('\n=== Phase 5: Hooking matched class methods for live capture ===');
    Object.keys(matched).forEach(function(cn) {
        try {
            var clazz = Java.use(cn);
            var methods = clazz.class.getDeclaredMethods();
            var hooked = 0;
            for (var i = 0; i < methods.length; i++) {
                try {
                    var m = methods[i];
                    var mn = m.getName();
                    var ptypes = m.getParameterTypes();
                    // Only hook void or byte[] returning methods with 0-2 args
                    var ret = m.getReturnType().getName();
                    if (ret !== 'void' && ret !== '[B') continue;
                    if (ptypes.length > 3) continue;
                    (function(klass, name, retType) {
                        try {
                            klass[name].implementation = function() {
                                var argStrs = [];
                                for (var a = 0; a < arguments.length; a++) {
                                    try {
                                        var arg = arguments[a];
                                        if (arg === null || arg === undefined) argStrs.push('null');
                                        else if (arg.getClass && arg.getClass().getName() === '[B')
                                            argStrs.push('bytes=' + hex(Array.from(arg)));
                                        else argStrs.push(String(arg).slice(0, 30));
                                    } catch(e) { argStrs.push('?'); }
                                }
                                var result = this[name].apply(this, arguments);
                                var retStr = '';
                                if (retType === '[B' && result) retStr = ' => ' + hex(Array.from(result));
                                if (argStrs.length > 0 || retStr) {
                                    console.log('[' + ts() + '] CALL ' + klass.class.getName() + '.' + name +
                                        '(' + argStrs.join(', ') + ')' + retStr);
                                }
                                return result;
                            };
                            hooked++;
                        } catch(e) {}
                    })(clazz, mn, ret);
                } catch(e) {}
            }
            if (hooked > 0) console.log('  [hook] ' + cn + ' (' + hooked + ' methods)');
        } catch(e) {}
    });

    console.log('\n[*] Ready. Now in the Baseus app:');
    console.log('    1. Navigate to EQ screen, tap each preset (Balanced / Bass / Voice / Clear)');
    console.log('    2. Navigate to Gesture screen, change each action once');
    console.log('    3. Watch TX lines above for the byte sequences\n');
});
