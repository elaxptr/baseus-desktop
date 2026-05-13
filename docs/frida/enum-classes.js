// Enumerate protocol/Bluetooth-related classes and dump static field values.
Java.perform(function () {
    var results = [];

    Java.enumerateLoadedClasses({
        onMatch: function (name) {
            var lower = name.toLowerCase();
            if (lower.indexOf('baseus') !== -1 ||
                lower.indexOf('bluetooth') !== -1 ||
                lower.indexOf('rfcomm') !== -1 ||
                lower.indexOf('protocol') !== -1 ||
                lower.indexOf('command') !== -1 ||
                lower.indexOf('packet') !== -1 ||
                lower.indexOf('frame') !== -1 ||
                lower.indexOf('opcode') !== -1 ||
                lower.indexOf('headset') !== -1 ||
                lower.indexOf('earbud') !== -1 ||
                lower.indexOf('socket') !== -1 ||
                lower.indexOf('serial') !== -1) {
                results.push(name);
            }
        },
        onComplete: function () {
            var skip = ['android.bluetooth', 'java.', 'javax.', 'android.net',
                        'android.os', 'com.android', 'dalvik', 'sun.'];
            var appClasses = results.filter(function (n) {
                for (var i = 0; i < skip.length; i++) {
                    if (n.indexOf(skip[i]) === 0) return false;
                }
                return true;
            });

            send('[enum] Total keyword matches: ' + results.length);
            send('[enum] App-level classes: ' + appClasses.length);
            appClasses.forEach(function (cls) { send('[CLASS] ' + cls); });

            appClasses.forEach(function (cls) {
                try {
                    var C = Java.use(cls);
                    var fields = C.class.getDeclaredFields();
                    fields.forEach(function (f) {
                        try {
                            var mods = f.getModifiers();
                            if ((mods & 8) !== 0) { // static
                                f.setAccessible(true);
                                var val = f.get(null);
                                send('[FIELD] ' + cls + '.' + f.getName() + ' = ' + val);
                            }
                        } catch (e) {}
                    });
                } catch (e) {}
            });
            send('[DONE]');
        }
    });
});
