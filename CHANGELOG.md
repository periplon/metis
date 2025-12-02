# Changelog
All notable changes to this project will be documented in this file. See [conventional commits](https://www.conventionalcommits.org/) for commit guidelines.

- - -
## 0.36.0 - 2025-12-02
#### Features
- **(ui)** add tabbed interface to Workflow create and edit forms - (96dcf8d) - Joan Marc Carbo Arnau

- - -

## 0.35.0 - 2025-12-02
#### Features
- **(ui)** add tabbed interface to Prompt create and edit forms - (8b1e4eb) - Joan Marc Carbo Arnau

- - -

## 0.34.0 - 2025-12-02
#### Features
- add schema-driven faker generation and schema form UI component - (85d5fa7) - Joan Marc Carbo Arnau

- - -

## 0.33.0 - 2025-12-01
#### Features
- **(ui)** add JSON editor with syntax highlighting for static responses - (5835d88) - Joan Marc Carbo Arnau

- - -

## 0.32.0 - 2025-12-01
#### Features
- **(ui)** standardize form layouts with consistent back navigation - (bbbeb78) - Joan Marc Carbo Arnau

- - -

## 0.31.0 - 2025-12-01
#### Features
- **(ui)** add tabbed settings interface and archetype counts - (f2f8ee0) - Joan Marc Carbo Arnau

- - -

## 0.30.0 - 2025-12-01
#### Features
- **(persistence)** add database layer with git-style version history - (cfaddea) - Joan Marc Carbo Arnau

- - -

## 0.29.1 - 2025-11-28
#### Bug Fixes
- **(config)** preserve symlinks for Kubernetes ConfigMap compatibility - (5ce312a) - Joan Marc Carbo Arnau

- - -

## 0.29.0 - 2025-11-28
#### Features
- **(config)** add optimistic locking and S3 save for all archetypes - (a3f07fb) - Joan Marc Carbo Arnau

- - -

## 0.28.0 - 2025-11-28
#### Features
- **(schemas)** add reusable JSON schema definitions with $ref support - (9de4754) - Joan Marc Carbo Arnau

- - -

## 0.27.0 - 2025-11-28
#### Features
- **(secrets)** encrypt and persist UI secrets to config file - (221a12d) - Joan Marc Carbo Arnau

- - -

## 0.26.0 - 2025-11-28
#### Features
- **(agents)** add resource and resource template support - (c3ede66) - Joan Marc Carbo Arnau

- - -

## 0.25.0 - 2025-11-27
#### Features
- **(ui)** group selected artifacts by category in agent editor - (b1e524d) - Joan Marc Carbo Arnau

- - -

## 0.24.0 - 2025-11-27
#### Features
- **(ui)** unified artifact selector for agents and workflows - (3fee12d) - Joan Marc Carbo Arnau

- - -

## 0.23.0 - 2025-11-27
#### Features
- **(s3)** use credentials from UI secrets store with env var fallback - (542a980) - Joan Marc Carbo Arnau

- - -

## 0.22.9 - 2025-11-27
#### Bug Fixes
- **(api)** extract detailed S3 service errors for debugging - (0949a0b) - Joan Marc Carbo Arnau

- - -

## 0.22.8 - 2025-11-27
#### Bug Fixes
- **(api)** improve S3 error messages with specific guidance - (cea2b4c) - Joan Marc Carbo Arnau

- - -

## 0.22.7 - 2025-11-27
#### Bug Fixes
- **(ui)** auto-save settings before Save to Disk/S3 - (e367187) - Joan Marc Carbo Arnau
#### Tests
- **(s3)** add TOML serialization tests and fix e2e test - (7934181) - Joan Marc Carbo Arnau

- - -

## 0.22.6 - 2025-11-27
#### Bug Fixes
- **(api)** improve S3 save error messages with specific guidance - (b4dc6be) - Joan Marc Carbo Arnau

- - -

## 0.22.5 - 2025-11-27
#### Bug Fixes
- **(api)** always return S3 config with defaults in settings endpoint - (6332363) - Joan Marc Carbo Arnau

- - -

## 0.22.4 - 2025-11-27
#### Bug Fixes
- **(workflows)** enable agent tools in workflow steps - (b8e227a) - Joan Marc Carbo Arnau

- - -

## 0.22.3 - 2025-11-27
#### Bug Fixes
- **(agents)** serialize structured input as JSON for schema-based agents - (b81a0ad) - Joan Marc Carbo Arnau

- - -

## 0.22.2 - 2025-11-27
#### Bug Fixes
- **(agents)** auto-generate prompt from structured input when no template - (a36613b) - Joan Marc Carbo Arnau
#### Miscellaneous Chores
- update build artifacts - (30665ab) - Joan Marc Carbo Arnau

- - -

## 0.22.1 - 2025-11-27
#### Bug Fixes
- **(api)** update workflow name when editing - (e886b6f) - Joan Marc Carbo Arnau

- - -

## 0.22.0 - 2025-11-27
#### Features
- **(ui)** expose workflows as tools in agent and workflow editors - (cd9ccea) - Joan Marc Carbo Arnau

- - -

## 0.21.2 - 2025-11-27
#### Bug Fixes
- **(ui)** fix workflow step editor disabled attribute rendering - (06cadc8) - Joan Marc Carbo Arnau

- - -

## 0.21.1 - 2025-11-27
#### Bug Fixes
- **(ui)** make step ID and tool name reactive in workflow step editor - (95b30a0) - Joan Marc Carbo Arnau

- - -

## 0.21.0 - 2025-11-27
#### Features
- **(ui)** include agent tools in workflow step tool selector - (db01464) - Joan Marc Carbo Arnau

- - -

## 0.20.0 - 2025-11-27
#### Features
- **(mcp)** add list change notifications and agent reinitialization - (ef3f2e5) - Joan Marc Carbo Arnau

- - -

## 0.19.1 - 2025-11-26
#### Bug Fixes
- **(mcp)** validate agent input schemas have type field - (fdcfba4) - Joan Marc Carbo Arnau

- - -

## 0.19.0 - 2025-11-26
#### Features
- **(ui)** add input/output schema editors for agents - (13e6e06) - Joan Marc Carbo Arnau

- - -

## 0.18.0 - 2025-11-26
#### Features
- **(mcp)** expose agents as tools externally via MCP server - (2204b45) - Joan Marc Carbo Arnau

- - -

## 0.17.0 - 2025-11-26
#### Features
- **(agents)** expose agents as tools for agent-to-agent and workflow consumption - (1806342) - Joan Marc Carbo Arnau

- - -

## 0.16.1 - 2025-11-26
#### Bug Fixes
- **(agents)** preserve conversation context in multi-turn ReAct sessions - (8af33cf) - Joan Marc Carbo Arnau

- - -

## 0.16.0 - 2025-11-26
#### Features
- **(secrets)** add in-memory secrets store and AGE encryption - (3fa3cc9) - Joan Marc Carbo Arnau

- - -

## 0.15.0 - 2025-11-26
#### Features
- **(agents)** add AI agents with MCP tool support and config import/export - (1b94453) - Joan Marc Carbo Arnau

- - -

## 0.14.0 - 2025-11-25
#### Features
- **(mcp)** add explicit ping support - (d7bbbf7) - Joan Marc Carbo Arnau

- - -

## 0.13.0 - 2025-11-25
#### Features
- **(resource-templates)** add full support for MCP resource templates - (ca8a195) - Joan Marc Carbo Arnau

- - -

## 0.12.2 - 2025-11-25
#### Bug Fixes
- **(docker)** set default METIS_HOST=0.0.0.0 for container networking - (449e516) - Joan Marc Carbo Arnau

- - -

## 0.12.1 - 2025-11-25
#### Bug Fixes
- **(docker)** use rust:1.91-bookworm to match runtime glibc - (f788839) - Joan Marc Carbo Arnau

- - -

## 0.12.0 - 2025-11-25
#### Features
- **(api)** make /health endpoints non-authenticated - (049468e) - Joan Marc Carbo Arnau

- - -

## 0.11.1 - 2025-11-25
#### Bug Fixes
- **(ui)** improve schema editors with enum support and proper URL encoding - (fbbc766) - Joan Marc Carbo Arnau

- - -

## 0.11.0 - 2025-11-25
#### Features
- **(ui)** add schema editors and DAG workflow support - (1852524) - Joan Marc Carbo Arnau

- - -

## 0.10.0 - 2025-11-24
#### Features
- **(ui)** add test functionality for tools, resources, prompts, and workflows - (8fca53b) - Joan Marc Carbo Arnau

- - -

## 0.9.4 - 2025-11-24
#### Bug Fixes
- **(ui)** use gloo-net instead of reqwest for HTTP requests - (e452c63) - Joan Marc Carbo Arnau

- - -

## 0.9.3 - 2025-11-24
#### Bug Fixes
- **(api)** use correct axum path parameter syntax - (0dd2038) - Joan Marc Carbo Arnau

- - -

## 0.9.2 - 2025-11-24
#### Bug Fixes
- **(ci)** use trunk instead of cargo-leptos for UI build - (fbd929f) - Joan Marc Carbo Arnau

- - -

## 0.9.1 - 2025-11-24
#### Bug Fixes
- **(ui)** fix edit forms loading blank by checking params readiness - (b0edb04) - Joan Marc Carbo Arnau

- - -

## 0.9.0 - 2025-11-24
#### Features
- **(ui)** add working edit/delete for workflows with view modes - (a73163b) - Joan Marc Carbo Arnau

- - -

## 0.8.1 - 2025-11-24
#### Bug Fixes
- **(ui)** handle API responses without data for save operations - (a20e502) - Joan Marc Carbo Arnau

- - -

## 0.8.0 - 2025-11-24
#### Features
- **(ui)** add edit/delete functionality and unified list views - (5b731e6) - Joan Marc Carbo Arnau

- - -

## 0.7.0 - 2025-11-24
#### Features
- **(ui)** add mock strategy editors and improve config handling - (4eba766) - Joan Marc Carbo Arnau

- - -

## 0.6.0 - 2025-11-24
#### Features
- **(ui)** add enhanced web UI with REST API for configuration management - (9b87b8b) - Joan Marc Carbo Arnau

- - -

## 0.5.0 - 2025-11-24
#### Features
- **(workflow)** implement advanced workflow engine as tool provider - (79b2e34) - Joan Marc Carbo Arnau

- - -

## 0.4.0 - 2025-11-24
#### Features
- **(mock)** implement File and Pattern strategies - (225446d) - Joan Marc Carbo Arnau

- - -

## 0.3.0 - 2025-11-24
#### Documentation
- update script strategy status to reflect all languages working - (dbed445) - Joan Marc Carbo Arnau
#### Features
- **(auth)** integrate authentication middleware into application - (5b97d41) - Joan Marc Carbo Arnau

- - -

## 0.2.0 - 2025-11-24
#### Features
- **(config)** add S3 live reload configuration with CLI support - (48f3927) - Joan Marc Carbo Arnau

- - -

## 0.1.12 - 2025-11-24
#### Bug Fixes
- **(ci)** properly rename release binaries to avoid directory conflicts - (8f4c353) - Joan Marc Carbo Arnau

- - -

## 0.1.11 - 2025-11-24
#### Bug Fixes
- **(ci)** remove v prefix from tag references in release job - (0ff5a39) - Joan Marc Carbo Arnau

- - -

## 0.1.10 - 2025-11-24
#### Bug Fixes
- **(deps)** upgrade leptos to 0.8 and update toolchain - (53e2496) - Joan Marc Carbo Arnau

- - -

## 0.1.9 - 2025-11-24
#### Bug Fixes
- **(ci)** use cargo-leptos 0.2.21 with --locked flag - (ce2abe6) - Joan Marc Carbo Arnau

- - -

## 0.1.8 - 2025-11-24
#### Bug Fixes
- **(ci)** update Rust version and pin cargo-leptos version - (2866d81) - Joan Marc Carbo Arnau

- - -

## 0.1.7 - 2025-11-24
#### Bug Fixes
- **(ci)** add Cargo.lock to repository for reproducible builds - (a34b476) - Joan Marc Carbo Arnau
#### Refactoring
- replace custom MCP layer with standard rmcp SDK - (cd6faff) - Joan Marc Carbo Arnau

- - -

## 0.1.6 - 2025-11-24
#### Bug Fixes
- **(ci)** set wasm-bindgen version to match rust dependency - (94a9a3e) - Joan Marc Carbo Arnau

- - -

## 0.1.5 - 2025-11-24
#### Bug Fixes
- **(ci)** remove unnecessary dart-sass installation - (dd14017) - Joan Marc Carbo Arnau

- - -

## 0.1.4 - 2025-11-24
#### Bug Fixes
- **(ci)** correct step id reference in CD workflow version output - (5e73385) - Joan Marc Carbo Arnau
#### Documentation
- add repository guidelines covering project structure, build, style, testing, and commit conventions. - (6187c08) - Joan Marc Carbo Arnau

- - -

## 0.1.3 - 2025-11-20
#### Bug Fixes
- Ensure version-and-tag outputs are set correctly when bump is skipped - (53cdbe1) - Joan Marc Carbo Arnau

- - -

## 0.1.2 - 2025-11-20
#### Bug Fixes
- Skip docker and release jobs when version bump is skipped - (da29c75) - Joan Marc Carbo Arnau

- - -

## 0.1.1 - 2025-11-20
#### Bug Fixes
- Use array syntax for workspace.metadata.leptos configuration - (0819656) - Joan Marc Carbo Arnau
#### Continuous Integration
- Configure leptos workspace metadata and simplify build command - (0802dce) - Joan Marc Carbo Arnau

- - -

## 0.1.0 - 2025-11-20
#### Continuous Integration
- Clean up cocogitto installation artifacts to prevent untracked files error - (5dd4c78) - Joan Marc Carbo Arnau
- Fix build-ui job by installing dart-sass and pinning cargo-binstall - (cf7c763) - Joan Marc Carbo Arnau
- Fix CD pipeline robustness for git push and cargo-leptos installation - (89806b5) - Joan Marc Carbo Arnau
- Setup complete CI/CD pipeline with semantic versioning and multi-platform builds - (1b5f197) - Joan Marc Carbo Arnau
#### Documentation
- Update implementation plan executive summary to reflect current project status - (d4ec213) - Joan Marc Carbo Arnau
- Refine roadmap based on deep code inspection - (5d459cb) - Joan Marc Carbo Arnau
- Update project status in README to reflect current development state - (3e61094) - Joan Marc Carbo Arnau
- Specify 'text' language for project structure ASCII art - (ee6b460) - Joan Marc Carbo Arnau
- Fix README.md ASCII art rendering - (ab49214) - Joan Marc Carbo Arnau
#### Features
- Implement support for Python scripting using rustpython-vm - (8eed860) - Joan Marc Carbo Arnau
- Add a new web UI, integrate LLM capabilities for mock strategies, and introduce benchmarking infrastructure. - (51f1575) - Joan Marc Carbo Arnau
- Add authentication, health checks, Prometheus metrics, Docker support, and CI/CD pipelines. - (b72df81) - Joan Marc Carbo Arnau
- Implement external configuration loading for tools, resources, and prompts with dynamic watching. - (3a3090f) - Joan Marc Carbo Arnau
- Initialize Metis project with core application structure, configuration, various handlers for resources, tools, prompts, and mock strategies, and related tests. - (f3d1f9e) - Joan Marc Carbo Arnau
#### Miscellaneous Chores
- save current state before pushing to GitHub - (35fefea) - Joan Marc Carbo Arnau

- - -

Changelog generated by [cocogitto](https://github.com/cocogitto/cocogitto).