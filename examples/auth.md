# Authentication Examples

## API Key Authentication

```toml
[auth]
enabled = true
mode = "ApiKey"
api_keys = ["your-api-key-here", "another-key"]
```

**Usage:**
```bash
curl -H "x-api-key: your-api-key-here" http://localhost:3000/mcp
```

---

## JWT Bearer Token Authentication

```toml
[auth]
enabled = true
mode = "BearerToken"
jwt_secret = "your-secret-key"
jwt_algorithm = "HS256"  # or "HS384", "HS512"
```

**Usage:**
```bash
# Generate a JWT token (example using jwt.io or a library)
TOKEN="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."

curl -H "Authorization: Bearer $TOKEN" http://localhost:3000/mcp
```

---

## No Authentication (Development)

```toml
[auth]
enabled = false
```

**Usage:**
```bash
curl http://localhost:3000/mcp
```

---

## Complete Example with Auth

```toml
[server]
host = "0.0.0.0"
port = 3000

[auth]
enabled = true
mode = "ApiKey"
api_keys = ["dev-key-123"]

[[resources]]
uri = "secure://data"
name = "Secure Data"
content = "This requires authentication"
```

**Test:**
```bash
# Without auth - should fail
curl http://localhost:3000/mcp

# With auth - should succeed
curl -H "x-api-key: dev-key-123" http://localhost:3000/mcp
```
