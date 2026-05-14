/**
 * eq-static-extract.js — Extract EQ preset + gesture command bytes WITHOUT device connection
 *
 * Strategy:
 *   1. Dump all fields/methods of EQ data model classes
 *   2. Enumerate QtConstant and sub-classes for opcode constants
 *   3. Scan ALL CCSDK classes (b1..z9 obfuscated) for setEq/setGesture patterns
 *   4. Hook GATT TX for when we force-invoke the EQ setter
 *   5. Try to directly invoke the EQ command builder and log bytes
 *
 * Usage:
 *   frida -U -n Baseus -l docs/frida/eq-static-extract.js
 */

'use strict';

function hex(bytes) {
    if (!bytes) return '(null)';
    var arr = Array.from(bytes);
    return arr.map(function(b) { return ((b & 0xff) < 16 ? '0' : '') + (b & 0xff).toString(16).toUpperCase(); }).join(' ');
}

function describeClass(className) {
    try {
        var clazz = Java.use(className);
        var fields = clazz.class.getDeclaredFields();
        var methods = clazz.class.getDeclaredMethods();
        console.log('\n[CLASS] ' + className);
        for (var i = 0; i < fields.length; i++) {
            try {
                var f = fields[i];
                f.setAccessible(true);
                var mods = f.getModifiers();
                var modStr = ((mods & 1) ? 'pub ' : '') + ((mods & 2) ? 'pri ' : '') + ((mods & 8) ? 'sta ' : '');
                var typeName = f.getType().getName();
                var valStr = '';
                if ((mods & 8) !== 0) {
                    try {
                        var v = f.get(null);
                        if (v !== null) {
                            var cn = v.getClass().getName();
                            if (cn === '[B') valStr = ' = ' + hex(Array.from(v));
                            else valStr = ' = ' + String(v).slice(0, 60);
                        }
                    } catch(e) {}
                }
                console.log('  FIELD ' + modStr + typeName + ' ' + f.getName() + valStr);
            } catch(e) {}
        }
        for (var i = 0; i < methods.length; i++) {
            try {
                var m = methods[i];
                var ptypes = m.getParameterTypes();
                var params = Array.from(ptypes).map(function(p) { return p.getName(); }).join(', ');
                console.log('  METHOD ' + m.getReturnType().getName() + ' ' + m.getName() + '(' + params + ')');
            } catch(e) {}
        }
    } catch(e) {
        console.log('  [error loading ' + className + ']: ' + e);
    }
}

Java.perform(function() {
    console.log('\n[*] eq-static-extract.js attached — ' + new Date().toISOString());

    // ── 1. Describe EQ model classes ──────────────────────────────────────
    console.log('\n=== EQ Model Class Inspection ===');
    [
        'com.baseus.model.control.EqSimpleData',
        'com.baseus.model.control.EqRelatedValueBean',
        'com.baseus.model.control.EqUploadRequestBean',
        'com.baseus.model.control.ResRequest',
    ].forEach(describeClass);

    // ── 2. Enumerate QtConstant hierarchy ────────────────────────────────
    console.log('\n=== QtConstant hierarchy ===');
    Java.enumerateLoadedClasses({
        onMatch: function(name) {
            if (name.indexOf('QtConstant') >= 0 || name.indexOf('qt.Constant') >= 0 ||
                name.indexOf('QtProperty') >= 0 || name.indexOf('CcsdkConstant') >= 0) {
                describeClass(name);
            }
        },
        onComplete: function() { console.log('  (QtConstant scan done)'); }
    });

    // ── 3. Scan ALL loaded CCSDK classes for EQ/gesture-relevant methods ──
    console.log('\n=== CCSDK class scan for setEq/setGesture patterns ===');
    Java.enumerateLoadedClasses({
        onMatch: function(name) {
            if (name.indexOf('bluetrum') < 0 && name.indexOf('ccsdk') < 0) return;
            try {
                var clazz = Java.use(name);
                var methods = clazz.class.getDeclaredMethods();
                for (var i = 0; i < methods.length; i++) {
                    var mn = methods[i].getName().toLowerCase();
                    if (mn.indexOf('eq') >= 0 || mn.indexOf('equal') >= 0 ||
                        mn.indexOf('gesture') >= 0 || mn.indexOf('touch') >= 0 ||
                        mn.indexOf('shortcut') >= 0 || mn.indexOf('preset') >= 0 ||
                        mn.indexOf('sound') >= 0 || mn.indexOf('key') >= 0) {
                        console.log('  [MATCH] ' + name + '.' + methods[i].getName() +
                            ' (' + Array.from(methods[i].getParameterTypes()).map(function(p){return p.getName();}).join(',') + ')' +
                            ' -> ' + methods[i].getReturnType().getName());
                    }
                }
                // Also log any class that has a method returning byte[]
                for (var i = 0; i < methods.length; i++) {
                    if (methods[i].getReturnType().getName() === '[B') {
                        console.log('  [BYTE_BUILDER] ' + name + '.' + methods[i].getName() +
                            '(' + Array.from(methods[i].getParameterTypes()).map(function(p){return p.getName();}).join(',') + ')');
                    }
                }
            } catch(e) {}
        },
        onComplete: function() { console.log('  (CCSDK scan done)'); }
    });

    // ── 4. Hook GATT TX (armed for when we invoke below) ─────────────────
    console.log('\n=== GATT TX hook ===');
    var BluetoothGatt = Java.use('android.bluetooth.BluetoothGatt');
    try {
        BluetoothGatt.writeCharacteristic
            .overload('android.bluetooth.BluetoothGattCharacteristic')
            .implementation = function(ch) {
                var val = Array.from(ch.getValue() || []);
                if (val.length > 0) {
                    console.log('\n>>>TX uuid=' + ch.getUuid());
                    console.log('    HEX: ' + hex(val));
                }
                return this.writeCharacteristic(ch);
            };
        console.log('  [hook] writeCharacteristic (legacy)');
    } catch(e) { console.log('  [skip legacy]: ' + e); }
    try {
        BluetoothGatt.writeCharacteristic
            .overload('android.bluetooth.BluetoothGattCharacteristic', '[B', 'int')
            .implementation = function(ch, value, writeType) {
                var val = Array.from(value || []);
                if (val.length > 0) {
                    console.log('\n>>>TX uuid=' + ch.getUuid());
                    console.log('    HEX: ' + hex(val));
                }
                return this.writeCharacteristic(ch, value, writeType);
            };
        console.log('  [hook] writeCharacteristic (api33+)');
    } catch(e) { console.log('  [skip api33]: ' + e); }

    // ── 5. Try to find and invoke the EQ command builder directly ─────────
    console.log('\n=== Attempting direct EQ command byte extraction ===');

    // Try EqUploadRequestBean.getBytes() or toBytes() patterns
    setTimeout(function() {
        Java.perform(function() {
            try {
                var EqUpload = Java.use('com.baseus.model.control.EqUploadRequestBean');
                var ctors = EqUpload.class.getDeclaredConstructors();
                console.log('\nEqUploadRequestBean constructors:');
                for (var i = 0; i < ctors.length; i++) {
                    var ps = Array.from(ctors[i].getParameterTypes()).map(function(p){return p.getName();});
                    console.log('  <init>(' + ps.join(',') + ')');
                }
            } catch(e) { console.log('EqUploadRequestBean error: ' + e); }

            // Scan for classes that wrap preset integers and have toBytes
            Java.enumerateLoadedClasses({
                onMatch: function(name) {
                    if (name.indexOf('bluetrum') < 0 && name.indexOf('ccsdk') < 0) return;
                    try {
                        var clazz = Java.use(name);
                        var fields = clazz.class.getDeclaredFields();
                        var hasByteMethod = false;
                        var methods = clazz.class.getDeclaredMethods();
                        for (var i = 0; i < methods.length; i++) {
                            if (methods[i].getReturnType().getName() === '[B') { hasByteMethod = true; break; }
                        }
                        if (!hasByteMethod) return;

                        // Try to instantiate with int(0..3) and call byte method
                        for (var mi = 0; mi < methods.length; mi++) {
                            var m = methods[mi];
                            if (m.getReturnType().getName() !== '[B') continue;
                            var ptypes = Array.from(m.getParameterTypes()).map(function(p){return p.getName();});
                            // Static method with 0-2 int params
                            var mods = m.getModifiers();
                            if ((mods & 8) !== 0) { // static
                                try {
                                    m.setAccessible(true);
                                    var args = ptypes.map(function(t){
                                        if (t === 'int' || t === 'byte') return 0;
                                        if (t === 'boolean') return false;
                                        return null;
                                    });
                                    var result = m.invoke(null, args);
                                    if (result) {
                                        console.log('\n  [STATIC BYTES] ' + name + '.' + m.getName() + '(' + ptypes.join(',') + ')');
                                        console.log('    preset=0: ' + hex(Array.from(result)));
                                    }
                                    // Try with arg=1,2,3 for presets
                                    for (var preset = 1; preset <= 3; preset++) {
                                        if (ptypes.length > 0) {
                                            args[0] = preset;
                                            result = m.invoke(null, args);
                                            if (result) console.log('    preset=' + preset + ': ' + hex(Array.from(result)));
                                        }
                                    }
                                } catch(e2) {}
                            }
                        }
                    } catch(e) {}
                },
                onComplete: function() { console.log('  (byte builder scan done)'); }
            });
        });
    }, 3000);

    console.log('\n[*] Script running. Check output above for class structure.');
    console.log('[*] GATT TX hook armed — will log any writes that occur.\n');
});
