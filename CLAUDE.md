# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Metis is a high-performance Model Context Protocol (MCP) mock server built in Rust. It provides 9 mock strategies, AI agent capabilities, database persistence with git-style version history, and a full Leptos-based web UI.

## Build Commands

```bash
# Build (debug)
cargo build

# Build (release)
cargo build --release

# Build UI only
cargo build --package metis-ui

# Run server (default: http://127.0.0.1:3000)
cargo run

# Run with logging
RUST_LOG=info cargo run
```

## Testing

```bash
# Run all tests
cargo test

# Run unit tests only
cargo test --lib

# Run specific integration test
cargo test --test api_integration_test

# Run with output
RUST_LOG=debug cargo test -- --nocapture

# Run benchmarks
cargo bench
```

## Code Quality

```bash
cargo fmt      # Format code
cargo clippy   # Lint
cargo check    # Type check
```

## Architecture

**Hexagonal Architecture (Ports & Adapters):**

```
src/
├── adapters/           # Infrastructure layer
│   ├── api_handler.rs      # REST API for Web UI
│   ├── rmcp_server.rs      # MCP protocol implementation
│   ├── mock_strategy.rs    # 9 mock strategy execution engine
│   ├── workflow_engine.rs  # Multi-step workflow execution
│   └── auth_middleware.rs  # Authentication (API Key, JWT, Basic, OAuth2)
├── agents/             # AI Agent subsystem
│   ├── core/               # Agent types: single_turn, multi_turn, react
│   ├── llm/                # LLM providers: OpenAI, Anthropic, Gemini, Ollama, Azure
│   └── memory/             # Memory backends: in_memory, file, database
├── config/             # Configuration with hot reload, S3 support
├── persistence/        # SQLx-based storage (SQLite, PostgreSQL, MySQL)
└── main.rs             # Entry point
```

**UI Subproject (Leptos CSR):**
```
ui/
├── src/
│   ├── api.rs          # API client bindings
│   ├── components/     # Dashboard, config editor, CRUD for all archetypes
│   └── types.rs        # Shared type definitions
```

## Key Patterns

- **Arc<RwLock<Settings>>** - Shared mutable state for live config reload
- **Strategy Pattern** - 9 mock strategies (Template, Random, Database, LLM, Script, etc.)
- **Repository Pattern** - Data persistence abstraction via DataStore
- **async-trait** - Protocol handlers are async traits

## Configuration Precedence (low to high)

1. Environment variables (`METIS_*`, `AWS_*`)
2. Local config file (`metis.toml`)
3. CLI arguments
4. S3 configuration (with live reload)
5. UI configuration (in-memory)

## Mock Strategies

Static, Template (Tera), Random (Faker), Stateful, Script (Rhai/Lua/JS/Python), File, Pattern, LLM (OpenAI/Anthropic), Database (SQLx)

## CLI Subcommands

```bash
metis encrypt-secret "value" -p "passphrase"  # AGE encryption
metis migrate --db-url sqlite://metis.db      # Run migrations
metis export --output config.json             # Export config
metis version-list                            # Version history
metis rollback --commit abc123                # Rollback to commit
```

## Adding Features

**New Mock Strategy:**
1. Add to `src/adapters/mock_strategy.rs`
2. Add config struct to `src/config/schema.rs`
3. Update `MockStrategyHandler::execute()`

**New API Endpoint:**
1. Add handler in `src/adapters/api_handler.rs`
2. Register route in `src/lib.rs` (`create_app()`)
3. Add UI component in `ui/src/components/`

## Technology Stack

- **Runtime:** Tokio, Axum 0.7
- **Protocol:** rmcp 0.9 SDK (official MCP)
- **Database:** SQLx 0.8 (SQLite, PostgreSQL, MySQL)
- **Templates:** Tera
- **Scripting:** Rhai, mlua (Lua 5.4), boa_engine (JS), rustpython-vm
- **UI:** Leptos 0.8 (CSR), wasm-bindgen, gloo-net
- **Encryption:** age (AGE passphrase encryption)
