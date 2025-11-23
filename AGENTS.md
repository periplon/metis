# Repository Guidelines

## Project Structure & Module Organization
- Core Rust service lives in `src/`: `adapters/` handle MCP protocol resources/tools, `application/` orchestrates flows, `domain/` holds shared types, `config/` handles loading; entrypoints are `main.rs` and `lib.rs`.
- Config examples live in `config/` (metis.toml, prompts/resources/tools) with runtime config at repo root `metis.toml`. Integration tests sit in `tests/` (`api_integration_test.rs`), benchmarks in `benches/`, samples in `examples/`, data fixtures in `data/`, and the Leptos UI in `ui/` (`src`, `dist`, `style.css`).

## Build, Test, and Development Commands
- Use `just` (see `justfile`) for common flows: `just build`, `just run`, `just test`, `just fmt`, `just clippy`, `just ui-dev` (Leptos watch), `just ui-build` (prod UI), `just start-full` (`--all-features` server plus built UI).
- Cargo equivalents: `cargo build --release` for optimized binaries, `RUST_LOG=info cargo run --all-features` to start the service, `cargo test` for unit/integration suites, `cargo clean` to reset artifacts. Use Docker (`Dockerfile`, `docker-compose.yml`) only when reproducing containerized runs.

## Coding Style & Naming Conventions
- Enforce `rustfmt`; run `just fmt` before PRs. Keep Clippy clean (`just clippy` treats warnings as errors). Default to 4-space indent and rustfmt width; avoid unchecked `unwrap`/`expect` in adapters unless converted to clear errors.
- Naming: modules/files and functions/vars use `snake_case`, types/enums/traits `PascalCase`, constants `SCREAMING_SNAKE_CASE`. Keep layer boundaries: domain for logic and types, adapters for IO/MCP wiring, config for loading/validation.

## Testing Guidelines
- Prefer unit tests next to modules and integration tests under `tests/` for protocol-level or config-driven behavior. Name tests descriptively (e.g., `it_handles_template_strategy`, `rejects_invalid_config`).
- Run `just test` (or `cargo test`) before pushing; add UI manual checks when touching `ui/` flows. When randomness is involved, seed generators or inject fakes to keep tests deterministic; place reusable fixtures under `data/`.

## Commit & Pull Request Guidelines
- Follow conventional commits used here (`fix:`, `chore(version):`, `feat(scope):`, `refactor:`); imperative mood with concise subject. Include scope when it narrows the change (e.g., `fix(adapter): handle missing headers`).
- PRs should summarize intent, call out config changes (`metis.toml`, `config/*`), link issues, and list verification (`just test`, `just clippy`, `just fmt`). Add screenshots/gifs for UI changes and note any new env vars.

## Configuration & Security Tips
- Keep secrets (LLM keys, DB URIs) out of tracked configs; load them via environment and local overrides. Use `RUST_LOG=debug` when diagnosing config reloads or mock strategies, and avoid committing generated `ui/dist` unless part of a tagged release.
