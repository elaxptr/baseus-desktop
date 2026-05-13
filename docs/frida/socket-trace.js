// Hooks Bluetrum CCSDK BLE GATT writes and notifications to log every byte.
// The BP1 Pro ANC uses BLE GATT (NOT RFCOMM) for all runtime control:
//   Write char:  02F00000-0000-0000-0000-00000000FF01  (app → earbuds)
//   Notify char: 02F00000-0000-0000-0000-00000000FF02  (earbuds → app)
//
// Usage (real Android device, not emulator):
//   frida -U -f com.baseus.intelligent -l docs/frida/socket-trace.js
//
// NOTE: Frida 17.x Java bridge does not work on x86_64 Android 12 emulators.
//       Run this on a real Android device with USB debugging enabled.

Java.perform(function () {
  function toHex(buf) {
    if (!buf) return '(null)';
    var out = [];
    for (var i = 0; i < buf.length; i++) {
      var b = buf[i] & 0xff;
      out.push((b < 16 ? '0' : '') + b.toString(16));
    }
    return out.join(' ');
  }

  // ── BLE GATT write (app → earbuds) ──────────────────────────────────────
  try {
    var BluetoothGatt = Java.use('android.bluetooth.BluetoothGatt');

    BluetoothGatt.writeCharacteristic.overload(
      'android.bluetooth.BluetoothGattCharacteristic'
    ).implementation = function (ch) {
      var uuid = ch.getUuid().toString();
      var val  = ch.getValue();
      console.log('[GATT TX ' + new Date().toISOString() + '] uuid=' + uuid
        + ' bytes=' + toHex(val));
      return this.writeCharacteristic(ch);
    };

    // Android 13+ overload
    BluetoothGatt.writeCharacteristic.overload(
      'android.bluetooth.BluetoothGattCharacteristic', '[B', 'int'
    ).implementation = function (ch, value, writeType) {
      var uuid = ch.getUuid().toString();
      console.log('[GATT TX13 ' + new Date().toISOString() + '] uuid=' + uuid
        + ' bytes=' + toHex(value));
      return this.writeCharacteristic(ch, value, writeType);
    };
  } catch (e) {
    console.log('[socket-trace] GATT write hook failed: ' + e);
  }

  // ── BLE GATT notification callback (earbuds → app) ──────────────────────
  try {
    var BluetoothGattCallback = Java.use('android.bluetooth.BluetoothGattCallback');

    BluetoothGattCallback.onCharacteristicChanged.overload(
      'android.bluetooth.BluetoothGatt',
      'android.bluetooth.BluetoothGattCharacteristic'
    ).implementation = function (gatt, ch) {
      var uuid = ch.getUuid().toString();
      var val  = ch.getValue();
      console.log('[GATT RX ' + new Date().toISOString() + '] uuid=' + uuid
        + ' bytes=' + toHex(val));
      return this.onCharacteristicChanged(gatt, ch);
    };

    // Android 13+ callback signature
    BluetoothGattCallback.onCharacteristicChanged.overload(
      'android.bluetooth.BluetoothGatt',
      'android.bluetooth.BluetoothGattCharacteristic', '[B'
    ).implementation = function (gatt, ch, value) {
      var uuid = ch.getUuid().toString();
      console.log('[GATT RX13 ' + new Date().toISOString() + '] uuid=' + uuid
        + ' bytes=' + toHex(value));
      return this.onCharacteristicChanged(gatt, ch, value);
    };
  } catch (e) {
    console.log('[socket-trace] GATT notify hook failed: ' + e);
  }

  // ── Also hook classic RFCOMM (JieLi OTA path) ───────────────────────────
  try {
    var OutputStream = Java.use('java.io.OutputStream');
    var InputStream  = Java.use('java.io.InputStream');

    OutputStream.write.overload('[B', 'int', 'int').implementation = function (buf, off, len) {
      console.log('[SPP TX ' + new Date().toISOString() + '] ' + toHex(buf.slice(off, off + len)));
      return this.write(buf, off, len);
    };

    InputStream.read.overload('[B', 'int', 'int').implementation = function (buf, off, len) {
      var n = this.read(buf, off, len);
      if (n > 0) console.log('[SPP RX ' + new Date().toISOString() + '] ' + toHex(buf.slice(off, off + n)));
      return n;
    };
  } catch (e) {
    console.log('[socket-trace] SPP hook failed: ' + e);
  }

  console.log('[socket-trace] hooks installed — waiting for BLE GATT / SPP I/O...');
  console.log('[socket-trace] Target UUIDs:');
  console.log('[socket-trace]   Service  02f00000-0000-0000-0000-00000000fe00');
  console.log('[socket-trace]   Write    02f00000-0000-0000-0000-00000000ff01');
  console.log('[socket-trace]   Notify   02f00000-0000-0000-0000-00000000ff02');
});
