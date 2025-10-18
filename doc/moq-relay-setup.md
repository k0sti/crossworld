# MoQ Relay Server Setup Guide

Quick reference for setting up a MoQ relay server for Crossworld voice chat.

## Quick Start (Testing with Public Relay)

The default setup uses a public test relay - **no setup required**:

```bash
cd crates/worldtool
cargo run -- init-live
```

⚠️ **Public relay limitations**:
- Shared with other developers
- May have rate limits
- No availability/uptime guarantees
- Not suitable for production

## Quick Start (Local Development Server)

Use worldtool to set up a local MoQ relay server in minutes:

```bash
cd crates/worldtool

# Initialize server (clone and build)
cargo run -- server init

# Run server (generates self-signed cert automatically)
cargo run -- server run

# In another terminal, configure live event
cargo run -- init-live --streaming https://localhost:4443/anon
```

That's it! The server is now running locally.

**Commands**:
- `server init [--dir <path>]` - Clone and build MoQ relay
- `server run [options]` - Start the relay server

**Options for `server run`**:
- `--port <PORT>` - Port to bind (default: 4443)
- `--bind <ADDR>` - Bind address (default: 0.0.0.0)
- `--tls-cert <PATH>` - Custom TLS certificate
- `--tls-key <PATH>` - Custom TLS key
- `--verbose` - Enable verbose logging

**Examples**:
```bash
# Custom directory
cargo run -- server init --dir ~/my-moq-server

# Custom port
cargo run -- server run --port 8443

# With custom certificates
cargo run -- server run --tls-cert /path/to/cert.pem --tls-key /path/to/key.pem

# Verbose logging
cargo run -- server run --verbose
```

## Production Deployment

### Prerequisites

- Server with public IP
- Domain name (e.g., moq.yourdomain.com)
- HTTPS certificate (Let's Encrypt recommended)
- Open port 4443 (or custom port)

### Method 1: Native Installation

**Install Rust** (if not already installed):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

**Clone and build MoQ relay**:
```bash
git clone https://github.com/kixelated/moq.git
cd moq
cargo build --release --bin moq-relay
```

**Run with systemd** (recommended):

Create `/etc/systemd/system/moq-relay.service`:

```ini
[Unit]
Description=MoQ Relay Server
After=network.target

[Service]
Type=simple
User=moq
WorkingDirectory=/opt/moq
ExecStart=/opt/moq/target/release/moq-relay \
  --bind 0.0.0.0:4443 \
  --tls-cert /etc/letsencrypt/live/moq.yourdomain.com/fullchain.pem \
  --tls-key /etc/letsencrypt/live/moq.yourdomain.com/privkey.pem
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl daemon-reload
sudo systemctl enable moq-relay
sudo systemctl start moq-relay
sudo systemctl status moq-relay
```

### Method 2: Docker

**Simple docker run**:
```bash
docker run -d \
  --name moq-relay \
  -p 4443:4443 \
  -v /etc/letsencrypt:/etc/letsencrypt:ro \
  kixelated/moq-relay \
  --bind 0.0.0.0:4443 \
  --tls-cert /etc/letsencrypt/live/moq.yourdomain.com/fullchain.pem \
  --tls-key /etc/letsencrypt/live/moq.yourdomain.com/privkey.pem
```

**Docker Compose** (`docker-compose.yml`):
```yaml
version: '3.8'

services:
  moq-relay:
    image: kixelated/moq-relay:latest
    container_name: moq-relay
    restart: unless-stopped
    ports:
      - "4443:4443/tcp"
      - "4443:4443/udp"
    volumes:
      - /etc/letsencrypt:/etc/letsencrypt:ro
    command: >
      --bind 0.0.0.0:4443
      --tls-cert /etc/letsencrypt/live/moq.yourdomain.com/fullchain.pem
      --tls-key /etc/letsencrypt/live/moq.yourdomain.com/privkey.pem
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"
```

Start:
```bash
docker-compose up -d
docker-compose logs -f  # View logs
```

### HTTPS Setup

#### Option A: Caddy (Automatic HTTPS)

**Install Caddy**:
```bash
sudo apt install -y debian-keyring debian-archive-keyring apt-transport-https
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | sudo gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | sudo tee /etc/apt/sources.list.d/caddy-stable.list
sudo apt update
sudo apt install caddy
```

**Configure** (`/etc/caddy/Caddyfile`):
```caddyfile
moq.yourdomain.com {
    reverse_proxy localhost:4443 {
        transport http {
            versions h2
        }
    }
}
```

**Start**:
```bash
sudo systemctl restart caddy
```

#### Option B: nginx + Let's Encrypt

**Install certbot**:
```bash
sudo apt install certbot python3-certbot-nginx
```

**Get certificate**:
```bash
sudo certbot --nginx -d moq.yourdomain.com
```

**Configure nginx** (`/etc/nginx/sites-available/moq`):
```nginx
server {
    listen 443 ssl http2;
    server_name moq.yourdomain.com;

    ssl_certificate /etc/letsencrypt/live/moq.yourdomain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/moq.yourdomain.com/privkey.pem;

    location / {
        proxy_pass https://localhost:4443;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

**Enable and reload**:
```bash
sudo ln -s /etc/nginx/sites-available/moq /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl reload nginx
```

### Firewall Configuration

**UFW (Ubuntu/Debian)**:
```bash
sudo ufw allow 4443/tcp
sudo ufw allow 4443/udp
sudo ufw allow 443/tcp  # If using reverse proxy
```

**firewalld (CentOS/RHEL)**:
```bash
sudo firewall-cmd --permanent --add-port=4443/tcp
sudo firewall-cmd --permanent --add-port=4443/udp
sudo firewall-cmd --permanent --add-port=443/tcp
sudo firewall-cmd --reload
```

## Configuration

### Update Crossworld Live Event

```bash
cd crates/worldtool

# With your relay
cargo run -- init-live-chat --streaming "https://moq.yourdomain.com/anon"
```

### Test Connection

```bash
# Basic connectivity (should get WebTransport error - this is expected)
curl -k https://moq.yourdomain.com:4443/anon

# From browser console
fetch('https://moq.yourdomain.com/anon')
  .then(r => console.log('Relay reachable'))
  .catch(e => console.log('Expected WebTransport error:', e))
```

## Monitoring

### View Logs

**systemd**:
```bash
sudo journalctl -u moq-relay -f
```

**Docker**:
```bash
docker logs -f moq-relay
```

### Metrics

Monitor:
- Connection count
- Bandwidth usage
- CPU/memory usage

```bash
# View active connections
ss -tln | grep 4443

# Docker stats
docker stats moq-relay
```

## Troubleshooting

### Connection Refused

**Check if relay is running**:
```bash
sudo systemctl status moq-relay  # systemd
docker ps | grep moq-relay       # docker
```

**Check port is open**:
```bash
sudo netstat -tlnp | grep 4443
```

### Certificate Errors

**Verify cert is valid**:
```bash
openssl s_client -connect moq.yourdomain.com:4443
```

**Renew Let's Encrypt cert**:
```bash
sudo certbot renew
sudo systemctl restart moq-relay
```

### Client Can't Connect

**Check CORS headers** (if using reverse proxy):
```nginx
add_header Access-Control-Allow-Origin *;
add_header Access-Control-Allow-Methods "GET, POST, OPTIONS";
```

**Verify WebTransport support**:
- Browser must support WebTransport (Chrome 97+, Edge 97+)
- HTTPS is required (no HTTP)
- Shared array buffer headers must be set on app

## Performance Tuning

### System Limits

Increase file descriptor limits:

```bash
# /etc/security/limits.conf
* soft nofile 65536
* hard nofile 65536
```

### Relay Configuration

```bash
moq-relay \
  --bind 0.0.0.0:4443 \
  --tls-cert cert.pem \
  --tls-key key.pem \
  --max-connections 1000  # Adjust based on capacity
```

## Security

### Best Practices

1. **Use strong TLS configuration**:
```nginx
ssl_protocols TLSv1.2 TLSv1.3;
ssl_ciphers HIGH:!aNULL:!MD5;
```

2. **Rate limiting** (nginx):
```nginx
limit_req_zone $binary_remote_addr zone=relay:10m rate=10r/s;
limit_req zone=relay burst=20 nodelay;
```

3. **Fail2ban** (optional):
Monitor and ban abusive IPs

4. **Regular updates**:
```bash
cd /opt/moq
git pull
cargo build --release --bin moq-relay
sudo systemctl restart moq-relay
```

## Resources

- MoQ Project: https://github.com/kixelated/moq
- WebTransport Spec: https://w3c.github.io/webtransport/
- QUIC Protocol: https://quicwg.org/

## Support

For issues:
1. Check MoQ relay logs
2. Verify network connectivity
3. Test with public relay first
4. Open issue at https://github.com/k0sti/cw-audio/issues
