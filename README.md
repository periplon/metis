```
│                                                               │
│  ┌─────────────────┐         ┌──────────────────┐          │
│  │  Config Loader  │────────▶│  Config Manager  │          │
│  │  (File Watcher) │         │  (Live Reload)   │          │
│  └─────────────────┘         └──────────────────┘          │
│                                        │                     │
│  ┌─────────────────────────────────────┼──────────────────┐│
│  │              MCP Protocol Layer      │                  ││
│  │  ┌────────────────────────────────────────────────┐   ││
│  │  │         JSON-RPC 2.0 Handler                   │   ││
│  │  └────────────────────────────────────────────────┘   ││
│  └─────────────────────────────────────┬──────────────────┘│
│                                        │                     │
│  ┌────────────┬─────────────┬─────────┼──────────────┐    │
│  │  Resource  │    Tool     │  Prompt │  State       │    │
│  │  Handler   │   Handler   │ Handler │  Manager     │    │
│  └────────────┴─────────────┴─────────┴──────────────┘    │
│         │             │            │           │            │
│  ┌──────┼─────────────┼────────────┼───────────┼──────┐   │
│  │      │     Mock Data Generation Layer       │      │   │
│  │  ┌───▼──┐  ┌───▼───┐  ┌────▼────┐          │      │   │
│  │  │Random│  │Template│  │ Custom  │          │      │   │
│  │  └──────┘  └────────┘  └─────────┘          │      │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```
```

### Project Structure

```text
metis/
├── src/
│   ├── adapters/           # Infrastructure layer (Ports & Adapters)
│   │   ├── mcp_protocol_handler.rs
│   │   ├── resource_handler.rs
│   │   ├── tool_handler.rs
│   │   ├── prompt_handler.rs
│   │   ├── mock_strategy.rs
│   │   └── state_manager.rs
│   ├── application/        # Application layer
│   │   └── mod.rs
│   ├── domain/            # Domain layer (Business logic)
│   │   └── mcp_types.rs
│   ├── config/            # Configuration management
│   │   └── mod.rs
│   ├── lib.rs
│   └── main.rs
├── tests/                 # Integration tests
│   └── api_integration_test.rs
├── metis.toml            # Configuration file
└── Cargo.toml
```

## ✨ Key Features

### 🛠️ 9 Powerful Mock Strategies
- **Static**: Return fixed null/empty responses
- **Template**: Dynamic responses using Tera templates
- **Random**: Generate realistic fake data (names, emails, etc.)
- **Stateful**: Get, set, and increment state variables
- **Script**: Execute scripts in Rhai, Lua, JavaScript, or Python
- **File**: Serve content from local files (sequential or random selection)
- **Pattern**: Generate strings from regex-like patterns
- **LLM**: Proxy to OpenAI or Anthropic for AI-generated mocks
- **Database**: Execute SQL queries against real databases

### 🛡️ Reliability & Observability
- **Rate Limiting**: Built-in token bucket rate limiter
- **Health Checks**: Kubernetes-ready endpoints (`/health/live`, `/health/ready`)
- **Metrics**: Prometheus-compatible metrics endpoint (`/metrics`)
- **Hot Reload**: Zero-downtime configuration updates

### 💻 Developer Experience
- **Web UI**: Basic dashboard for monitoring
- **CLI**: Full-featured command-line interface with environment variable support
- **TOML Config**: Human-readable configuration
- **S3 Config**: Load configuration from S3 buckets with live reload
- **Validation**: Startup configuration checks

## 🚀 Quick Start

### Prerequisites

- Rust 1.75 or higher
- Cargo (comes with Rust)

### Installation

1. Clone the repository:
```bash
git clone https://github.com/yourusername/metis.git
cd metis
```

2. Build the project:
```bash
cargo build --release
```

3. Run the server:
```bash
cargo run
```

The server will start on `http://127.0.0.1:3000` by default.

### Command Line Options

```bash
metis [OPTIONS]

Options:
  -c, --config <CONFIG>           Path to config file [default: metis.toml]
      --host <HOST>               Server host address
      --port <PORT>               Server port
      --s3-enabled                Enable S3 configuration source
      --s3-bucket <BUCKET>        S3 bucket name for configuration files
      --s3-prefix <PREFIX>        S3 key prefix (e.g., "config/")
      --s3-region <REGION>        AWS region for S3
      --s3-endpoint <ENDPOINT>    S3 endpoint URL (for MinIO/LocalStack)
      --s3-poll-interval <SECS>   S3 polling interval in seconds
  -h, --help                      Print help
  -V, --version                   Print version
```

All CLI options can also be set via environment variables:
- `METIS_CONFIG`, `METIS_HOST`, `METIS_PORT`
- `METIS_S3_ENABLED`, `METIS_S3_BUCKET`, `METIS_S3_PREFIX`
- `METIS_S3_REGION`, `METIS_S3_ENDPOINT`, `METIS_S3_POLL_INTERVAL`

### Basic Usage

1. **Configure your mocks** in `metis.toml`:

```toml
[server]
host = "127.0.0.1"
port = 3000

# Define a resource with template strategy
[[resources]]
uri = "file:///greeting.txt"
name = "Greeting Template"
description = "A personalized greeting"
mime_type = "text/plain"

[resources.mock]
strategy = "template"
template = "Hello, {{ name }}! Welcome to Metis."

# Define a resource with random strategy
[[resources]]
uri = "file:///user.json"
name = "Random User"
description = "Generate random user data"
mime_type = "application/json"

[resources.mock]
strategy = "random"
faker_type = "name"

# Define a tool
[[tools]]
name = "calculate"
description = "Performs a calculation"
input_schema = { type = "object", properties = { operation = { type = "string" } } }

[tools.mock]
strategy = "template"
template = "{\"result\": 42}"
```

2. **Connect your MCP client** to `http://127.0.0.1:3000`

3. **Test the server**:
```bash
# Run tests
cargo test

# Run with logging
RUST_LOG=info cargo run
```

## 📝 Configuration

### Server Configuration

```toml
[server]
host = "127.0.0.1"  # Server host
port = 3000         # Server port
```

### S3 Configuration (Optional)

Load configuration files from an S3 bucket with automatic live reload:

```toml
[s3]
enabled = true
bucket = "my-metis-config"
prefix = "config/"              # Optional: key prefix
region = "us-east-1"            # Optional: AWS region
endpoint = "http://localhost:9000"  # Optional: for MinIO/LocalStack
poll_interval_secs = 30         # Default: 30 seconds
```

S3 configuration can also be provided via CLI or environment variables:

```bash
# Via CLI
metis --s3-enabled --s3-bucket my-bucket --s3-prefix config/

# Via environment variables
export METIS_S3_ENABLED=true
export METIS_S3_BUCKET=my-bucket
export METIS_S3_PREFIX=config/
export METIS_S3_REGION=us-east-1
metis
```

**Configuration Precedence:** CLI > Environment Variables > Config File > Defaults

### Authentication Configuration (Optional)

Metis supports multiple authentication modes:

```toml
[auth]
enabled = true
mode = "api_key"  # Options: "none", "api_key", "bearer_token", "basic_auth", "oauth2"

# For API Key authentication
api_keys = ["key-1", "key-2", "key-3"]

# For JWT Bearer Token authentication
jwt_secret = "your-secret-key"
jwt_algorithm = "HS256"  # HS256, HS384, HS512

# For Basic Auth
[auth.basic_users]
admin = "password123"
user = "userpass"

# For OAuth2/JWKS authentication
jwks_url = "https://your-idp.com/.well-known/jwks.json"
```

**Authentication Modes:**
- `none`: No authentication required (default)
- `api_key`: Requires `X-API-Key` header with valid key
- `bearer_token`: Requires `Authorization: Bearer <jwt>` header
- `basic_auth`: Requires `Authorization: Basic <base64>` header
- `oauth2`: Validates JWT tokens against JWKS endpoint

### Resource Configuration

Resources represent data sources that can be accessed via the MCP protocol.

```toml
[[resources]]
uri = "file:///path/to/resource"
name = "Resource Name"
description = "Resource description"
mime_type = "text/plain"

[resources.mock]
strategy = "template"  # or "random"
template = "Your template content here with {{ variables }}"
```

**Available Strategies:**
- `template`: Use Tera templates with variable substitution
- `random`: Generate random data using Faker library
- `static`: Return null/empty response
- `stateful`: Persist and retrieve state across requests
- `script`: Execute scripts (Rhai, Lua, JavaScript, Python)
- `file`: Read from external files
- `pattern`: Generate data from patterns
- `llm`: Generate content using OpenAI/Anthropic
- `database`: Query SQL databases

**Random Strategy Options:**
- `faker_type`: Type of fake data to generate (e.g., "name", "email", "sentence", "paragraph")

**File Strategy Options:**
```toml
[resources.mock]
strategy = "file"
[resources.mock.file]
path = "data/users.json"  # JSON array, JSON Lines, or raw text
selection = "sequential"   # "random", "sequential", "first", "last"
```

**Pattern Strategy Options:**
```toml
[resources.mock]
strategy = "pattern"
pattern = "ID-\\d\\d\\d\\d"  # Generate strings like "ID-1234"
```

Pattern syntax:
- `\d` - random digit (0-9)
- `\a` - random lowercase letter (a-z)
- `\A` - random uppercase letter (A-Z)
- `\w` - random word character (a-zA-Z0-9_)
- `\x` - random hex digit (0-9a-f)
- `[abc]` - one of the characters
- `[a-z]` - character from range
- `{n}` - repeat previous n times
- `{n,m}` - repeat previous n to m times

### Tool Configuration

Tools represent executable functions that can be called via the MCP protocol.

```toml
[[tools]]
name = "tool_name"
description = "Tool description"
input_schema = { type = "object", properties = { param = { type = "string" } } }

[tools.mock]
strategy = "template"
template = "{\"status\": \"success\", \"data\": \"{{ input }}\"}"
```

### Prompt Configuration

Prompts provide templated text for LLM interactions.

```toml
[[prompts]]
name = "code_review"
description = "Generate code review prompt"
arguments = [
    { name = "language", description = "Programming language", required = true }
]

[prompts.mock]
template = "Review this {{ language }} code for best practices."
```

## 🧪 Testing

### Run All Tests
```bash
cargo test
```

### Run Specific Tests
```bash
# Unit tests
cargo test --lib

# Integration tests
cargo test --test api_integration_test
```

### Run with Logging
```bash
RUST_LOG=debug cargo test -- --nocapture
```

## 🔧 Development

### Building from Source

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release
```

### Code Quality

```bash
# Format code
cargo fmt

# Lint code
cargo clippy

# Check for issues
cargo check
```

### Project Guidelines

- **Architecture**: Follow Hexagonal Architecture principles
- **SOLID Principles**: Maintain single responsibility, open/closed, and dependency inversion
- **Testing**: Aim for >80% test coverage
- **Documentation**: Document all public APIs and complex logic

## 📚 MCP Protocol Support

Metis implements the following MCP protocol methods:

### Initialization
- `initialize`: Handshake and capability negotiation
- `notifications/initialized`: Confirm initialization complete
- `notifications/message`: Handle client log messages

### Resources
- `resources/list`: List all available resources
- `resources/read`: Read resource content

### Tools
- `tools/list`: List all available tools
- `tools/call`: Execute a tool

### Prompts
- `prompts/list`: List all available prompts
- `prompts/get`: Get a specific prompt

### Utilities
- `ping`: Health check endpoint

## 🛣️ Roadmap

### Current Status (Under Development)
- ✅ Core MCP protocol implementation
- ✅ Mock Strategies:
    - ✅ Template
    - ✅ Random
    - ✅ Static
    - ✅ Stateful
    - ✅ Database (SQLx)
    - ✅ LLM (OpenAI/Anthropic)
    - ✅ Script (Rhai, Lua, JavaScript, Python)
    - ✅ File (JSON, JSON Lines, raw text; random/sequential/first/last selection)
    - ✅ Pattern (regex-like patterns with character classes and repetition)
- ✅ Resource, Tool, and Prompt handlers
- ✅ TOML-based configuration with Hot Reload
- ✅ S3 Configuration with Live Reload
- ✅ CLI with Environment Variable Support
- ✅ Health Checks & Prometheus Metrics
- ✅ Rate Limiting
- ✅ Basic Web UI (Embedded)
- ✅ Authentication (API Key, JWT Bearer, Basic Auth, OAuth2/JWKS)

### Planned Features
- [ ] Advanced Workflow engine
- [ ] Enhanced Web UI for configuration management
- [ ] Performance optimizations (>10k req/s)

See [metis-implementation-plan.md](metis-implementation-plan.md) for detailed roadmap.

## 🤝 Contributing

Contributions are welcome! Please follow these guidelines:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Development Setup

1. Install Rust: https://rustup.rs/
2. Clone the repository
3. Run `cargo build` to verify setup
4. Run `cargo test` to ensure all tests pass

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [Model Context Protocol](https://modelcontextprotocol.io/) - Protocol specification
- [Tokio](https://tokio.rs/) - Async runtime
- [Axum](https://github.com/tokio-rs/axum) - Web framework
- [Tera](https://tera.netlify.app/) - Template engine
- [Fake](https://github.com/cksac/fake-rs) - Fake data generation

## 📞 Support

- **Issues**: [GitHub Issues](https://github.com/yourusername/metis/issues)
- **Discussions**: [GitHub Discussions](https://github.com/yourusername/metis/discussions)
- **Documentation**: [Implementation Plan](metis-implementation-plan.md)

## 🔗 Related Projects

- [MCP Specification](https://modelcontextprotocol.io/)
- [MCP TypeScript SDK](https://github.com/modelcontextprotocol/typescript-sdk)
- [MCP Python SDK](https://github.com/modelcontextprotocol/python-sdk)

---

**Built with ❤️ using Rust**
