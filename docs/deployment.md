# 64cam Deployment Guide

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Quick Start — Single Host](#quick-start--single-host)
3. [Production Multi-Node Deployment](#production-multi-node-deployment)
4. [Raspberry Pi / Single Machine (All-in-One)](#raspberry-pi--single-machine-all-in-one)
5. [TLS with Caddy](#tls-with-caddy)
6. [Adding More Nodes](#adding-more-nodes)
7. [S3 / MinIO Tiered Storage](#s3--minio-tiered-storage)
8. [Environment Variable Reference](#environment-variable-reference)
9. [Upgrading](#upgrading)

---

## Prerequisites

| Requirement | Minimum | Notes |
|-------------|---------|-------|
| Docker Engine | 24+ | Older versions lack BuildKit features used by the Dockerfiles |
| Docker Compose | v2.20+ | Uses Compose v2 syntax (`docker compose`, not `docker-compose`) |
| Architecture | amd64 or arm64 | Pre-built images available for both; see [Multi-Arch Images](#multi-arch-images) |
| OS | Linux (production) | Windows/macOS supported for development; see [Platform Limitations](limitations.md) |
| RAM | 1 GB coordinator + 512 MB/node | PostgreSQL needs ~256 MB on top |
| Disk | 20 GB+ per node | Recordings consume roughly 1–3 GB per camera per day at 1080p H.264 |

---

## Quick Start — Single Host

For development or a quick evaluation with everything on one machine:

```bash
git clone https://github.com/your-org/64cam.git
cd 64cam

# Generate a strong secret (required)
export JWT_SECRET=$(openssl rand -base64 48)

docker compose up -d
```

Access the UI at **http://localhost:8080**.

Default credentials (set in migration `0010_bootstrap.sql`): `admin@localhost` / `admin`.
**Change the password immediately after first login.**

---

## Production Multi-Node Deployment

### Step 1 — Create a `.env` file

```bash
cp .env.node.example .env          # start from the node template as a reference
# Edit .env — at minimum set JWT_SECRET and DB_PASSWORD
JWT_SECRET=$(openssl rand -base64 48)
DB_PASSWORD=$(openssl rand -base64 24)
```

### Step 2 — Start coordinator and first node

```bash
docker compose --env-file .env up -d
```

### Step 3 — Enable TLS (recommended for production)

See [TLS with Caddy](#tls-with-caddy).

### Step 4 — Add a second node

See [Adding More Nodes](#adding-more-nodes).

---

## Raspberry Pi / Single Machine (All-in-One)

`docker-compose.single.yml` runs coordinator, node, PostgreSQL, and Caddy
on a single machine. It is tuned for Raspberry Pi 4 / 5 (arm64):

- PostgreSQL configured for low RAM (`shared_buffers=128MB`)
- Node uses **host networking** so ONVIF discovery and RTSP pull work on the
  local network without port-forwarding gymnastics
- Caddy provides automatic HTTPS

```bash
cp .env.single.example .env.single
# Edit .env.single — set JWT_SECRET and DOMAIN at minimum
nano .env.single

docker compose -f docker-compose.single.yml --env-file .env.single up -d
```

### Raspberry Pi OS setup

```bash
# Install Docker
curl -fsSL https://get.docker.com | sh
sudo usermod -aG docker $USER
newgrp docker

# Enable Docker on boot
sudo systemctl enable --now docker

# Optional: expand swap for smoother compilation (if building locally)
sudo dphys-swapfile swapoff
sudo sed -i 's/CONF_SWAPSIZE=.*/CONF_SWAPSIZE=2048/' /etc/dphys-swapfile
sudo dphys-swapfile setup
sudo dphys-swapfile swapon
```

### Pre-built arm64 images

To skip building on the Pi (strongly recommended — Rust compilation takes ~20 min
on Pi 4), pull the pre-built images from GHCR:

```yaml
# In docker-compose.single.yml, replace `build:` blocks with:
coordinator:
  image: ghcr.io/your-org/64cam-coordinator:latest
  # ... rest of service config unchanged

node:
  image: ghcr.io/your-org/64cam-node:latest
  # ...
```

---

## TLS with Caddy

### With a real domain (Let's Encrypt)

Port 80 and 443 must be reachable from the internet, or you must configure a
DNS-01 challenge. [Caddy docs for DNS providers](https://caddyserver.com/docs/automatic-https).

```bash
DOMAIN=nvr.example.com JWT_SECRET=... docker compose --profile tls up -d
```

Caddy automatically fetches and renews the Let's Encrypt certificate.

### LAN / self-signed

For LAN-only deployments, set `DOMAIN` to your machine's LAN IP or a local
hostname. Caddy generates a self-signed certificate. Browsers will show a
security warning — add the certificate to your device's trust store to
silence it (see Caddy docs on [local HTTPS](https://caddyserver.com/docs/automatic-https#local-https)).

```bash
DOMAIN=192.168.1.10 JWT_SECRET=... docker compose --profile tls up -d
```

### Let's Encrypt staging (testing)

To avoid Let's Encrypt rate limits while testing, uncomment the `acme_ca`
line in `docker/caddy/Caddyfile`:

```
{
    acme_ca https://acme-staging-v02.api.letsencrypt.org/directory
}
```

---

## Adding More Nodes

### Option A: Pre-provisioned (recommended)

An admin creates the node record before the physical machine boots. The node
joins immediately as `approved` — no approval wait.

```bash
# 1. Create the node via the API (or UI)
curl -X POST https://nvr.example.com/api/v1/nodes \
  -H "Authorization: Bearer <admin-jwt>" \
  -H "Content-Type: application/json" \
  -d '{"name": "garage"}'
# Response: { "node": { "id": "...", ... }, "api_key": "..." }

# 2. On the new machine, create .env.node
cat > .env.node <<EOF
COORDINATOR_URL=https://nvr.example.com
NODE_ID=<id from step 1>
NODE_API_KEY=<api_key from step 1>
NODE_NAME=garage
DATA_DIR=/data
EOF

# 3. Start the node
docker compose -f docker-compose.node.yml --env-file .env.node up -d
```

### Option B: Self-registration

Leave `NODE_ID` and `NODE_API_KEY` blank. The node registers itself on boot
with `status=pending`. An admin must approve it:

```bash
# After the node boots and logs "registered as <id>"
curl -X PATCH https://nvr.example.com/api/v1/nodes/<id>/status \
  -H "Authorization: Bearer <admin-jwt>" \
  -H "Content-Type: application/json" \
  -d '{"status": "approved"}'
```

### Node capacity

Each node accepts up to **64 cameras**. Check remaining capacity:

```bash
curl https://nvr.example.com/api/v1/nodes/<id>/capacity \
  -H "Authorization: Bearer <jwt>"
# { "node_id": "...", "camera_count": 12, "max_cameras": 64, "headroom": 52 }
```

When creating a camera, omit `node_id` to auto-assign to the approved node
with the most headroom:

```bash
curl -X POST https://nvr.example.com/api/v1/cameras \
  -H "Authorization: Bearer <jwt>" \
  -d '{"name": "front-door", "rtsp_url": "rtsp://..."}'
```

---

## S3 / MinIO Tiered Storage

Segments older than `S3_MIGRATE_AFTER_HOURS` (default 2) are automatically
uploaded to S3 and deleted from the node's local disk. The coordinator serves
migrated segments via presigned redirect — browsers download directly from S3.

### MinIO (self-hosted)

```yaml
# Add to docker-compose.yml
minio:
  image: minio/minio:latest
  command: server /data --console-address ":9001"
  environment:
    MINIO_ROOT_USER:     minioadmin
    MINIO_ROOT_PASSWORD: minioadmin
  ports:
    - "9000:9000"
    - "9001:9001"
  volumes:
    - minio_data:/data
```

Then set in `.env` / `.env.node`:

```
S3_ENDPOINT=http://minio:9000
S3_BUCKET=64cam-recordings
S3_KEY_ID=minioadmin
S3_SECRET_KEY=minioadmin
S3_REGION=us-east-1
S3_PATH_STYLE=true
S3_MIGRATE_AFTER_HOURS=2
```

Create the bucket before starting nodes:

```bash
docker run --rm --network <compose-network> \
  minio/mc:latest mc mb minio/64cam-recordings
```

### AWS S3

```
S3_ENDPOINT=https://s3.amazonaws.com
S3_BUCKET=my-64cam-recordings
S3_KEY_ID=AKIA...
S3_SECRET_KEY=...
S3_REGION=us-east-1
S3_PATH_STYLE=false
```

The coordinator also needs S3 credentials to generate presigned GET URLs.
Set the same `S3_*` variables in the coordinator's environment.

**Note:** FFmpeg clip export (`/export`) does not work for segments that have
been migrated to S3 — the node's local files are gone. Recordings still on
local disk export normally. A future phase will add S3-aware clip export.

---

## Multi-Arch Images

Pre-built images for `linux/amd64` and `linux/arm64` are published to GHCR
on every push to `main` and on version tags:

```
ghcr.io/your-org/64cam-coordinator:latest
ghcr.io/your-org/64cam-coordinator:1.2.3
ghcr.io/your-org/64cam-node:latest
ghcr.io/your-org/64cam-node:1.2.3
```

Docker pulls the correct variant for your machine automatically.

### Building locally (multi-arch)

```bash
# One-time setup
docker buildx create --use --name multiarch

# Build and push coordinator
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  --file docker/coordinator/Dockerfile \
  --tag ghcr.io/your-org/64cam-coordinator:dev \
  --push .

# Build and push node
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  --file docker/node/Dockerfile \
  --tag ghcr.io/your-org/64cam-node:dev \
  --push .
```

Cross-compilation is done on the build host (no QEMU for the Rust stage):
`amd64` host builds `arm64` binaries in ~5 minutes using the
`aarch64-linux-gnu-gcc` cross-linker.

---

## Environment Variable Reference

### Coordinator

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | — | PostgreSQL connection string (required) |
| `JWT_SECRET` | — | HS256 signing key, ≥32 bytes (required) |
| `BIND_ADDR` | `0.0.0.0:8080` | Listen address |
| `FRONTEND_DIR` | `./frontend/dist` | Path to pre-built React SPA |
| `RUST_LOG` | `info` | Tracing log level |
| `S3_ENDPOINT` | — | S3 endpoint for presigned redirect (optional) |
| `S3_KEY_ID` | — | S3 access key ID |
| `S3_SECRET_KEY` | — | S3 secret key |
| `S3_REGION` | `us-east-1` | S3 region |
| `S3_PATH_STYLE` | `true` | Use path-style S3 URLs (MinIO) |

### Node

| Variable | Default | Description |
|----------|---------|-------------|
| `COORDINATOR_URL` | — | HTTP URL of the coordinator (required) |
| `DATA_DIR` | `/data` | Recordings, state.json, MediaMTX config |
| `NODE_ID` | — | Pre-provisioned UUID (optional; triggers approved join) |
| `NODE_API_KEY` | — | API key for `NODE_ID` (required when `NODE_ID` is set) |
| `NODE_NAME` | hostname | Display name shown in the UI |
| `NODE_HTTP_PORT` | `8890` | Playback HTTP server port |
| `MEDIAMTX_BIN` | `/usr/local/bin/mediamtx` | Path to MediaMTX binary |
| `MEDIAMTX_API_PORT` | `9997` | MediaMTX HTTP API port |
| `MEDIAMTX_RTSP_PORT` | `8554` | RTSP listen port |
| `RUST_LOG` | `info` | Tracing log level |
| `S3_ENDPOINT` | — | S3 endpoint for segment migration (optional) |
| `S3_BUCKET` | — | Bucket name (required when S3_ENDPOINT is set) |
| `S3_KEY_ID` | — | S3 access key ID |
| `S3_SECRET_KEY` | — | S3 secret key |
| `S3_REGION` | `us-east-1` | S3 region |
| `S3_PATH_STYLE` | `true` | Use path-style S3 URLs (MinIO) |
| `S3_MIGRATE_AFTER_HOURS` | `2` | Hours before a segment is eligible for migration |

### Caddy (TLS profile)

| Variable | Default | Description |
|----------|---------|-------------|
| `DOMAIN` | `localhost` | Domain or IP for TLS provisioning |

---

## Upgrading

```bash
# Pull latest images
docker compose pull

# Restart services (coordinator runs migrations on startup)
docker compose up -d --remove-orphans
```

Migrations are idempotent. The `migrate` service applies any new SQL files
in `migrations/` before the coordinator starts.
