# Port Forwarder

A simple, low-latency, asynchronous TCP and UDP port proxy/forwarder written in Rust using [Tokio](https://tokio.rs/). 

This program reads a JSON configuration file and can concurrently listen on multiple ports, forwarding incoming TCP connections and bidirectional UDP traffic to specified target IP addresses and ports.

## Features

- **TCP Forwarding:** Transparently streams data back and forth between the client and the target server.
- **UDP Forwarding (Bidirectional):** Multiplexes UDP packets. It maintains underlying client states, meaning UDP replies from the target server are correctly routed back to the original client.
- **Low Latency:** Asynchronous I/O handling powered by Tokio ensures minimal overhead.

## Configuration

The forwarder is configured via a JSON file. The file should contain a JSON array of configuration objects.

**Example `config.json`:**
```json
[
  {
    "input_ip": "192.168.100.191",
    "input_port": 22,
    "output_ip": "10.8.0.4",
    "output_port": 223,
    "protocol": "tcp"
  },
  {
    "input_ip": "127.0.0.1",
    "input_port": 9090,
    "output_ip": "0.0.0.0",
    "output_port": 9091,
    "protocol": "udp"
  }
]
```

### Configuration Fields:
* `output_ip`: The IP address this proxy will **listen** on (e.g., `0.0.0.0` for all interfaces, or a specific IP like `10.8.0.4`).
* `output_port`: The port this proxy will **listen** on.
* `input_ip`: The target (destination) IP address the proxy will forward traffic to.
* `input_port`: The target (destination) port.
* `protocol`: Either `"tcp"` or `"udp"`.

## Building

To build the project for optimal performance, use Cargo's release profile:

```bash
cargo build --release
```
The compiled binary will be located at `target/release/port_forwarder`.

## Running the Proxy

Run the binary, supplying the path to your configuration file as the first argument:

```bash
./target/release/port_forwarder /path/to/your/config.json
```

### ⚠️ Troubleshooting: `Permission denied (os error 13)`
If you see the following error when starting the proxy:
> `Error in forwarder: Permission denied (os error 13)`

This happens because you are trying to listen (`output_port`) on a **privileged port** (any port number under `1024`, such as `223` or `80`). Standard users do not have permissions to bind to these ports on Linux.

**Solutions:**
1. **Run as Root (sudo):** 
   ```bash
   sudo ./target/release/port_forwarder /etc/port_forwarder/config.json
   ```
2. **Grant non-root bind capabilities (Preferred):**
   You can grant the binary permission to bind to privileged ports without running it as root using `setcap`:
   ```bash
   sudo setcap CAP_NET_BIND_SERVICE=+eip ./target/release/port_forwarder
   ./target/release/port_forwarder config.json
   ```
3. **Change the port:** Edit your config file and use an `output_port` greater than `1024` (e.g., `2222`).
