# Metis MCP Mock Server

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**Metis** is a high-performance, fully configurable [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) mock server written in Rust. It provides comprehensive mocking capabilities for resources, tools, and prompts with support for multiple data generation strategies, making it ideal for local development, testing, and prototyping of MCP-based applications.

## 🌟 Features

### Core Capabilities
- **Full MCP Protocol Support**: Complete implementation of the Model Context Protocol specification
- **Multiple Mock Strategies**: Template-based (Tera), Random (Faker), and extensible strategy system
- **Declarative Configuration**: Simple TOML-based configuration with hot-reload support
- **Configuration Hot-Reload**: Automatically reloads configuration changes without restarting the server
- **MCP Logging Support**: Captures and handles client log messages via the MCP protocol
- **High Performance**: Built with Rust and Tokio for async, high-throughput operations
- **Hexagonal Architecture**: Clean separation of concerns following SOLID principles

### Mock Strategies
- **Template Strategy**: Use Tera templates for dynamic content generation
- **Random Strategy**: Generate realistic fake data using the Faker library
- **Extensible**: Easy to add custom mock strategies

### MCP Components
- **Resources**: Mock file systems, databases, APIs, and other data sources
- **Tools**: Simulate tool execution with configurable responses
- **Prompts**: Provide templated prompts for testing LLM interactions

## 🏗️ Architecture

Metis follows **Hexagonal Architecture** (Ports and Adapters) combined with **SOLID principles** to ensure maintainability, testability, and extensibility.

```
┌─────────────────────────────────────────────────────────────┐
│                     Metis MCP Mock Server                    │
├─────────────────────────────────────────────────────────────┤
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

### Project Structure

```
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

**Random Strategy Options:**
- `faker_type`: Type of fake data to generate (e.g., "name", "email", "sentence", "paragraph")

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

### Current Status (v0.1.0)
- ✅ Core MCP protocol implementation
- ✅ Template and Random mock strategies
- ✅ Resource, Tool, and Prompt handlers
- ✅ TOML-based configuration
- ✅ Basic test coverage

### Planned Features
- [ ] Additional mock strategies (LLM, Script, Database, File)
- [ ] Authentication and authorization
- [x] Configuration hot-reload
- [ ] Workflow engine
- [ ] Web UI for configuration management
- [ ] Multi-language scripting support (Python, Lua, Rhai)
- [ ] Advanced observability (metrics, tracing)
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
