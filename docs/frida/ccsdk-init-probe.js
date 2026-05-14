/**
 * ccsdk-init-probe.js — Initialize CCSDK with app context, then probe all ICCAgent methods
 *
 * Strategy:
 *   1. Get the app context
 *   2. Call CCSdkApi.c(context) to initialize the SDK
 *   3. Hook writeCharacteristic to capture any bytes sent
 *   4. Also hook ALL methods on the ICCAgent implementation
 *   5. Call each ICCAgent method with preset values 0-7 and observe outputs
 *
 * Usage: frida -D emulator-5554 -n Baseus -l docs/frida/ccsdk-init-probe.js
 */

'use strict';

function hex(bytes) {
    if (!bytes) return '(null)';
    return Array.from(bytes).map(function(b) {
        return ((b & 0xff) < 16 ? '0' : '') + (b & 0xff).toString(16).toUpperCase();
    }).join(' ');
}

// Track bytes intercepted at any level
var capturedBytes = [];

Java.perform(function() {
    console.log('\n[*] ccsdk-init-probe.js — ' + new Date().toISOString());

    // ── Hook GATT TX first ────────────────────────────────────────────────
    try {
        Java.use('android.bluetooth.BluetoothGatt')
            .writeCharacteristic
            .overload('android.bluetooth.BluetoothGattCharacteristic')
            .implementation = function(ch) {
                var val = Array.from(ch.getValue() || []);
                if (val.length > 0) {
                    console.log('\n>>>TX uuid=' + ch.getUuid() + '  HEX=' + hex(val));
                    capturedBytes.push({ via: 'gatt', bytes: val });
                }
                return this.writeCharacteristic(ch);
            };
        console.log('[hook] GATT writeCharacteristic (legacy)');
    } catch(e) { console.log('[skip] ' + e); }

    // ── Also intercept any byte[] being built by known builder classes ────
    // Hook every method returning byte[] in CCSDK classes b4, c4, n3, o3 (seen in earlier scan)
    var PROBE_CLASSES = [
        'com.bluetrum.ccsdk.b4', 'com.bluetrum.ccsdk.c4',
        'com.bluetrum.ccsdk.n3', 'com.bluetrum.ccsdk.o3',
        'com.bluetrum.ccsdk.b5', 'com.bluetrum.ccsdk.c5',
        'com.bluetrum.ccsdk.d4', 'com.bluetrum.ccsdk.d5',
        'com.bluetrum.ccsdk.e4', 'com.bluetrum.ccsdk.e5',
        'com.bluetrum.ccsdk.f4', 'com.bluetrum.ccsdk.f5',
        'com.bluetrum.ccsdk.g4', 'com.bluetrum.ccsdk.g5',
        'com.bluetrum.ccsdk.h4', 'com.bluetrum.ccsdk.h5',
    ];

    PROBE_CLASSES.forEach(function(cn) {
        try {
            var clazz = Java.use(cn);
            var methods = clazz.class.getDeclaredMethods();
            for (var i = 0; i < methods.length; i++) {
                var m = methods[i];
                if (m.getReturnType().getName() !== '[B') continue;
                try {
                    m.setAccessible(true);
                    (function(klass, mn) {
                        try {
                            klass[mn].implementation = function() {
                                var result = this[mn].apply(this, arguments);
                                if (result) {
                                    var bytes = Array.from(result);
                                    console.log('[builder] ' + cn + '.' + mn + ' -> ' + hex(bytes));
                                    capturedBytes.push({ via: cn + '.' + mn, bytes: bytes });
                                }
                                return result;
                            };
                        } catch(e2) {}
                    })(clazz, m.getName());
                } catch(e) {}
            }
        } catch(e) {}
    });

    // ── Initialize CCSdkApi with application context ──────────────────────
    setTimeout(function() {
        Java.perform(function() {
            try {
                var ActivityThread = Java.use('android.app.ActivityThread');
                var appCtx = ActivityThread.currentApplication().getApplicationContext();
                console.log('\n[*] Got app context: ' + appCtx);

                var CCSdkApi = Java.use('com.bluetrum.ccsdk.CCSdkApi');
                console.log('[*] Calling CCSdkApi.c(context) ...');
                CCSdkApi.c(appCtx);
                console.log('[*] CCSdkApi initialized');

                var agent = CCSdkApi.b();
                console.log('[*] ICCAgent: ' + agent);

                if (agent) {
                    // Inspect ICCAgent implementation
                    var implClass = agent.getClass();
                    console.log('[*] ICCAgent impl class: ' + implClass.getName());

                    var methods = implClass.getDeclaredMethods();
                    console.log('[*] ICCAgent has ' + methods.length + ' methods');

                    // Call each method with various int args and watch for GATT TX
                    for (var i = 0; i < methods.length; i++) {
                        try {
                            var m = methods[i];
                            var ptypes = Array.from(m.getParameterTypes()).map(function(p) { return p.getName(); });
                            var retType = m.getReturnType().getName();

                            // Only call methods with 1-3 primitive args (command setters)
                            if (ptypes.length === 0 || ptypes.length > 3) continue;
                            var allPrim = ptypes.every(function(t) {
                                return t === 'int' || t === 'byte' || t === 'short' || t === 'boolean' || t === 'long';
                            });
                            if (!allPrim) continue;

                            var prevCapCount = capturedBytes.length;

                            // Try calling with preset=0,1,2,3
                            for (var preset = 0; preset <= 3; preset++) {
                                try {
                                    m.setAccessible(true);
                                    var args = ptypes.map(function(t, idx) {
                                        if (idx === 0) return preset;
                                        return 0;
                                    });
                                    m.invoke(agent, args);
                                } catch(e2) {}
                            }

                            if (capturedBytes.length > prevCapCount) {
                                console.log('\n[ACTIVE] ' + implClass.getName() + '.' + m.getName() +
                                    '(' + ptypes.join(', ') + ') triggered ' +
                                    (capturedBytes.length - prevCapCount) + ' byte writes');
                            }
                        } catch(e) {}
                    }
                } else {
                    console.log('[!] ICCAgent is null — SDK may need BT device to be connected');
                    // Fall back to pure static probing on the builder classes
                    probeBuildersDirectly();
                }
            } catch(e) {
                console.log('[error] ' + e);
                probeBuildersDirectly();
            }
        });
    }, 2000);

    function probeBuildersDirectly() {
        console.log('\n[*] Probing builder classes directly (no init)...');

        // The key insight: if the SDK follows Bluetrum's standard CCSDK design,
        // the command byte array builder is a static method in a class named like b4/c4.
        // The static int field b=2 we found earlier is likely the command type ID.
        // Let's look for classes with static int fields AND byte[] builders.

        Java.enumerateLoadedClasses({
            onMatch: function(name) {
                if (name.indexOf('bluetrum') < 0 && name.indexOf('ccsdk') < 0) return;
                try {
                    var clazz = Java.use(name);
                    var fields = clazz.class.getDeclaredFields();
                    var hasStaticInt = false;
                    var staticIntVal = -1;

                    for (var i = 0; i < fields.length; i++) {
                        var f = fields[i];
                        f.setAccessible(true);
                        var mods = f.getModifiers();
                        if ((mods & 8) === 0) continue;
                        var cn = f.getType().getName();
                        if (cn === 'int' || cn === 'java.lang.Integer') {
                            try {
                                var v = parseInt(String(f.get(null)));
                                if (v >= 0 && v < 256) {
                                    hasStaticInt = true;
                                    staticIntVal = v;
                                }
                            } catch(e) {}
                        }
                    }

                    if (!hasStaticInt) return;

                    // This class has a static int constant — check if it has a constructor
                    // taking 1-2 ints and methods returning byte[]
                    var ctors = clazz.class.getDeclaredConstructors();
                    var methods = clazz.class.getDeclaredMethods();

                    var hasByteMethod = false;
                    for (var i = 0; i < methods.length; i++) {
                        if (methods[i].getReturnType().getName() === '[B') { hasByteMethod = true; break; }
                    }
                    if (!hasByteMethod) return;

                    // Try constructors with int args
                    for (var ci = 0; ci < ctors.length; ci++) {
                        try {
                            var ctor = ctors[ci];
                            ctor.setAccessible(true);
                            var cptypes = Array.from(ctor.getParameterTypes()).map(function(p) { return p.getName(); });
                            if (cptypes.length > 3) continue;
                            var allPrim = cptypes.every(function(t) {
                                return t === 'int' || t === 'byte' || t === 'short' || t === 'boolean';
                            });
                            if (!allPrim && cptypes.length > 0) continue;

                            for (var preset = 0; preset <= 3; preset++) {
                                try {
                                    var args = cptypes.map(function(t, idx) {
                                        if (idx === 0) return preset;
                                        return 0;
                                    });
                                    var inst = ctor.newInstance(args);
                                    if (!inst) continue;

                                    // Call byte[] methods on instance
                                    for (var mi = 0; mi < methods.length; mi++) {
                                        var m = methods[mi];
                                        if (m.getReturnType().getName() !== '[B') continue;
                                        try {
                                            m.setAccessible(true);
                                            var result = m.invoke(inst, []);
                                            if (result) {
                                                var bytes = Array.from(result);
                                                if (bytes.length > 2) {
                                                    console.log('[INSTANCE] ' + name + ' ctor(' + preset + '): ' + m.getName() + '() => ' + hex(bytes));
                                                }
                                            }
                                        } catch(e2) {}
                                    }
                                } catch(e3) {}
                            }
                        } catch(e) {}
                    }
                } catch(e) {}
            },
            onComplete: function() { console.log('[*] Builder probe complete'); }
        });
    }
});
