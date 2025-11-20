# Docker Deployment Guide

## Quick Start

### Using Docker Compose (Recommended)

1. **Start Metis**:
```bash
docker-compose up -d
```

2. **Check status**:
```bash
docker-compose ps
docker-compose logs -f metis
```

3. **Test the server**:
```bash
curl http://localhost:3000/health
curl http://localhost:3000/metrics
```

4. **Stop**:
```bash
docker-compose down
```

---

## Configuration

### Custom Configuration

Create your own `config/metis.toml`:

```toml
[server]
host = "0.0.0.0"
port = 3000

[[resources]]
uri = "file:///myresource"
name = "My Resource"
content = "Custom content"
```

The `config/` directory is mounted as a volume, so changes are reflected immediately with live reload.

### Data Files

For file-based strategies, place your JSON files in the `data/` directory:

```bash
mkdir -p data
echo '[{"id": 1, "name": "Item 1"}]' > data/items.json
```

Reference in config:
```toml
[[tools]]
name = "get_item"
[tools.mock]
strategy = "file"
[tools.mock.file]
path = "/app/data/items.json"
selection = "random"
```

---

## Using Docker Directly

### Build Image

```bash
docker build -t metis:latest .
```

### Run Container

```bash
docker run -d \
  --name metis \
  -p 3000:3000 \
  -v $(pwd)/config:/app/config:ro \
  -v $(pwd)/data:/app/data:ro \
  -e RUST_LOG=info \
  metis:latest
```

### View Logs

```bash
docker logs -f metis
```

### Stop Container

```bash
docker stop metis
docker rm metis
```

---

## Monitoring Stack

Start Metis with Prometheus and Grafana:

```bash
docker-compose --profile monitoring up -d
```

Access:
- **Metis**: http://localhost:3000
- **Prometheus**: http://localhost:9090
- **Grafana**: http://localhost:3001 (admin/admin)

### Grafana Setup

1. Add Prometheus data source:
   - URL: `http://prometheus:9090`
2. Import dashboard or create custom queries:
   - `metis_requests_total`
   - `metis_request_duration_seconds`
   - `metis_strategy_executions_total`

---

## Health Checks

The container includes health checks:

```bash
# Check health status
docker inspect --format='{{.State.Health.Status}}' metis

# View health check logs
docker inspect --format='{{range .State.Health.Log}}{{.Output}}{{end}}' metis
```

---

## Production Deployment

### Environment Variables

```bash
docker run -d \
  --name metis \
  -p 3000:3000 \
  -e RUST_LOG=warn \
  -e RUST_BACKTRACE=0 \
  --restart unless-stopped \
  metis:latest
```

### Resource Limits

```yaml
services:
  metis:
    deploy:
      resources:
        limits:
          cpus: '1'
          memory: 512M
        reservations:
          cpus: '0.5'
          memory: 256M
```

### Security

Run as non-root user (already configured in Dockerfile):
```dockerfile
USER metis
```

---

## Troubleshooting

### Container won't start

```bash
# Check logs
docker-compose logs metis

# Check configuration
docker-compose config
```

### Health check failing

```bash
# Test health endpoint manually
docker exec metis curl http://localhost:3000/health/live

# Check if port is accessible
curl http://localhost:3000/health
```

### Configuration not loading

```bash
# Verify volume mount
docker inspect metis | grep Mounts -A 10

# Check file permissions
ls -la config/
```

---

## Image Size Optimization

Current image size: ~50-100 MB (multi-stage build)

To further optimize:
```dockerfile
# Use alpine instead of debian
FROM alpine:latest
RUN apk add --no-cache ca-certificates curl
```

---

## Multi-Architecture Builds

Build for multiple platforms:

```bash
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -t metis:latest \
  --push .
```

---

## Docker Hub Publishing

```bash
# Tag image
docker tag metis:latest yourusername/metis:v1.0.0
docker tag metis:latest yourusername/metis:latest

# Push to Docker Hub
docker push yourusername/metis:v1.0.0
docker push yourusername/metis:latest
```
