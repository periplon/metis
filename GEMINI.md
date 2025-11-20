# Metis: MCP Mock Server

## Project Overview
Metis is a robust mock server for the **Model Context Protocol (MCP)**. It is designed to help developers simulate MCP resources, tools, and prompts using various dynamic strategies. It acts as a versatile backend for testing MCP clients without needing live connections to real LLMs or production services.

**Core Value:** Enables offline, deterministic, and edge-case testing for MCP-compliant applications.

## Tech Stack
*   **Language:** Rust (Edition 2021)
*   **Web Framework:** Axum (Backend)
*   **Frontend:** Leptos (Rust-based Web Assembly framework, CSR)
*   **Async Runtime:** Tokio
*   **Configuration:** TOML (with hot reload)
*   **Key Libraries:**
    *   `tera`: Templating
    *   `rhai`: Scripting
    *   `fake` + `rand`: Random data generation
    *   `sqlx`: Database interactions
    *   `async-openai`: LLM proxying
    *   `tracing`: Observability

## Project Structure
*   **`src/`**: Main backend source code.
    *   `adapters/`: Implementation of protocol handlers and strategies (Ports & Adapters pattern).
    *   `application/`: Core application logic.
    *   `domain/`: Type definitions and business logic.
    *   `config/`: Configuration loading and watching.
*   **`ui/`**: Leptos-based frontend application.
*   **`tests/`**: Integration tests.
*   **`benches/`**: Performance benchmarks.
*   **`examples/`**: Example configuration files (`.toml`) and data.
*   **`metis.toml`**: Main configuration file.

## Key Features & Strategies
Metis supports 9 mock strategies:
1.  **Static**: Returns fixed data.
2.  **Template**: Tera templates for dynamic text.
3.  **Random**: Faker-based random data.
4.  **Stateful**: Memory-persistent state (counters, toggles).
5.  **Script**: Rhai scripts for complex logic.
6.  **File**: Serves content from local files.
7.  **Pattern**: Regex-like string generation.
8.  **LLM**: Proxies to OpenAI/Anthropic.
9.  **Database**: SQL queries (Postgres, MySQL, SQLite).

## Development & Usage

### Prerequisites
*   Rust 1.75+
*   `just` (Command runner)
*   `cargo-leptos` (for UI development: `cargo install cargo-leptos`)

### Common Commands
The project uses a `justfile` for convenience:

*   **Build Backend:** `just build` (`cargo build`)
*   **Run Backend:** `just run` (`cargo run`)
*   **Run Tests:** `just test` (`cargo test`)
*   **Format Code:** `just fmt`
*   **Lint Code:** `just clippy`
*   **Start UI Dev Server:** `just ui-dev` (Requires `cargo-leptos`)
*   **Build UI for Prod:** `just ui-build`
*   **Start Full App (Backend + UI):** `just start-full`

### Configuration
Configuration is handled via `metis.toml`.
*   **Hot Reload:** The server watches this file and applies changes without restarting.
*   **Structure:** Defines `[server]`, `[[resources]]`, `[[tools]]`, and `[[prompts]]`.

### Testing
*   **Unit Tests:** `cargo test --lib`
*   **Integration Tests:** `cargo test --test api_integration_test`
*   **Benchmarks:** `cargo bench`

## Architecture Notes
*   **Hexagonal Architecture:** The project explicitly attempts to follow Ports & Adapters (Hexagonal) architecture to separate core domain logic from external interfaces (HTTP, MCP protocol).
*   **Observability:** Extensive use of `tracing` and Prometheus metrics (`/metrics`).
