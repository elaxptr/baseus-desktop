/**
 * ccsdk-probe.js — Force-load and probe ALL CCSDK classes for byte-building methods
 *
 * Strategy: Since CCSDK classes aren't loaded without a device connection, we
 * enumerate them from DEX and force-call any static method returning byte[].
 * Vary int args 0..7 to discover EQ preset and gesture command bytes.
 *
 * Usage:
 *   frida -U -n Baseus -l docs/frida/ccsdk-probe.js
 */

'use strict';

function hex(bytes) {
    if (!bytes) return '(null)';
    return Array.from(bytes).map(function(b) {
        return ((b & 0xff) < 16 ? '0' : '') + (b & 0xff).toString(16).toUpperCase();
    }).join(' ');
}

// All CCSDK class names from static DEX analysis (classes4.dex)
var CCSDK_CLASSES = [
    'com.bluetrum.ccsdk.CCSdkApi',
    'com.bluetrum.ccsdk.a0','com.bluetrum.ccsdk.a2','com.bluetrum.ccsdk.a4','com.bluetrum.ccsdk.a5',
    'com.bluetrum.ccsdk.a6','com.bluetrum.ccsdk.a7',
    'com.bluetrum.ccsdk.b','com.bluetrum.ccsdk.b1','com.bluetrum.ccsdk.b2','com.bluetrum.ccsdk.b3',
    'com.bluetrum.ccsdk.b4','com.bluetrum.ccsdk.b5','com.bluetrum.ccsdk.b6','com.bluetrum.ccsdk.b7',
    'com.bluetrum.ccsdk.b8',
    'com.bluetrum.ccsdk.c0','com.bluetrum.ccsdk.c1','com.bluetrum.ccsdk.c2','com.bluetrum.ccsdk.c3',
    'com.bluetrum.ccsdk.c4','com.bluetrum.ccsdk.c5','com.bluetrum.ccsdk.c7',
    'com.bluetrum.ccsdk.d','com.bluetrum.ccsdk.d0','com.bluetrum.ccsdk.d2','com.bluetrum.ccsdk.d3',
    'com.bluetrum.ccsdk.d4','com.bluetrum.ccsdk.d5','com.bluetrum.ccsdk.d7',
    'com.bluetrum.ccsdk.e','com.bluetrum.ccsdk.e2','com.bluetrum.ccsdk.e5','com.bluetrum.ccsdk.e7',
    'com.bluetrum.ccsdk.f','com.bluetrum.ccsdk.f0','com.bluetrum.ccsdk.f2','com.bluetrum.ccsdk.f4',
    'com.bluetrum.ccsdk.f5','com.bluetrum.ccsdk.f6','com.bluetrum.ccsdk.f7',
    'com.bluetrum.ccsdk.g','com.bluetrum.ccsdk.g0','com.bluetrum.ccsdk.g2','com.bluetrum.ccsdk.g3',
    'com.bluetrum.ccsdk.g5','com.bluetrum.ccsdk.g6','com.bluetrum.ccsdk.g7',
    'com.bluetrum.ccsdk.h','com.bluetrum.ccsdk.h2','com.bluetrum.ccsdk.h3','com.bluetrum.ccsdk.h4',
    'com.bluetrum.ccsdk.h5','com.bluetrum.ccsdk.h6','com.bluetrum.ccsdk.h7',
    'com.bluetrum.ccsdk.i','com.bluetrum.ccsdk.i0','com.bluetrum.ccsdk.i2','com.bluetrum.ccsdk.i4',
    'com.bluetrum.ccsdk.i5','com.bluetrum.ccsdk.i6',
    'com.bluetrum.ccsdk.j','com.bluetrum.ccsdk.j0','com.bluetrum.ccsdk.j2','com.bluetrum.ccsdk.j3',
    'com.bluetrum.ccsdk.j4','com.bluetrum.ccsdk.j5','com.bluetrum.ccsdk.j6','com.bluetrum.ccsdk.j7',
    'com.bluetrum.ccsdk.k','com.bluetrum.ccsdk.k0','com.bluetrum.ccsdk.k2','com.bluetrum.ccsdk.k4',
    'com.bluetrum.ccsdk.k5','com.bluetrum.ccsdk.k6','com.bluetrum.ccsdk.k7',
    'com.bluetrum.ccsdk.l','com.bluetrum.ccsdk.l0','com.bluetrum.ccsdk.l1','com.bluetrum.ccsdk.l2',
    'com.bluetrum.ccsdk.l3','com.bluetrum.ccsdk.l4','com.bluetrum.ccsdk.l5','com.bluetrum.ccsdk.l6',
    'com.bluetrum.ccsdk.l7',
    'com.bluetrum.ccsdk.m','com.bluetrum.ccsdk.m0','com.bluetrum.ccsdk.m1','com.bluetrum.ccsdk.m2',
    'com.bluetrum.ccsdk.m3','com.bluetrum.ccsdk.m4','com.bluetrum.ccsdk.m5','com.bluetrum.ccsdk.m6',
    'com.bluetrum.ccsdk.m7',
    'com.bluetrum.ccsdk.n','com.bluetrum.ccsdk.n0','com.bluetrum.ccsdk.n1','com.bluetrum.ccsdk.n3',
    'com.bluetrum.ccsdk.n4','com.bluetrum.ccsdk.n5','com.bluetrum.ccsdk.n6','com.bluetrum.ccsdk.n7',
    'com.bluetrum.ccsdk.o','com.bluetrum.ccsdk.o0','com.bluetrum.ccsdk.o1','com.bluetrum.ccsdk.o2',
    'com.bluetrum.ccsdk.o3','com.bluetrum.ccsdk.o4','com.bluetrum.ccsdk.o5','com.bluetrum.ccsdk.o7',
    'com.bluetrum.ccsdk.p','com.bluetrum.ccsdk.p0','com.bluetrum.ccsdk.p1','com.bluetrum.ccsdk.p2',
    'com.bluetrum.ccsdk.p4','com.bluetrum.ccsdk.p5','com.bluetrum.ccsdk.p6','com.bluetrum.ccsdk.p7',
    'com.bluetrum.ccsdk.q','com.bluetrum.ccsdk.q0','com.bluetrum.ccsdk.q1','com.bluetrum.ccsdk.q2',
    'com.bluetrum.ccsdk.q3','com.bluetrum.ccsdk.q4','com.bluetrum.ccsdk.q5','com.bluetrum.ccsdk.q6',
    'com.bluetrum.ccsdk.q7',
    'com.bluetrum.ccsdk.r','com.bluetrum.ccsdk.r1','com.bluetrum.ccsdk.r2','com.bluetrum.ccsdk.r3',
    'com.bluetrum.ccsdk.r4','com.bluetrum.ccsdk.r5','com.bluetrum.ccsdk.r6',
    'com.bluetrum.ccsdk.s','com.bluetrum.ccsdk.s1','com.bluetrum.ccsdk.s2','com.bluetrum.ccsdk.s3',
    'com.bluetrum.ccsdk.s4','com.bluetrum.ccsdk.s5','com.bluetrum.ccsdk.s6','com.bluetrum.ccsdk.s7',
    'com.bluetrum.ccsdk.t','com.bluetrum.ccsdk.t1','com.bluetrum.ccsdk.t2','com.bluetrum.ccsdk.t3',
    'com.bluetrum.ccsdk.t4','com.bluetrum.ccsdk.t5','com.bluetrum.ccsdk.t7',
    'com.bluetrum.ccsdk.u','com.bluetrum.ccsdk.u1','com.bluetrum.ccsdk.u2','com.bluetrum.ccsdk.u3',
    'com.bluetrum.ccsdk.u4','com.bluetrum.ccsdk.u5','com.bluetrum.ccsdk.u6',
    'com.bluetrum.ccsdk.v','com.bluetrum.ccsdk.v1','com.bluetrum.ccsdk.v2','com.bluetrum.ccsdk.v3',
    'com.bluetrum.ccsdk.v4','com.bluetrum.ccsdk.v5','com.bluetrum.ccsdk.v6','com.bluetrum.ccsdk.v7',
    'com.bluetrum.ccsdk.w','com.bluetrum.ccsdk.w1','com.bluetrum.ccsdk.w2','com.bluetrum.ccsdk.w3',
    'com.bluetrum.ccsdk.w4','com.bluetrum.ccsdk.w5','com.bluetrum.ccsdk.w6','com.bluetrum.ccsdk.w7',
    'com.bluetrum.ccsdk.x','com.bluetrum.ccsdk.x1','com.bluetrum.ccsdk.x2','com.bluetrum.ccsdk.x3',
    'com.bluetrum.ccsdk.x4','com.bluetrum.ccsdk.x6','com.bluetrum.ccsdk.x7',
    'com.bluetrum.ccsdk.y','com.bluetrum.ccsdk.y1','com.bluetrum.ccsdk.y2','com.bluetrum.ccsdk.y4',
    'com.bluetrum.ccsdk.y5','com.bluetrum.ccsdk.y6','com.bluetrum.ccsdk.y7',
    'com.bluetrum.ccsdk.z','com.bluetrum.ccsdk.z1','com.bluetrum.ccsdk.z2','com.bluetrum.ccsdk.z3',
    'com.bluetrum.ccsdk.z4','com.bluetrum.ccsdk.z5','com.bluetrum.ccsdk.z6','com.bluetrum.ccsdk.z7',
];

Java.perform(function() {
    console.log('\n[*] ccsdk-probe.js attached — ' + new Date().toISOString());
    console.log('[*] Probing ' + CCSDK_CLASSES.length + ' CCSDK classes...\n');

    var found = [];

    CCSDK_CLASSES.forEach(function(cn) {
        try {
            var clazz = Java.use(cn);
            var methods = clazz.class.getDeclaredMethods();
            var fields = clazz.class.getDeclaredFields();

            // Look for static methods returning byte[] with 0-3 int params
            for (var i = 0; i < methods.length; i++) {
                var m = methods[i];
                var mods = m.getModifiers();
                var retType = m.getReturnType().getName();
                var ptypes = Array.from(m.getParameterTypes()).map(function(p) { return p.getName(); });

                if (retType !== '[B') continue;
                if (ptypes.length > 4) continue;

                // Check if all params are primitives (int, byte, boolean, short)
                var allPrimitive = ptypes.every(function(t) {
                    return t === 'int' || t === 'byte' || t === 'boolean' || t === 'short' || t === 'long';
                });
                if (!allPrimitive && ptypes.length > 0) continue;

                found.push({ cls: cn, m: m, ptypes: ptypes, isStatic: (mods & 8) !== 0 });
            }
        } catch(e) {}
    });

    console.log('[*] Found ' + found.length + ' byte[]-returning methods with primitive args\n');

    // Try calling each with preset=0..3
    found.forEach(function(entry) {
        try {
            entry.m.setAccessible(true);
            var results = {};
            var anyDifferent = false;
            var prev = null;

            for (var preset = 0; preset <= 3; preset++) {
                try {
                    var args = entry.ptypes.map(function(t, idx) {
                        if (idx === 0) return preset;
                        if (t === 'int') return 0;
                        if (t === 'byte') return 0;
                        if (t === 'boolean') return false;
                        if (t === 'short') return 0;
                        if (t === 'long') return 0;
                        return 0;
                    });

                    var result = entry.isStatic
                        ? entry.m.invoke(null, args)
                        : null; // skip instance methods for now

                    if (result) {
                        var bytes = Array.from(result);
                        var h = hex(bytes);
                        results[preset] = h;
                        if (prev !== null && prev !== h) anyDifferent = true;
                        prev = h;
                    }
                } catch(e) {}
            }

            // Only print if results vary (proof it's a command selector)
            if (anyDifferent && Object.keys(results).length > 1) {
                console.log('[VARIES] ' + entry.cls + '.' + entry.m.getName() +
                    '(' + entry.ptypes.join(', ') + ')');
                Object.keys(results).forEach(function(p) {
                    console.log('  arg[0]=' + p + ': ' + results[p]);
                });
            } else if (Object.keys(results).length > 0) {
                // Print single-result methods too (might be fixed-command)
                var firstKey = Object.keys(results)[0];
                var bytes = results[firstKey];
                // Only show if it looks like a protocol packet (starts with BA or AA)
                if (bytes.startsWith('BA') || bytes.startsWith('AA') || bytes.length > 5) {
                    console.log('[FIXED] ' + entry.cls + '.' + entry.m.getName() +
                        '(' + entry.ptypes.join(', ') + '): ' + bytes);
                }
            }
        } catch(e) {}
    });

    // Also: look at CCSdkApi's public methods directly
    console.log('\n=== CCSdkApi methods ===');
    try {
        var api = Java.use('com.bluetrum.ccsdk.CCSdkApi');
        var methods = api.class.getDeclaredMethods();
        for (var i = 0; i < methods.length; i++) {
            var m = methods[i];
            var ptypes = Array.from(m.getParameterTypes()).map(function(p) { return p.getName(); });
            console.log('  ' + m.getReturnType().getName() + ' ' + m.getName() + '(' + ptypes.join(', ') + ')');
        }
    } catch(e) { console.log('  Error: ' + e); }

    console.log('\n[*] Probe complete.\n');
});
