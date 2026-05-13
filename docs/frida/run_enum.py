import frida, sys, time, os

os.chdir(os.path.dirname(os.path.abspath(__file__)) + r"\..\..")

script_src = open(r"docs\frida\enum-classes.js").read()

device = frida.get_usb_device(timeout=5)
# Spawn fresh so Java runtime is definitely initialised
pid = device.spawn(["com.baseus.intelligent"])
session = device.attach(pid)
script = session.create_script(script_src)

lines = []
done = [False]

def on_message(msg, data):
    if msg["type"] in ("send", "log"):
        payload = msg.get("payload", "")
        lines.append(str(payload))
        print(str(payload))
        if str(payload) == "[DONE]":
            done[0] = True
    elif msg["type"] == "error":
        lines.append("[ERR] " + msg.get("description", ""))
        print("[ERR]", msg.get("description", ""))

script.on("message", on_message)
script.load()
device.resume(pid)

deadline = time.time() + 45
while not done[0] and time.time() < deadline:
    time.sleep(0.5)

with open(r"docs\protocol\captures\frida-class-enum.log", "w") as f:
    f.write("\n".join(lines))

try:
    session.detach()
except:
    pass
print("\nSaved to docs\\protocol\\captures\\frida-class-enum.log")
