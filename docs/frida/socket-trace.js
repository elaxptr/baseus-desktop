// Hooks Android BluetoothSocket I/O to log every byte in hex + timestamp.
// Works on both Classic BT (RFCOMM) and BLE GATT via the Java I/O layer.
Java.perform(function () {
  var OutputStream = Java.use('java.io.OutputStream');
  var InputStream  = Java.use('java.io.InputStream');

  function toHex(buf, offset, len) {
    var out = [];
    for (var i = 0; i < len; i++) {
      var b = buf[offset + i] & 0xff;
      out.push((b < 16 ? '0' : '') + b.toString(16));
    }
    return out.join(' ');
  }

  // Log outgoing bytes (app → earbuds)
  OutputStream.write.overload('[B', 'int', 'int').implementation = function (buf, off, len) {
    console.log('[TX ' + new Date().toISOString() + '] ' + toHex(buf, off, len));
    return this.write(buf, off, len);
  };

  // Log incoming bytes (earbuds → app)
  InputStream.read.overload('[B', 'int', 'int').implementation = function (buf, off, len) {
    var n = this.read(buf, off, len);
    if (n > 0) console.log('[RX ' + new Date().toISOString() + '] ' + toHex(buf, off, n));
    return n;
  };

  console.log('[socket-trace] hooks installed — waiting for Bluetooth I/O...');
});
