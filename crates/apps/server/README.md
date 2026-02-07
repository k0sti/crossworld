# server

Game server draft using WebTransport (QUIC/HTTP3).

**Status:** Draft implementation

## Current Status

The server structure has been implemented with the following components:

- ✅ Core data structures (GameServer, Player, ServerConfig)
- ✅ Message protocol (ReliableMessage, UnreliableMessage, CompactPosition)
- ✅ Position broadcasting system with interest management
- ✅ Anti-cheat position validation
- ✅ Nostr discovery announcer
- ✅ CLI arguments and configuration
- ✅ Metrics and monitoring

## Known Issues

**WebTransport API Compatibility:**
The `wtransport` crate API has changed significantly between versions. The current implementation targets wtransport 0.6.x, which has different APIs than earlier versions.

**Action Required:**
To complete the implementation, update `connection.rs` and `broadcast.rs` to use the wtransport 0.6 API:

1. Replace `transport.datagrams()` with the new datagram API
2. Update stream handling to use wtransport 0.6 patterns
3. Refer to wtransport documentation: https://docs.rs/wtransport/0.6.1/wtransport/

## Running the Server

See `justfile` for server commands:

```bash
just gen-cert   # Generate self-signed certificate
just server     # Run in development mode
```

## Architecture

See `CLAUDE.md` (Game Server section) and `doc/server.md` for full architecture documentation.

## Client Integration

Clients will need to implement WebTransport connection logic to communicate with the server using the message protocol defined in `messages.rs`.
