/**
 * ccsdk-init2.js — Get CCSdkApi instance via Kotlin companion, init SDK, probe ICCAgent
 * Usage: frida -D emulator-5554 -n Baseus -l docs/frida/ccsdk-init2.js
 */
'use strict';

function hex(bytes) {
    if (!bytes) return '(null)';
    return Array.from(bytes).map(function(b) {
        return ((b & 0xff) < 16 ? '0' : '') + (b & 0xff).toString(16).toUpperCase();
    }).join(' ');
}

Java.perform(function() {
    console.log('\n[*] ccsdk-init2.js — ' + new Date().toISOString());

    // Hook GATT TX
    try {
        Java.use('android.bluetooth.BluetoothGatt')
            .writeCharacteristic
            .overload('android.bluetooth.BluetoothGattCharacteristic')
            .implementation = function(ch) {
                var val = Array.from(ch.getValue() || []);
                if (val.length > 0) console.log('\n>>>TX uuid=' + ch.getUuid() + '  HEX=' + hex(val));
                return this.writeCharacteristic(ch);
            };
    } catch(e) {}

    // Inspect CCSdkApi class thoroughly
    try {
        var CCSdkApi = Java.use('com.bluetrum.ccsdk.CCSdkApi');
        var fields = CCSdkApi.class.getDeclaredFields();
        console.log('\n=== CCSdkApi fields ===');
        for (var i = 0; i < fields.length; i++) {
            var f = fields[i];
            f.setAccessible(true);
            var mods = f.getModifiers();
            var isStatic = (mods & 8) !== 0;
            var valStr = '';
            if (isStatic) {
                try { valStr = ' = ' + String(f.get(null)).slice(0, 60); } catch(e) {}
            }
            console.log('  [' + (isStatic ? 'S' : 'I') + '] ' + f.getType().getName() + ' ' + f.getName() + valStr);
        }

        var methods = CCSdkApi.class.getDeclaredMethods();
        console.log('\n=== CCSdkApi methods ===');
        for (var i = 0; i < methods.length; i++) {
            var m = methods[i];
            var ptypes = Array.from(m.getParameterTypes()).map(function(p) { return p.getName(); });
            var mods = m.getModifiers();
            console.log('  [' + ((mods & 8) ? 'S' : 'I') + '] ' + m.getReturnType().getName() + ' ' + m.getName() + '(' + ptypes.join(', ') + ')');
        }
    } catch(e) { console.log('CCSdkApi error: ' + e); }

    // Inspect CCSdkApi$Companion
    try {
        var Companion = Java.use('com.bluetrum.ccsdk.CCSdkApi$Companion');
        var fields = Companion.class.getDeclaredFields();
        var methods = Companion.class.getDeclaredMethods();
        console.log('\n=== CCSdkApi$Companion ===');
        for (var i = 0; i < fields.length; i++) {
            var f = fields[i];
            f.setAccessible(true);
            console.log('  FIELD: ' + f.getType().getName() + ' ' + f.getName());
        }
        for (var i = 0; i < methods.length; i++) {
            var m = methods[i];
            var ptypes = Array.from(m.getParameterTypes()).map(function(p) { return p.getName(); });
            console.log('  METHOD: ' + m.getReturnType().getName() + ' ' + m.getName() + '(' + ptypes.join(', ') + ')');
        }
    } catch(e) { console.log('Companion error: ' + e); }

    setTimeout(function() {
        Java.perform(function() {
            try {
                var appCtx = Java.use('android.app.ActivityThread').currentApplication().getApplicationContext();

                // Try to get CCSdkApi instance via INSTANCE field (Kotlin object)
                var CCSdkApi = Java.use('com.bluetrum.ccsdk.CCSdkApi');
                var fields = CCSdkApi.class.getDeclaredFields();
                var instance = null;

                for (var i = 0; i < fields.length; i++) {
                    var f = fields[i];
                    f.setAccessible(true);
                    var mods = f.getModifiers();
                    if ((mods & 8) === 0) continue; // static only
                    var typeName = f.getType().getName();
                    if (typeName === 'com.bluetrum.ccsdk.CCSdkApi' || typeName.indexOf('CCSdkApi') >= 0) {
                        instance = f.get(null);
                        if (instance) {
                            console.log('\n[*] Found CCSdkApi instance via field: ' + f.getName());
                            break;
                        }
                    }
                }

                if (!instance) {
                    // Try via a() method (Kotlin lazy)
                    try {
                        var lazy = CCSdkApi.a();
                        if (lazy) {
                            console.log('[*] Got lazy: ' + lazy);
                            var getMethod = lazy.getClass().getMethod('getValue', []);
                            instance = getMethod.invoke(lazy, []);
                            console.log('[*] Got instance from lazy: ' + instance);
                        }
                    } catch(e) { console.log('lazy error: ' + e); }
                }

                if (!instance) {
                    console.log('[!] Could not get CCSdkApi instance, trying to create one...');
                    try {
                        var ctors = CCSdkApi.class.getDeclaredConstructors();
                        for (var i = 0; i < ctors.length; i++) {
                            var ctor = ctors[i];
                            ctor.setAccessible(true);
                            var ptypes = Array.from(ctor.getParameterTypes());
                            if (ptypes.length === 0) {
                                instance = ctor.newInstance([]);
                                console.log('[*] Created CCSdkApi via default ctor');
                                break;
                            }
                        }
                    } catch(e) { console.log('ctor error: ' + e); }
                }

                if (!instance) { console.log('[!] Could not obtain CCSdkApi instance'); return; }

                // Initialize with context
                try {
                    var initMethod = CCSdkApi.class.getDeclaredMethod('c', [appCtx.getClass().getSuperclass().getSuperclass()]);
                    // c(Context)
                } catch(e) {}

                // Try calling c() with various context type params
                var methods = CCSdkApi.class.getDeclaredMethods();
                for (var i = 0; i < methods.length; i++) {
                    var m = methods[i];
                    var ptypes = Array.from(m.getParameterTypes()).map(function(p) { return p.getName(); });
                    if (ptypes.length === 1 && ptypes[0].indexOf('Context') >= 0) {
                        try {
                            m.setAccessible(true);
                            m.invoke(instance, [appCtx]);
                            console.log('[*] Called ' + m.getName() + '(Context) — SDK initialized');
                        } catch(e) { console.log('[?] init call: ' + e); }
                    }
                }

                // Get ICCAgent
                var agent = null;
                for (var i = 0; i < methods.length; i++) {
                    var m = methods[i];
                    var retType = m.getReturnType().getName();
                    var ptypes = Array.from(m.getParameterTypes());
                    if (retType.indexOf('ICCAgent') >= 0 && ptypes.length === 0) {
                        try {
                            m.setAccessible(true);
                            agent = m.invoke(instance, []);
                            console.log('[*] Got ICCAgent via ' + m.getName() + '(): ' + agent);
                            break;
                        } catch(e) { console.log('agent error: ' + e); }
                    }
                }

                if (!agent) { console.log('[!] ICCAgent is null after init'); return; }

                // Now probe all ICCAgent methods with int args
                console.log('\n=== Probing ICCAgent methods ===');
                var agentClass = agent.getClass();
                var agentMethods = agentClass.getDeclaredMethods();
                console.log('[*] ' + agentMethods.length + ' methods on ' + agentClass.getName());

                for (var i = 0; i < agentMethods.length; i++) {
                    try {
                        var m = agentMethods[i];
                        m.setAccessible(true);
                        var ptypes = Array.from(m.getParameterTypes()).map(function(p) { return p.getName(); });
                        var retType = m.getReturnType().getName();

                        // Skip methods with complex/non-primitive args
                        if (ptypes.length > 4) continue;
                        var allPrim = ptypes.every(function(t) {
                            return t === 'int' || t === 'byte' || t === 'short' || t === 'boolean' || t === 'long';
                        });
                        if (!allPrim && ptypes.length > 0) continue;

                        // Call with preset 0-3 watching for GATT writes
                        var txBefore = 0; // we'll track via console
                        console.log('\n  Testing ' + m.getName() + '(' + ptypes.join(',') + '):');
                        for (var preset = 0; preset <= 3; preset++) {
                            try {
                                var args = ptypes.map(function(t, idx) {
                                    if (idx === 0) return preset;
                                    return 0;
                                });
                                var result = m.invoke(agent, args);
                                if (retType === '[B' && result) {
                                    console.log('    preset=' + preset + ': ' + hex(Array.from(result)));
                                } else if (retType === 'void') {
                                    console.log('    preset=' + preset + ': void (watch for GATT TX above)');
                                }
                            } catch(e) {
                                if (preset === 0) console.log('    error: ' + e.message.slice(0, 80));
                            }
                        }
                    } catch(e) {}
                }
            } catch(e) { console.log('[error] ' + e); }
        });
    }, 2000);
});
