import frida, time, os
os.chdir(os.path.dirname(os.path.abspath(__file__)) + r"\..\..")

device = frida.get_usb_device(timeout=5)

# Kill existing, spawn fresh — Java bridge only works from process start
try:
    device.kill("com.baseus.intelligent")
    time.sleep(1)
except:
    pass

pid = device.spawn(["com.baseus.intelligent"])
print("Spawned PID:", pid)
session = device.attach(pid)

script = session.create_script("""
function waitForJava(cb, attempts) {
    attempts = attempts || 0;
    if (typeof Java !== 'undefined' && Java.available) {
        send("Java available after " + attempts + " retries");
        cb();
    } else if (attempts < 80) {
        setTimeout(function() { waitForJava(cb, attempts + 1); }, 250);
    } else {
        send("Java never became available after 20s");
    }
}
waitForJava(function() {
    Java.perform(function() {
        var count = 0;
        Java.enumerateLoadedClasses({
            onMatch: function(n) { count++; },
            onComplete: function() { send("OK total classes: " + count); }
        });
    });
});
""")

def on_msg(msg, data):
    print(msg.get("payload", msg))

script.on("message", on_msg)
script.load()
device.resume(pid)
time.sleep(25)
session.detach()
