# Crossworld Test Client

A Rust test client for the Crossworld game server using WebTransport.

## Features

- Connects to WebTransport server via QUIC/HTTP3
- Sends Join message with player information
- Sends position updates at configurable rate
- Receives and displays position broadcasts from server
- Measures latency with ping/pong

## Usage

### Quick Test

```bash
# Terminal 1: Start the server
just server

# Terminal 2: Run the test client
just test-client
```

### Custom Options

```bash
cargo run --bin test-client -- \
  --server https://127.0.0.1:4433 \
  --name "MyPlayer" \
  --updates 200 \
  --rate-ms 50 \
  --log-level debug
```

### Options

- `--server` - Server URL (default: `https://127.0.0.1:4433`)
- `--name` - Player display name (default: `TestPlayer`)
- `--updates` - Number of position updates to send (default: `100`)
- `--rate-ms` - Update rate in milliseconds (default: `100`)
- `--log-level` - Log level: trace, debug, info, warn, error (default: `info`)

## What It Tests

1. **Connection** - WebTransport connection to server
2. **Join Flow** - Sends Join message with player info
3. **Position Updates** - Sends position datagrams at configured rate
4. **Position Broadcasting** - Receives position batches from server
5. **Graceful Disconnect** - Sends Leave message before closing

## Expected Output

```
INFO  Crossworld Test Client v0.1.0
INFO  Connecting to: https://127.0.0.1:4433
INFO  Connected to server!
INFO  Sent Join message for player: TestPlayer
INFO  Sending 100 position updates at 100ms intervals
DEBUG Sent position update #10: [1.0, 10.0, 0.5]
INFO  Received position batch with 1 players (total batches: 10)
INFO  Finished sending 100 position updates
INFO  Sent Leave message
INFO  Test client finished successfully
```

## Testing Multiple Clients

Run multiple clients simultaneously to test server broadcasting:

```bash
# Terminal 1: Server
just server

# Terminal 2: Client 1
cargo run --bin test-client -- --name "Alice" --updates 200

# Terminal 3: Client 2
cargo run --bin test-client -- --name "Bob" --updates 200

# Terminal 4: Client 3
cargo run --bin test-client -- --name "Carol" --updates 200
```

Each client will see position updates from the others in the server's broadcasts.

## TLS Certificates

For testing with self-signed certificates, you may need to:

1. Generate the certificate: `just gen-cert`
2. The client uses native cert store, so self-signed certs won't work by default
3. For testing, you can modify the client code to skip cert validation (not recommended for production)

## Troubleshooting

**Connection refused**
- Ensure the server is running: `just server`
- Check the server address and port

**TLS errors**
- Verify the certificate matches the server hostname
- Use `just gen-cert` to generate a fresh certificate

**No position broadcasts received**
- Check server logs for connection acceptance
- Verify the client sent a Join message successfully
- Try running with `--log-level debug` to see more details
