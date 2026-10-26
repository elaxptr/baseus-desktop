# Get your Baseus model supported

Every model in this app was added from a recording of the official Baseus app talking to
real earbuds. If your model isn't supported yet, you can make that recording on your own
phone in about 15 minutes, with no root, no PC tools, and no coding. Then send it in and it
can be added.

The [BP1 Ultra](protocol/bp1-ultra.md) was added exactly this way by its owner.

## What you need

- An **Android** phone (iPhones can't record Bluetooth traffic without a Mac and Apple's
  developer tools)
- The official **Baseus** app, already working with your earbuds
- Your earbuds, charged

## 1. Turn on Bluetooth logging

1. Open **Settings → About phone** and tap **Build number** seven times. You'll see
   "You are now a developer". (On some phones it's under **Software information**.)
2. Go to **Settings → System → Developer options** (or search settings for "developer").
3. Turn on **Enable Bluetooth HCI snoop log**. If it offers a choice, pick **Enabled** or
   **Full**, not "Filtered".
4. Turn **Bluetooth off and back on**. Logging only starts after this.

## 2. Record

Disconnect other Bluetooth devices (watch, car, speaker) if you can, so the log only has
your earbuds in it.

Open the Baseus app with your earbuds connected. Do each action below **slowly, about
5 seconds apart**, and **write down what you did, in order**. The list is as important as
the log. Skip anything your app doesn't have.

1. Close and reopen the Baseus app
2. Noise control: **Off → ANC → Transparency → Off**
3. If there's an ANC strength slider or levels: **lowest → highest**
4. Each **EQ preset**, one by one
5. **Game / low-latency mode**: on → off
6. **Find my earbuds**: left, then right (stop each one)
7. Anything else the app offers: spatial audio, wind reduction, gestures, ...
8. Put the buds in the case, close the lid, open it again

Example list:

```
1. reopened app
2. ANC off -> ANC on -> transparency -> off
3. strength 1 -> 5
4. EQ: Balanced, Bass, Vocal, Treble
5. game mode on -> off
```

## 3. Get the log

1. Back in **Developer options**, tap **Take bug report** → **Full report**. Wait a few
   minutes for the notification, then share or save the `.zip`.
2. Open the zip and find **`btsnoop_hci.log`**. It's usually under
   `FS/data/misc/bluetooth/logs/`, but the folder varies by phone, so search the zip for
   `btsnoop`.
3. Turn **Enable Bluetooth HCI snoop log** back off.

## 4. Send it in

[Open an issue](https://github.com/elaxptr/baseus-desktop/issues/new) (or reply on the
existing one for your model) with:

- the earbuds' **exact name** as shown in your phone's Bluetooth list
- the `btsnoop_hci.log` file (zip it if GitHub won't accept it)
- your list of actions
- the Baseus app version and, if the app shows it, the earbuds' firmware version

**Privacy:** the log holds the Bluetooth addresses of your phone and earbuds, and anything
else that used Bluetooth while it was recording. It doesn't contain your passwords or
messages, but if you'd rather not post it publicly, say so in the issue and we'll find
another way.

## After that

The recording gets decoded into a protocol description, then implemented. You'll likely be
asked to test a build on your earbuds, since a model is only marked supported once it's
been confirmed on real hardware.

Comfortable with Wireshark, adb or Frida and want to decode it yourself? See
[re-methodology.md](re-methodology.md) and the [BP1 Ultra notes](protocol/bp1-ultra.md)
for a worked example.
