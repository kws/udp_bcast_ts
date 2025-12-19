# UDP Broadcast Timestamp

A lightweight Rust utility that broadcasts Unix timestamps (in milliseconds) over UDP at regular intervals. Useful for network time synchronization, especially with embedded devices like Raspberry Pi Pico.

## Features

- Broadcasts Unix timestamps (milliseconds since epoch) as 8-byte big-endian `u64`
- Supports both IPv4 and IPv6
- Configurable broadcast interval
- Minimal dependencies (standard library only)
- Optimized release build with LTO and symbol stripping

## Requirements

- Rust 1.70+ (or latest stable)
- Cargo

## Compilation

### Debug Build

```bash
cargo build
```

The binary will be located at `target/debug/udp_bcast_ts`

### Release Build

```bash
cargo build --release
```

The optimized binary will be located at `target/release/udp_bcast_ts`

The release profile is configured with:
- Link-time optimization (LTO)
- Single codegen unit for better optimization
- Panic abort (smaller binary)
- Symbol stripping

### Cross-compilation

For cross-compilation (e.g., for Raspberry Pi), install the appropriate target:

```bash
# Example: ARMv7 (Raspberry Pi)
rustup target add armv7-unknown-linux-gnueabihf

# Build for target
cargo build --release --target armv7-unknown-linux-gnueabihf
```

## Usage

```bash
udp_bcast_ts --addr <IPv4-or-IPv6> --port <1-65535> [--interval-ms <ms>]
```

### Arguments

- `--addr <IP>`: **Required.** The broadcast address (IPv4 or IPv6)
  - IPv4 example: `255.255.255.255` (local network broadcast)
  - IPv6 example: `ff02::1` (all nodes multicast)
- `--port <PORT>`: **Required.** Destination port number (1-65535)
- `--interval-ms <MS>`: **Optional.** Broadcast interval in milliseconds (default: 1000)
- `-h, --help`: Display usage information

### Examples

**IPv4 broadcast every second:**
```bash
./target/release/udp_bcast_ts --addr 255.255.255.255 --port 12321 --interval-ms 1000
```

**IPv6 multicast every 500ms:**
```bash
./target/release/udp_bcast_ts --addr ff02::1 --port 12321 --interval-ms 500
```

**High-frequency updates (10ms interval):**
```bash
./target/release/udp_bcast_ts --addr 255.255.255.255 --port 12321 --interval-ms 10
```

## Payload Format

Each UDP packet contains exactly 8 bytes:
- Format: Big-endian `u64` (network byte order)
- Content: Milliseconds since Unix epoch (January 1, 1970, 00:00:00 UTC)
- Equivalent to Python: `struct.pack("!Q", timestamp_ms)`

### Receiving the Timestamp

**Python example:**
```python
import socket
import struct
import time

sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
sock.bind(('0.0.0.0', 12321))

while True:
    data, addr = sock.recvfrom(8)
    ts_ms = struct.unpack('!Q', data)[0]
    print(f"Received timestamp: {ts_ms} ({time.ctime(ts_ms / 1000)})")
```

**Rust example:**
```rust
use std::net::UdpSocket;

let sock = UdpSocket::bind("0.0.0.0:12321")?;
let mut buf = [0u8; 8];

loop {
    sock.recv_from(&mut buf)?;
    let ts_ms = u64::from_be_bytes(buf);
    println!("Received timestamp: {} ms", ts_ms);
}
```

## Exit Codes

- `0`: Success (only when `--help` is used)
- `1`: Runtime error (socket binding, system clock error, etc.)
- `2`: Usage error (invalid arguments, missing required options)

## Error Handling

- **Send failures**: The program logs errors but continues running to allow recovery from transient network issues
- **System clock errors**: Program exits if the system clock is set before Unix epoch
- **Timestamp overflow**: Program exits if the timestamp exceeds `u64::MAX` (unlikely in practice)

## License

MIT