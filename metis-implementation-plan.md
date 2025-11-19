# Metis MCP Mock Server - Implementation Plan

## Executive Summary

Metis is a high-performance, fully configurable MCP (Model Context Protocol) mock server written in Rust. It provides comprehensive mocking capabilities for resources, tools, and prompts with support for multiple authentication methods, various data generation strategies, and live configuration reloading.

**Project Timeline**: 36 weeks (9 months) from start to full v1.3 release
**Checkpoints**: Week 0 (validation), Week 11 (Phase 3 review), Week 17 (v1.0 decision point)
**Incremental Releases**: v1.0 (Week 17), v1.1 (Week 21), v1.2 (Week 31), v1.3 (Week 35)

**Key Phases**:
- **Week 0**: Technology validation and risk mitigation
- **Weeks 1-17**: Core platform (v1.0) - MCP protocol, mock strategies, auth, config, observability
- **Weeks 18-35**: Advanced features (v1.1-v1.3) - Workflows, multi-language scripting, Web UI, comprehensive testing

**Success Metrics**:
- >10k requests/second performance
- >85% test coverage
- 500+ organizations using in first year
- Complete documentation with tutorials

## Table of Contents

1. [Project Goals](#project-goals)
2. [Architecture Overview](#architecture-overview)
   - [Hexagonal Architecture & SOLID Principles](#hexagonal-architecture--solid-principles)
3. [Core Components](#core-components)
   - [MCP Protocol Handler](#1-mcp-protocol-handler)
   - [Resource Handler](#2-resource-handler)
   - [Tool Handler](#3-tool-handler)
   - [Prompt Handler](#4-prompt-handler)
   - [Pre-Generated Fake Data](#5-pre-generated-fake-data)
   - [Workflow Engine](#6-workflow-engine)
4. [Web UI (Leptos)](#web-ui-leptos)
5. [Mock Data Generation Strategies](#mock-data-generation-strategies)
6. [Configuration System](#configuration-system)
7. [Performance Optimization](#performance-optimization)
8. [Development Phases](#development-phases)
9. [Testing Strategy](#testing-strategy)
10. [Automatic Testing with MCP Clients](#automatic-testing-with-mcp-clients)
11. [Agent Endpoints & Orchestration](#agent-endpoints--orchestration)
12. [Model Definitions & Relationships](#model-definitions--relationships)
13. [Deployment & Operations](#deployment--operations)

---

## Project Goals

### Primary Objectives
- **Comprehensive Mocking**: Support all MCP protocol features (resources, tools, prompts)
- **Flexibility**: Multiple data generation strategies for different use cases
- **Performance**: High throughput, low latency, efficient resource usage
- **Configurability**: Declarative configuration with live reload
- **Developer Experience**: Easy setup, clear documentation, intuitive configuration
- **Automatic Testing**: Built-in test client with automatic test generation and protocol compliance validation
- **Agent Orchestration**: Single and multi-agent endpoints with tool calling capabilities
- **Production-Ready**: Robust error handling, logging, monitoring capabilities

### Use Cases
- Local development and testing of MCP clients
- Integration testing of MCP-based applications
- Automatic test generation and execution with real MCP clients
- Agent development and testing with tool calling
- Multi-agent system prototyping and simulation
- Agent orchestration pattern experimentation
- Performance testing and benchmarking
- Protocol compliance verification
- Contract testing between MCP providers and consumers
- Snapshot testing for regression detection
- Training and demonstration environments
- CI/CD pipeline integration

---

## Architecture Overview

### High-Level Architecture

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
│  │  │   JSON-RPC 2.0 Handler (Official Rust SDK)     │   ││
│  │  └────────────────────────────────────────────────┘   ││
│  └─────────────────────────────────────┬──────────────────┘│
│                                        │                     │
│  ┌────────────────┐  ┌────────────────┼───────────────┐   │
│  │ Auth Middleware│◀─┤     Router     │               │   │
│  └────────────────┘  └────────────────┼───────────────┘   │
│                                        │                     │
│  ┌────────────┬─────────────┬─────────┼──────────────┐    │
│  │  Resource  │    Tool     │  Prompt │  Session     │    │
│  │  Handler   │   Handler   │ Handler │  Manager     │    │
│  └────────────┴─────────────┴─────────┴──────────────┘    │
│         │             │            │           │            │
│  ┌──────┼─────────────┼────────────┼───────────┼──────┐   │
│  │      │     Mock Data Generation Layer       │      │   │
│  │  ┌───▼──┐  ┌───▼───┐  ┌───▼───┐  ┌────▼────┐    │   │
│  │  │Random│  │Template│  │  LLM  │  │ Script  │    │   │
│  │  └──────┘  └────────┘  └───────┘  └─────────┘    │   │
│  │  ┌──────┐  ┌────────┐  ┌───────┐  ┌─────────┐   │   │
│  │  │Pattern│ │Database│  │  File │  │ Static  │   │   │
│  │  └──────┘  └────────┘  └───────┘  └─────────┘    │   │
│  └─────────────────────────────────────────────────┘    │
│                                                           │
│  ┌───────────────────────────────────────────────────┐  │
│  │        Support Services Layer                      │  │
│  │  ┌─────────┐  ┌──────────┐  ┌──────────────┐    │  │
│  │  │ Logger  │  │ Metrics  │  │ Cache        │    │  │
│  │  └─────────┘  └──────────┘  └──────────────┘    │  │
│  └───────────────────────────────────────────────────┘  │
│                                                           │
│  ┌───────────────────────────────────────────────────┐  │
│  │        Automatic Testing Layer                     │  │
│  │  ┌──────────────┐  ┌─────────────────────────┐   │  │
│  │  │ Test Client  │  │ Test Generator          │   │  │
│  │  └──────────────┘  └─────────────────────────┘   │  │
│  │  ┌──────────────┐  ┌─────────────────────────┐   │  │
│  │  │ Compliance   │  │ Contract Tests          │   │  │
│  │  └──────────────┘  └─────────────────────────┘   │  │
│  └───────────────────────────────────────────────────┘  │
│                                                           │
│  ┌───────────────────────────────────────────────────┐  │
│  │        Agent Orchestration Layer                   │  │
│  │  ┌──────────────┐  ┌─────────────────────────┐   │  │
│  │  │Single Agent  │  │ Multi-Agent             │   │  │
│  │  │Endpoint      │  │ Orchestrator            │   │  │
│  │  └──────────────┘  └─────────────────────────┘   │  │
│  │  ┌──────────────┐  ┌─────────────────────────┐   │  │
│  │  │Tool Executor │  │ Agent State Manager     │   │  │
│  │  └──────────────┘  └─────────────────────────┘   │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### Technology Stack

**Core Framework**
- Rust 1.75+
- Official Rust MCP SDK (`mcp-rs` or equivalent)
- Tokio (async runtime)
- Axum or Actix-web (HTTP server)

**Configuration**
- TOML/YAML parsing (serde)
- File watching (notify)

**Data Generation**
- Fake data: `fake` crate
- Template engine: `tera` or `handlebars`
- LLM integration: OpenAI/Anthropic API clients
- Multi-language scripting:
  - Python: `pyo3`
  - Lua: `mlua`
  - Ruby: `rutie` or `magnus`
  - Rhai: Native Rust scripting
  - JavaScript: `boa` or `deno_core`

**Database Support**
- SQLx (PostgreSQL, MySQL, SQLite)
- Redis client (optional caching)

**Utilities**
- Regex: `regex` crate
- JSON: `serde_json`
- Logging: `tracing` + `tracing-subscriber`
- Metrics: `prometheus` or `opentelemetry`

**Testing**
- Test framework: `tokio-test`
- Test client: Official MCP Rust SDK client
- Property-based testing: `proptest`
- Fuzzing: `cargo-fuzz`
- Snapshot testing: `insta`
- Contract testing: Custom framework
- Multi-client testing: Node.js, Python, Rust clients

**Agent Orchestration**
- LLM clients: OpenAI, Anthropic, local model support
- Agent frameworks: Custom agent runtime
- State management: In-memory and persistent stores
- Message queue: Tokio channels, optional Redis
- Tool calling: MCP tool integration layer
- Agent coordination: Custom orchestration engine

**Web UI**
- Frontend: Leptos (Rust WebAssembly framework)
- Routing: Leptos Router
- Code Editor: Monaco Editor component
- Visualization: Mermaid for workflow diagrams
- Styling: TailwindCSS
- Backend API: Axum with WebSocket support
- File Watcher: notify crate for config hot-reload

### Hexagonal Architecture & SOLID Principles

Metis follows **Hexagonal Architecture** (Ports and Adapters) combined with **SOLID principles** to ensure maintainability, testability, and extensibility.

#### Hexagonal Architecture Layers

```
┌─────────────────────────────────────────────────────────┐
│                    Presentation Layer                    │
│        (Web UI, CLI, MCP Protocol Handlers)             │
│                     ↓ depends on ↓                      │
└───────────────────────┬─────────────────────────────────┘
                        │
┌───────────────────────▼─────────────────────────────────┐
│              Application Layer (Ports)                   │
│                                                           │
│  pub trait ResourcePort: Send + Sync {                   │
│      async fn get_resource(&self, uri: &str);           │
│  }                                                        │
│                                                           │
│  pub trait MockStrategyPort: Send + Sync {               │
│      async fn generate(&self, ctx: &Context);           │
│  }                                                        │
│                                                           │
│  pub trait WorkflowExecutionPort: Send + Sync {          │
│      async fn execute(&self, workflow: &str);           │
│  }                                                        │
│                     ↓ depends on ↓                      │
└───────────────────────┬─────────────────────────────────┘
                        │
┌───────────────────────▼─────────────────────────────────┐
│              Domain Layer (Business Logic)               │
│                                                           │
│  - ConfigValidator: Validates configurations            │
│  - ResourceRegistry: Manages resource lifecycle          │
│  - WorkflowOrchestrator: Orchestrates workflow steps    │
│  - AgentManager: Manages agent state and execution       │
│  - MockStrategySelector: Chooses appropriate strategy    │
│                                                           │
│  Business rules are independent of infrastructure        │
│                     ↓ depends on ↓                      │
└───────────────────────┬─────────────────────────────────┘
                        │
┌───────────────────────▼─────────────────────────────────┐
│         Infrastructure Layer (Adapters)                  │
│                                                           │
│  Inbound Adapters (Driving):                            │
│  - McpProtocolAdapter: Implements MCP protocol          │
│  - WebUIAdapter: Handles HTTP/WebSocket for UI          │
│  - CliAdapter: Command-line interface                    │
│                                                           │
│  Outbound Adapters (Driven):                            │
│  - FileSystemAdapter: File operations                    │
│  - DatabaseAdapter: SQL/NoSQL database access            │
│  - LlmAdapter: LLM API integration                       │
│  - CacheAdapter: Redis/in-memory caching                 │
│  - ScriptEngineAdapter: Multi-language script execution  │
│                                                           │
└─────────────────────────────────────────────────────────┘
```

#### SOLID Principles Implementation

**S - Single Responsibility Principle**
```rust
// Each component has one reason to change
pub struct ResourceHandler {
    registry: Arc<dyn ResourceRegistryPort>,
    strategy_factory: Arc<StrategyFactory>,
}

pub struct AuthenticationMiddleware {
    auth_service: Arc<dyn AuthenticationPort>,
}

pub struct CacheManager {
    cache: Arc<dyn CachePort>,
}
```

**O - Open/Closed Principle**
```rust
// New mock strategies can be added without modifying existing code
#[async_trait]
pub trait MockStrategy: Send + Sync {
    async fn generate(&self, context: &RequestContext) -> Result<serde_json::Value, Error>;
}

pub struct CustomMLStrategy {
    model: String,
}

#[async_trait]
impl MockStrategy for CustomMLStrategy {
    async fn generate(&self, context: &RequestContext) -> Result<serde_json::Value, Error> {
        // Custom implementation
    }
}
```

**L - Liskov Substitution Principle**
```rust
// Any strategy can be substituted
let random_handler = create_handler(Arc::new(RandomStrategy::new()));
let llm_handler = create_handler(Arc::new(LlmStrategy::new()));
let db_handler = create_handler(Arc::new(DatabaseStrategy::new()));
```

**I - Interface Segregation Principle**
```rust
// Small, focused trait definitions
pub trait ResourceQueryPort: Send + Sync {
    async fn get_resource(&self, uri: &str) -> Result<Resource, Error>;
    async fn list_resources(&self) -> Result<Vec<Resource>, Error>;
}

pub trait ConfigManagementPort: Send + Sync {
    async fn save_file(&self, path: &str, content: &str) -> Result<(), Error>;
    async fn reload(&self) -> Result<(), Error>;
}

pub trait MetricsCollectionPort: Send + Sync {
    async fn record_metric(&self, name: &str, value: f64);
    fn get_metrics(&self) -> ServerStats;
}
```

**D - Dependency Inversion Principle**
```rust
// High-level modules depend on abstractions
pub struct WorkflowEngine {
    tool_handler: Arc<dyn ToolExecutionPort>,
    script_engine: Arc<dyn ScriptExecutionPort>,
    resource_provider: Arc<dyn ResourceQueryPort>,
}

// Concrete implementations injected at runtime
let engine = WorkflowEngine::new(
    Arc::new(McpToolHandler::new()),
    Arc::new(MultiLanguageScriptEngine::new()),
    Arc::new(CachedResourceProvider::new()),
);
```

#### Benefits
1. **Testability**: Easy to mock dependencies via traits
2. **Flexibility**: Swap implementations without code changes
3. **Maintainability**: Clear separation of concerns
4. **Extensibility**: Add new features without modifying existing code
5. **Type Safety**: Rust's type system enforces contracts at compile time
6. **Domain Isolation**: Business logic independent of frameworks

---

## Core Components

### 1. MCP Protocol Handler

**Responsibilities**
- Implement MCP protocol specification
- Handle JSON-RPC 2.0 messages
- Manage protocol lifecycle (initialization, requests, notifications)
- Session management

**Implementation Details**
```rust
// Core protocol types
pub struct McpServer {
    config: Arc<RwLock<ServerConfig>>,
    resource_handler: Arc<ResourceHandler>,
    tool_handler: Arc<ToolHandler>,
    prompt_handler: Arc<PromptHandler>,
    auth_manager: Arc<AuthManager>,
    session_manager: Arc<SessionManager>,
}

// Protocol capabilities
pub struct ServerCapabilities {
    resources: Option<ResourceCapabilities>,
    tools: Option<ToolCapabilities>,
    prompts: Option<PromptCapabilities>,
    logging: Option<LoggingCapabilities>,
}
```

### 2. Resource Handler

**Capabilities**
- List available resources
- Read resource content
- Subscribe to resource updates
- Support resource templates
- Load resource definitions from JSON/YAML/TOML files

**Resource Definition Files**

Resources can be defined inline in `metis.toml` or in separate files in `config/resources/`:

**Inline Definition** (`metis.toml`)
```toml
[[resources]]
uri = "file:///{path}"
name = "Mock File System"
description = "Simulated file system access"
mime_type = "text/plain"

[resources.mock]
strategy = "template"
template = "templates/file_content.txt"
variables = { path = "random", size = "range:100-10000" }

[resources.mock.behavior]
latency_ms = "range:10-100"
error_rate = 0.05  # 5% error rate
cache_duration_sec = 300

# Import external resource definitions
resources_dir = "config/resources/"
```

**External YAML Definition** (`config/resources/users.yaml`)
```yaml
uri: "db://users"
name: "Users Database"
description: "Mock user database"
mime_type: "application/json"

mock:
  strategy: "random"
  random:
    type: "array"
    min_length: 5
    max_length: 20
    item:
      type: "object"
      schema:
        id: { type: "uuid" }
        username: { type: "fake", fake_type: "internet.username" }
        email: { type: "fake", fake_type: "internet.email" }

  behavior:
    latency_ms: "range:50-200"
    error_rate: 0.02
    cache_duration_sec: 60
```

**External JSON Definition** (`config/resources/files.json`)
```json
{
  "uri": "file:///documents/{id}",
  "name": "Document Storage",
  "description": "Mock document storage system",
  "mime_type": "application/pdf",
  "mock": {
    "strategy": "file",
    "file": {
      "path": "fixtures/documents/*.pdf",
      "selection": "random"
    }
  }
}
```

### 3. Tool Handler

**Capabilities**
- List available tools with schemas
- Execute tool calls
- Return structured results
- Handle streaming responses
- Load tool definitions from JSON/YAML/TOML files

**Tool Definition Files**

Tools can be defined inline in `metis.toml` or in separate files in `config/tools/`:

**Inline Definition** (`metis.toml`)
```toml
[[tools]]
name = "search_database"
description = "Search the database"

[tools.input_schema]
type = "object"
properties = { query = { type = "string" } }

[tools.mock]
strategy = "script"
script = """
fn execute(input) {
    let results = [];
    for i in 0..rand_int(5, 20) {
        results.push({
            id: uuid(),
            title: fake("company.name"),
            score: rand_float(0.0, 1.0)
        });
    }
    return results;
}
"""

[tools.mock.behavior]
success_rate = 0.95
execution_time_ms = "range:50-500"

# Import external tool definitions
tools_dir = "config/tools/"
```

**External TOML Definition** (`config/tools/code_search.toml`)
```toml
name = "search_code"
description = "Search through codebase for specific patterns"

[input_schema]
type = "object"
required = ["query"]

[input_schema.properties]
query = { type = "string", description = "Search query" }
file_types = { type = "array", items = { type = "string" } }
case_sensitive = { type = "boolean", default = false }

[mock]
strategy = "template"
template_file = "templates/code_search_results.json"

[mock.variables]
result_count = { type = "integer", min = 5, max = 50 }

[mock.behavior]
execution_time_ms = "range:100-500"
success_rate = 0.98
```

**External YAML Definition** (`config/tools/data_analysis.yaml`)
```yaml
name: "analyze_data"
description: "Perform statistical analysis on dataset"

input_schema:
  type: "object"
  required: ["data"]
  properties:
    data:
      type: "array"
      description: "Dataset to analyze"
    analysis_type:
      type: "string"
      enum: ["mean", "median", "std_dev", "correlation"]
      default: "mean"

mock:
  strategy: "script"
  script_language: "python"
  script: |
    import statistics
    def execute(input):
        data = input['data']
        analysis_type = input.get('analysis_type', 'mean')
        if analysis_type == 'mean':
            return {'result': statistics.mean(data)}
        elif analysis_type == 'median':
            return {'result': statistics.median(data)}
        # ... more analysis types
```

**External JSON Definition** (`config/tools/api_call.json`)
```json
{
  "name": "call_external_api",
  "description": "Make an external API call",
  "input_schema": {
    "type": "object",
    "required": ["url"],
    "properties": {
      "url": { "type": "string" },
      "method": {
        "type": "string",
        "enum": ["GET", "POST", "PUT", "DELETE"],
        "default": "GET"
      },
      "headers": { "type": "object" },
      "body": { "type": "object" }
    }
  },
  "mock": {
    "strategy": "composite",
    "composite": {
      "mode": "weighted",
      "strategies": [
        {
          "strategy": "static",
          "weight": 0.7,
          "static": {
            "content": { "status": 200, "data": {} }
          }
        },
        {
          "strategy": "llm",
          "weight": 0.3,
          "llm": {
            "provider": "anthropic",
            "model": "claude-3-5-sonnet-20241022"
          }
        }
      ]
    }
  }
}
```

### 4. Prompt Handler

**Capabilities**
- List available prompts
- Get prompt with arguments
- Return messages with roles
- Load prompt definitions from JSON/YAML/TOML files

**Prompt Definition Files**

Prompts can be defined inline in `metis.toml` or in separate files in `config/prompts/`:

**Inline Definition** (`metis.toml`)
```toml
[[prompts]]
name = "code_review"
description = "Generate code review prompt"
arguments = [
    { name = "language", required = true },
    { name = "style", required = false }
]

[prompts.mock]
strategy = "llm"
llm_provider = "openai"
model = "gpt-4"
system_prompt = "You are a code reviewer"
user_template = "Review this {{language}} code: {{code}}"

[prompts.mock.fallback]
strategy = "template"
template = "templates/code_review.txt"

# Import external prompt definitions
prompts_dir = "config/prompts/"
```

**External YAML Definition** (`config/prompts/refactoring.yaml`)
```yaml
name: "refactoring_suggestions"
description: "Generate refactoring suggestions for code"

arguments:
  - name: "code"
    required: true
    description: "Code to analyze"
  - name: "language"
    required: true
    description: "Programming language"
  - name: "focus"
    required: false
    description: "Specific area to focus on"
    default: "general"

mock:
  strategy: "template"
  template: |
    You are an expert code reviewer specializing in {{language}}.
    Analyze the following code and suggest refactorings:

    ```{{language}}
    {{code}}
    ```

    {% if focus != "general" %}
    Focus particularly on: {{focus}}
    {% endif %}
```

**External JSON Definition** (`config/prompts/documentation.json`)
```json
{
  "name": "generate_docs",
  "description": "Generate documentation for code",
  "arguments": [
    {
      "name": "code",
      "required": true,
      "description": "Code to document"
    },
    {
      "name": "style",
      "required": false,
      "description": "Documentation style",
      "enum": ["javadoc", "sphinx", "jsdoc"],
      "default": "sphinx"
    }
  ],
  "mock": {
    "strategy": "llm",
    "llm": {
      "provider": "anthropic",
      "model": "claude-3-5-sonnet-20241022",
      "system_prompt": "Generate clear, comprehensive documentation",
      "user_template": "Generate {{style}} documentation for:\n{{code}}"
    }
  }
}
```

**External TOML Definition** (`config/prompts/testing.toml`)
```toml
name = "generate_tests"
description = "Generate unit tests for code"

[[arguments]]
name = "code"
required = true

[[arguments]]
name = "framework"
required = false
default = "pytest"

[mock]
strategy = "composite"

[mock.composite]
mode = "fallback"

[[mock.composite.strategies]]
strategy = "llm"
timeout_ms = 5000

[mock.composite.strategies.llm]
provider = "openai"
model = "gpt-4-turbo"

[[mock.composite.strategies]]
strategy = "template"
template_file = "templates/test_generation.txt"
```

### 5. Pre-Generated Fake Data

**Concept**

Instead of generating fake data on every request, Metis can pre-generate datasets and save them to JSON/YAML files. These files are then loaded at runtime and exposed as resources or used by tools, providing consistent, fast access to mock data.

**Data Generation Configuration**

```toml
# metis.toml

[fake_data]
enabled = true
output_dir = "data/generated"
formats = ["json", "yaml"]  # Output formats

# Auto-generate on startup
auto_generate = true
regenerate_on_reload = false  # Keep existing data on config reload

# Generation jobs
[[fake_data.jobs]]
name = "users"
count = 1000
format = "json"
output_file = "data/generated/users.json"

[fake_data.jobs.schema]
type = "array"
items = {
    id = { type = "uuid" },
    name = { type = "fake", fake_type = "name.full_name" },
    email = { type = "fake", fake_type = "internet.email" },
    phone = { type = "fake", fake_type = "phone.number" },
    address = {
        street = { type = "fake", fake_type = "address.street" },
        city = { type = "fake", fake_type = "address.city" },
        country = { type = "fake", fake_type = "address.country" }
    },
    created_at = { type = "timestamp", format = "rfc3339" }
}

[[fake_data.jobs]]
name = "products"
count = 500
format = "yaml"
output_file = "data/generated/products.yaml"

[fake_data.jobs.schema]
type = "array"
items = {
    sku = { type = "pattern", pattern = "PRD-[0-9]{5}" },
    name = { type = "fake", fake_type = "commerce.product_name" },
    price = { type = "float", min = 9.99, max = 999.99 },
    category = { type = "choice", values = ["Electronics", "Clothing", "Food", "Books"] }
}
```

**External Job Definition** (`config/fake_data/orders.yaml`)
```yaml
name: "orders"
count: 2000
format: "json"
output_file: "data/generated/orders.json"

schema:
  type: "array"
  items:
    order_id:
      type: "pattern"
      pattern: "ORD-{:08d}"
    user_id:
      type: "foreign_key"
      from_file: "data/generated/users.json"
      field: "id"
    items:
      type: "array"
      min_length: 1
      max_length: 5
      item:
        product_id:
          type: "foreign_key"
          from_file: "data/generated/products.yaml"
          field: "sku"
        quantity:
          type: "integer"
          min: 1
          max: 10
    total:
      type: "float"
      min: 10.0
      max: 5000.0
    status:
      type: "choice"
      values: ["pending", "processing", "shipped", "delivered"]
      weights: [0.1, 0.2, 0.3, 0.4]
    created_at:
      type: "timestamp"
      range: "last_year"
```

**Generating Data**

```bash
# Generate all fake data
metis generate-data --config metis.toml

# Generate specific job
metis generate-data --job users

# Generate with specific count override
metis generate-data --job products --count 1000

# Validate generated data
metis validate-data --file data/generated/users.json
```

**Exposing Pre-Generated Data as Resources**

```toml
# Auto-expose generated data files as resources
[[resources]]
uri = "generated://users"
name = "Generated Users Dataset"
mime_type = "application/json"

[resources.mock]
strategy = "file"
file = "data/generated/users.json"
cache = true

[[resources]]
uri = "generated://products"
name = "Generated Products Dataset"
mime_type = "application/yaml"

[resources.mock]
strategy = "file"
file = "data/generated/products.yaml"
```

**Using Pre-Generated Data in Tools**

```toml
[[tools]]
name = "get_user_by_id"
description = "Get user by ID from generated dataset"

[tools.input_schema]
type = "object"
properties = { user_id = { type = "string" } }

[tools.mock]
strategy = "script"
script = """
fn execute(input) {
    let users = load_json("data/generated/users.json");
    let user_id = input.user_id;

    for user in users {
        if user.id == user_id {
            return user;
        }
    }

    return { error: "User not found" };
}
"""
```

**Data Generator CLI**

```rust
pub struct DataGenerator {
    jobs: Vec<GenerationJob>,
    output_dir: PathBuf,
}

impl DataGenerator {
    pub async fn generate_all(&self) -> Result<(), Error> {
        for job in &self.jobs {
            println!("Generating {} records for {}", job.count, job.name);

            let data = self.generate_dataset(job).await?;
            let output_path = self.output_dir.join(&job.output_file);

            match job.format.as_str() {
                "json" => self.write_json(&output_path, &data).await?,
                "yaml" => self.write_yaml(&output_path, &data).await?,
                _ => return Err(Error::UnsupportedFormat(job.format.clone())),
            }

            println!("✓ Wrote {} records to {}", job.count, output_path.display());
        }

        Ok(())
    }
}
```

**Benefits**
- **Performance**: Faster than generating on every request
- **Consistency**: Same data across requests and restarts
- **Relationships**: Easier to maintain referential integrity
- **Testing**: Reproducible test data
- **Version Control**: Can commit generated datasets
- **Portability**: Share datasets across team members

### 6. Workflow Engine

**Concept**

The Workflow Engine enables orchestration of complex multi-step processes with branching logic, looping constructs, error handling, and integration with MCP tools. Workflows are defined declaratively in YAML/JSON/TOML files and can include scripted tasks, conditional execution, and parallel processing.

**Core Features**
- **Declarative Workflows**: Define workflows in YAML/JSON/TOML
- **Control Flow**: Branching (if/else, switch), looping (for, while, foreach)
- **MCP Tool Integration**: Call any registered MCP tool from workflow steps
- **Scripted Tasks**: Execute inline scripts (Python, Lua, Ruby, Rhai, JS)
- **State Management**: Workflow variables and context passing between steps
- **Error Handling**: Try/catch blocks, retry logic with exponential backoff
- **Parallel Execution**: Run multiple steps concurrently
- **Sub-workflows**: Compose workflows from other workflows
- **Conditional Execution**: Step execution based on expressions
- **Workflow Visualization**: Generate DAG diagrams from workflow definitions

**Workflow Definition Example** (`config/workflows/data_processing.yaml`)

```yaml
name: "Data Processing Pipeline"
description: "Process user data with validation and enrichment"
version: "1.0"

# Input schema for the workflow
input_schema:
  type: "object"
  required: ["user_ids"]
  properties:
    user_ids:
      type: "array"
      items:
        type: "string"
    enrich_data:
      type: "boolean"
      default: true

# Workflow variables
variables:
  processed_count: 0
  failed_count: 0
  results: []

# Workflow steps
steps:
  # Step 1: Validate inputs
  - id: "validate_input"
    type: "script"
    script:
      language: "python"
      code: |
        def execute(ctx):
            if not ctx.input.user_ids:
                raise ValueError("user_ids cannot be empty")
            return {"valid": True, "count": len(ctx.input.user_ids)}
    on_error:
      action: "fail"
      message: "Input validation failed"

  # Step 2: Loop through user IDs
  - id: "process_users"
    type: "foreach"
    items: "{{ input.user_ids }}"
    item_var: "user_id"
    steps:
      # Step 2.1: Fetch user data via MCP tool
      - id: "fetch_user"
        type: "mcp_tool_call"
        tool: "get_user_by_id"
        arguments:
          user_id: "{{ user_id }}"
        output_var: "user_data"
        retry:
          max_attempts: 3
          backoff: "exponential"
          initial_delay: "1s"
        on_error:
          action: "continue"
          log_level: "warn"

      # Step 2.2: Conditional enrichment
      - id: "enrich_data"
        type: "if"
        condition: "{{ input.enrich_data == true }}"
        then:
          - id: "call_enrichment_api"
            type: "mcp_tool_call"
            tool: "enrich_user_data"
            arguments:
              user: "{{ user_data }}"
            output_var: "enriched_data"
        else:
          - id: "use_raw_data"
            type: "set_variable"
            variable: "enriched_data"
            value: "{{ user_data }}"

      # Step 2.3: Validate processed data
      - id: "validate_result"
        type: "script"
        script:
          language: "rhai"
          code: |
            fn execute(ctx) {
              let data = ctx.vars.enriched_data;
              if data.email == "" {
                return #{error: "Missing email"};
              }
              return #{valid: true};
            }

      # Step 2.4: Store result
      - id: "store_result"
        type: "script"
        script:
          language: "python"
          code: |
            def execute(ctx):
                ctx.vars.results.append(ctx.vars.enriched_data)
                ctx.vars.processed_count += 1
                return {"stored": True}

  # Step 3: Parallel processing for notifications
  - id: "send_notifications"
    type: "parallel"
    steps:
      - id: "send_email"
        type: "mcp_tool_call"
        tool: "send_email"
        arguments:
          to: "admin@example.com"
          subject: "Processing complete"
          body: "Processed {{ variables.processed_count }} users"

      - id: "update_metrics"
        type: "mcp_tool_call"
        tool: "update_metrics"
        arguments:
          metric: "users_processed"
          value: "{{ variables.processed_count }}"

  # Step 4: Return final results
  - id: "return_results"
    type: "return"
    value:
      success: true
      processed: "{{ variables.processed_count }}"
      failed: "{{ variables.failed_count }}"
      results: "{{ variables.results }}"
```

**Branching Example**

```yaml
# Switch statement example
steps:
  - id: "categorize_user"
    type: "switch"
    value: "{{ user.subscription_type }}"
    cases:
      "premium":
        - id: "premium_processing"
          type: "mcp_tool_call"
          tool: "process_premium_user"
          arguments:
            user: "{{ user }}"

      "basic":
        - id: "basic_processing"
          type: "mcp_tool_call"
          tool: "process_basic_user"
          arguments:
            user: "{{ user }}"

      default:
        - id: "free_processing"
          type: "script"
          script:
            language: "lua"
            code: |
              function execute(ctx)
                return {tier = "free", features = {"limited"}}
              end
```

**Looping Examples**

```yaml
# While loop
steps:
  - id: "retry_until_success"
    type: "while"
    condition: "{{ variables.retry_count < 5 and variables.success == false }}"
    steps:
      - id: "attempt_operation"
        type: "mcp_tool_call"
        tool: "unstable_operation"
        output_var: "result"

      - id: "check_result"
        type: "script"
        script:
          language: "python"
          code: |
            def execute(ctx):
                ctx.vars.retry_count += 1
                if ctx.vars.result.get("status") == "success":
                    ctx.vars.success = True
                return ctx.vars.result

      - id: "wait_before_retry"
        type: "sleep"
        duration: "{{ variables.retry_count * 2 }}s"

# For loop with range
steps:
  - id: "batch_processing"
    type: "for"
    range:
      start: 0
      end: 100
      step: 10
    index_var: "offset"
    steps:
      - id: "fetch_batch"
        type: "mcp_tool_call"
        tool: "get_users_paginated"
        arguments:
          offset: "{{ offset }}"
          limit: 10
```

**Sub-workflow Example**

```yaml
# Main workflow
steps:
  - id: "run_data_validation"
    type: "workflow"
    workflow: "data_validation"  # References another workflow file
    arguments:
      data: "{{ variables.raw_data }}"
    output_var: "validation_result"
```

**Workflow Execution API**

The workflow engine is exposed as an MCP tool:

```toml
# Auto-generated tool definition
[[tools]]
name = "workflow_data_processing"
description = "Execute Data Processing Pipeline workflow"

[tools.input_schema]
type = "object"
required = ["user_ids"]
properties = {
  user_ids = { type = "array", items = { type = "string" } },
  enrich_data = { type = "boolean", default = true }
}

[tools.mock]
strategy = "workflow"
workflow_file = "config/workflows/data_processing.yaml"
```

**Workflow Engine Implementation**

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use async_trait::async_trait;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub name: String,
    pub description: String,
    pub version: String,
    pub input_schema: serde_json::Value,
    pub variables: HashMap<String, serde_json::Value>,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Step {
    Script {
        id: String,
        script: ScriptConfig,
        #[serde(default)]
        output_var: Option<String>,
        #[serde(default)]
        on_error: ErrorHandler,
    },
    McpToolCall {
        id: String,
        tool: String,
        arguments: HashMap<String, String>,
        #[serde(default)]
        output_var: Option<String>,
        #[serde(default)]
        retry: Option<RetryConfig>,
        #[serde(default)]
        on_error: ErrorHandler,
    },
    If {
        id: String,
        condition: String,
        then: Vec<Step>,
        #[serde(default)]
        r#else: Vec<Step>,
    },
    Switch {
        id: String,
        value: String,
        cases: HashMap<String, Vec<Step>>,
        #[serde(default)]
        default: Vec<Step>,
    },
    ForEach {
        id: String,
        items: String,
        item_var: String,
        steps: Vec<Step>,
    },
    For {
        id: String,
        range: RangeConfig,
        index_var: String,
        steps: Vec<Step>,
    },
    While {
        id: String,
        condition: String,
        steps: Vec<Step>,
    },
    Parallel {
        id: String,
        steps: Vec<Step>,
    },
    Workflow {
        id: String,
        workflow: String,
        arguments: HashMap<String, serde_json::Value>,
        #[serde(default)]
        output_var: Option<String>,
    },
    SetVariable {
        id: String,
        variable: String,
        value: String,
    },
    Sleep {
        id: String,
        duration: String,
    },
    Return {
        id: String,
        value: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptConfig {
    pub language: String,
    pub code: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ErrorHandler {
    #[serde(default = "default_error_action")]
    pub action: ErrorAction,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ErrorAction {
    Fail,
    Continue,
    Retry,
}

fn default_error_action() -> ErrorAction {
    ErrorAction::Fail
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub backoff: BackoffStrategy,
    pub initial_delay: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackoffStrategy {
    Fixed,
    Linear,
    Exponential,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangeConfig {
    pub start: i64,
    pub end: i64,
    #[serde(default = "default_step")]
    pub step: i64,
}

fn default_step() -> i64 {
    1
}

pub struct WorkflowContext {
    pub input: serde_json::Value,
    pub variables: HashMap<String, serde_json::Value>,
    pub tool_handler: Arc<ToolHandler>,
    pub script_engine: Arc<ScriptEngine>,
}

pub struct WorkflowEngine {
    workflows: HashMap<String, WorkflowDefinition>,
    tool_handler: Arc<ToolHandler>,
    script_engine: Arc<ScriptEngine>,
}

impl WorkflowEngine {
    pub async fn execute(
        &self,
        workflow_name: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, WorkflowError> {
        let workflow = self.workflows.get(workflow_name)
            .ok_or_else(|| WorkflowError::NotFound(workflow_name.to_string()))?;

        let mut context = WorkflowContext {
            input,
            variables: workflow.variables.clone(),
            tool_handler: self.tool_handler.clone(),
            script_engine: self.script_engine.clone(),
        };

        self.execute_steps(&workflow.steps, &mut context).await
    }

    async fn execute_steps(
        &self,
        steps: &[Step],
        context: &mut WorkflowContext,
    ) -> Result<serde_json::Value, WorkflowError> {
        let mut last_result = serde_json::Value::Null;

        for step in steps {
            last_result = self.execute_step(step, context).await?;
        }

        Ok(last_result)
    }

    async fn execute_step(
        &self,
        step: &Step,
        context: &mut WorkflowContext,
    ) -> Result<serde_json::Value, WorkflowError> {
        match step {
            Step::Script { id, script, output_var, on_error } => {
                let result = self.execute_script_step(script, context, on_error).await?;
                if let Some(var) = output_var {
                    context.variables.insert(var.clone(), result.clone());
                }
                Ok(result)
            }

            Step::McpToolCall { id, tool, arguments, output_var, retry, on_error } => {
                let result = self.execute_tool_call(tool, arguments, context, retry, on_error).await?;
                if let Some(var) = output_var {
                    context.variables.insert(var.clone(), result.clone());
                }
                Ok(result)
            }

            Step::If { id, condition, then, r#else } => {
                let condition_result = self.evaluate_expression(condition, context)?;
                if condition_result.as_bool().unwrap_or(false) {
                    self.execute_steps(then, context).await
                } else {
                    self.execute_steps(r#else, context).await
                }
            }

            Step::Switch { id, value, cases, default } => {
                let switch_value = self.evaluate_expression(value, context)?;
                let switch_str = switch_value.as_str().unwrap_or("");

                if let Some(case_steps) = cases.get(switch_str) {
                    self.execute_steps(case_steps, context).await
                } else {
                    self.execute_steps(default, context).await
                }
            }

            Step::ForEach { id, items, item_var, steps } => {
                let items_value = self.evaluate_expression(items, context)?;
                let items_array = items_value.as_array()
                    .ok_or_else(|| WorkflowError::InvalidType("foreach items must be array"))?;

                let mut last_result = serde_json::Value::Null;
                for item in items_array {
                    context.variables.insert(item_var.clone(), item.clone());
                    last_result = self.execute_steps(steps, context).await?;
                }
                Ok(last_result)
            }

            Step::While { id, condition, steps } => {
                let mut last_result = serde_json::Value::Null;
                while self.evaluate_expression(condition, context)?.as_bool().unwrap_or(false) {
                    last_result = self.execute_steps(steps, context).await?;
                }
                Ok(last_result)
            }

            Step::Parallel { id, steps } => {
                let futures: Vec<_> = steps.iter()
                    .map(|step| {
                        let mut ctx = context.clone();
                        self.execute_step(step, &mut ctx)
                    })
                    .collect();

                let results = futures::future::try_join_all(futures).await?;
                Ok(serde_json::json!(results))
            }

            Step::Return { id, value } => {
                Ok(value.clone())
            }

            _ => Ok(serde_json::Value::Null),
        }
    }

    async fn execute_tool_call(
        &self,
        tool_name: &str,
        arguments: &HashMap<String, String>,
        context: &WorkflowContext,
        retry_config: &Option<RetryConfig>,
        error_handler: &ErrorHandler,
    ) -> Result<serde_json::Value, WorkflowError> {
        // Resolve template arguments
        let resolved_args: HashMap<String, serde_json::Value> = arguments
            .iter()
            .map(|(k, v)| {
                let resolved = self.evaluate_expression(v, context).unwrap_or_default();
                (k.clone(), resolved)
            })
            .collect();

        // Execute with retry logic
        let max_attempts = retry_config.as_ref().map(|r| r.max_attempts).unwrap_or(1);
        let mut attempt = 0;

        loop {
            attempt += 1;

            match context.tool_handler.execute_tool(tool_name, &resolved_args).await {
                Ok(result) => return Ok(result),
                Err(e) if attempt < max_attempts => {
                    let delay = self.calculate_backoff_delay(retry_config, attempt);
                    tokio::time::sleep(delay).await;
                    continue;
                }
                Err(e) => {
                    return match error_handler.action {
                        ErrorAction::Fail => Err(WorkflowError::ToolExecutionFailed(e.to_string())),
                        ErrorAction::Continue => Ok(serde_json::json!({"error": e.to_string()})),
                        ErrorAction::Retry => Err(WorkflowError::MaxRetriesExceeded),
                    };
                }
            }
        }
    }

    fn evaluate_expression(
        &self,
        expr: &str,
        context: &WorkflowContext,
    ) -> Result<serde_json::Value, WorkflowError> {
        // Template variable substitution using Handlebars/Tera
        let template = handlebars::Handlebars::new();
        let data = serde_json::json!({
            "input": context.input,
            "variables": context.variables,
        });

        let result = template.render_template(expr, &data)
            .map_err(|e| WorkflowError::ExpressionEvaluation(e.to_string()))?;

        serde_json::from_str(&result)
            .or_else(|_| Ok(serde_json::Value::String(result)))
    }

    fn calculate_backoff_delay(&self, retry_config: &Option<RetryConfig>, attempt: u32) -> Duration {
        let config = match retry_config {
            Some(c) => c,
            None => return Duration::from_secs(1),
        };

        let base_delay = parse_duration(&config.initial_delay).unwrap_or(Duration::from_secs(1));

        match config.backoff {
            BackoffStrategy::Fixed => base_delay,
            BackoffStrategy::Linear => base_delay * attempt,
            BackoffStrategy::Exponential => base_delay * 2u32.pow(attempt - 1),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WorkflowError {
    #[error("Workflow not found: {0}")]
    NotFound(String),

    #[error("Invalid type: {0}")]
    InvalidType(&'static str),

    #[error("Expression evaluation failed: {0}")]
    ExpressionEvaluation(String),

    #[error("Tool execution failed: {0}")]
    ToolExecutionFailed(String),

    #[error("Maximum retries exceeded")]
    MaxRetriesExceeded,
}
```

**Workflow Visualization**

```rust
pub struct WorkflowVisualizer;

impl WorkflowVisualizer {
    pub fn generate_dag(&self, workflow: &WorkflowDefinition) -> String {
        // Generate Mermaid or DOT format diagram
        let mut output = String::from("graph TD\n");

        for (i, step) in workflow.steps.iter().enumerate() {
            output.push_str(&format!("    Step{}[{}]\n", i, self.step_label(step)));

            if i > 0 {
                output.push_str(&format!("    Step{} --> Step{}\n", i - 1, i));
            }
        }

        output
    }
}
```

**Testing Strategy**

```rust
#[cfg(test)]
mod workflow_tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_workflow_execution() {
        let workflow = WorkflowDefinition {
            name: "test".into(),
            steps: vec![
                Step::SetVariable {
                    id: "set_x".into(),
                    variable: "x".into(),
                    value: "10".into(),
                },
                Step::Return {
                    id: "return".into(),
                    value: json!({"x": "{{ variables.x }}"}),
                },
            ],
            // ...
        };

        let engine = WorkflowEngine::new(/* ... */);
        let result = engine.execute("test", json!({})).await.unwrap();

        assert_eq!(result["x"], json!("10"));
    }

    #[tokio::test]
    async fn test_conditional_branching() {
        let workflow = create_test_workflow_with_if();
        let engine = WorkflowEngine::new(/* ... */);

        // Test true branch
        let result = engine.execute("test", json!({"condition": true})).await.unwrap();
        assert_eq!(result["branch"], "then");

        // Test false branch
        let result = engine.execute("test", json!({"condition": false})).await.unwrap();
        assert_eq!(result["branch"], "else");
    }

    #[tokio::test]
    async fn test_foreach_loop() {
        let workflow = WorkflowDefinition {
            steps: vec![
                Step::ForEach {
                    id: "loop".into(),
                    items: "{{ input.items }}".into(),
                    item_var: "item".into(),
                    steps: vec![
                        Step::Script {
                            id: "process".into(),
                            script: ScriptConfig {
                                language: "python".into(),
                                code: "def execute(ctx): return ctx.vars['item'] * 2".into(),
                            },
                            output_var: Some("result".into()),
                            on_error: ErrorHandler::default(),
                        },
                    ],
                },
            ],
            // ...
        };

        let result = engine.execute("test", json!({"items": [1, 2, 3]})).await.unwrap();
        // Verify loop executed 3 times
    }

    #[tokio::test]
    async fn test_retry_with_exponential_backoff() {
        let mut call_count = 0;

        let workflow = WorkflowDefinition {
            steps: vec![
                Step::McpToolCall {
                    id: "unstable_call".into(),
                    tool: "unstable_tool".into(),
                    arguments: HashMap::new(),
                    output_var: None,
                    retry: Some(RetryConfig {
                        max_attempts: 3,
                        backoff: BackoffStrategy::Exponential,
                        initial_delay: "100ms".into(),
                    }),
                    on_error: ErrorHandler::default(),
                },
            ],
            // ...
        };

        // Mock tool that fails first 2 times
        // Verify it succeeds on 3rd attempt
    }

    #[tokio::test]
    async fn test_parallel_execution() {
        use std::time::Instant;

        let workflow = WorkflowDefinition {
            steps: vec![
                Step::Parallel {
                    id: "parallel".into(),
                    steps: vec![
                        Step::Sleep { id: "s1".into(), duration: "100ms".into() },
                        Step::Sleep { id: "s2".into(), duration: "100ms".into() },
                        Step::Sleep { id: "s3".into(), duration: "100ms".into() },
                    ],
                },
            ],
            // ...
        };

        let start = Instant::now();
        engine.execute("test", json!({})).await.unwrap();
        let duration = start.elapsed();

        // Should take ~100ms, not 300ms (parallel execution)
        assert!(duration < Duration::from_millis(150));
    }

    #[tokio::test]
    async fn test_error_handling_continue() {
        let workflow = WorkflowDefinition {
            steps: vec![
                Step::McpToolCall {
                    id: "failing_tool".into(),
                    tool: "tool_that_fails".into(),
                    arguments: HashMap::new(),
                    output_var: Some("result".into()),
                    retry: None,
                    on_error: ErrorHandler {
                        action: ErrorAction::Continue,
                        message: Some("Tool failed".into()),
                        log_level: "warn".into(),
                    },
                },
                Step::Return {
                    id: "return".into(),
                    value: json!({"completed": true}),
                },
            ],
            // ...
        };

        // Should complete despite error
        let result = engine.execute("test", json!({})).await.unwrap();
        assert_eq!(result["completed"], true);
    }

    proptest! {
        #[test]
        fn test_expression_evaluation_safety(expr in ".*") {
            let context = WorkflowContext::default();
            let engine = WorkflowEngine::new(/* ... */);

            // Should never panic, even with malformed expressions
            let _ = engine.evaluate_expression(&expr, &context);
        }
    }
}
```

**Benefits**
- **Complex Orchestration**: Handle multi-step processes with ease
- **Reusability**: Share workflows across projects
- **Visibility**: Clear visualization of execution flow
- **Reliability**: Built-in retry and error handling
- **Flexibility**: Mix MCP tools and scripts in workflows
- **Performance**: Parallel execution for independent steps
- **Testability**: Workflows are declarative and easy to test

### 7. Authentication System

**Supported Methods**
- None (default, no authentication)
- API Key (header or query parameter)
- Bearer Token (JWT)
- Basic Auth
- OAuth 2.0
- Custom (script-based validation)
- mTLS (mutual TLS)

**Configuration Example**
```toml
[auth]
enabled = true
mode = "api_key"  # none, api_key, bearer, basic, oauth2, custom, mtls

[auth.api_key]
header = "X-API-Key"
valid_keys = ["key1", "key2"]
# Or load from file/env
keys_file = "keys.txt"
keys_env = "METIS_API_KEYS"

[auth.bearer]
jwt_secret = "your-secret"
jwt_algorithm = "HS256"
validate_expiry = true
required_claims = { sub = true, scope = ["read", "write"] }

[auth.custom]
script = """
fn validate(request) {
    return request.headers.get("X-Custom-Auth") == "valid";
}
"""
```

---

## Web UI (Leptos)

### Overview

Metis includes a modern, reactive web interface built with **Leptos** (Rust full-stack framework) for managing and configuring all aspects of the mock server with live reloading capabilities. The UI provides a visual way to create, edit, and test resources, tools, prompts, models, agents, and workflows without manually editing configuration files.

**Key Features**
- **Live Configuration Editing**: Edit YAML/JSON/TOML configs with syntax highlighting and validation
- **Hot Reload**: Changes reflect immediately without server restart
- **Visual Workflow Designer**: Drag-and-drop workflow builder with DAG visualization
- **Request Testing**: Built-in HTTP/MCP client for testing endpoints
- **Resource Browser**: Browse and search all configured resources, tools, and prompts
- **Agent Dashboard**: Monitor agent execution and view conversation history
- **Model Explorer**: Visualize model relationships and dependencies
- **Metrics & Monitoring**: Real-time performance metrics and request logs
- **Dark/Light Theme**: Modern, responsive UI with theme support
- **Export/Import**: Export configurations as shareable packages

### Technology Stack

```toml
[dependencies]
leptos = "0.6"
leptos_router = "0.6"
leptos_meta = "0.6"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"
toml = "0.8"
monaco = "0.1"  # Code editor component
mermaid-rs = "0.2"  # Workflow visualization
tailwindcss = "3.0"  # Styling
axum = "0.7"  # Backend API
tower-http = { version = "0.5", features = ["cors", "fs"] }
```

### Architecture (Hexagonal Pattern)

```
┌─────────────────────────────────────────────────────────┐
│                   Presentation Layer                     │
│  ┌────────────────────────────────────────────────────┐ │
│  │         Leptos Components (UI)                     │ │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────────────┐  │ │
│  │  │Dashboard │ │ Config   │ │ Workflow Designer│  │ │
│  │  │          │ │ Editor   │ │                  │  │ │
│  │  └──────────┘ └──────────┘ └──────────────────┘  │ │
│  └────────────────────────────────────────────────────┘ │
└───────────────────────┬─────────────────────────────────┘
                        │ HTTP API / WebSocket
┌───────────────────────▼─────────────────────────────────┐
│               Application Layer (Ports)                  │
│  ┌────────────────────────────────────────────────────┐ │
│  │  ConfigManagementPort  │  WorkflowExecutionPort   │ │
│  │  ResourceQueryPort     │  MetricsCollectionPort   │ │
│  └────────────────────────────────────────────────────┘ │
└───────────────────────┬─────────────────────────────────┘
                        │
┌───────────────────────▼─────────────────────────────────┐
│              Domain Layer (Business Logic)               │
│  ┌────────────────────────────────────────────────────┐ │
│  │  ConfigValidator    │  WorkflowOrchestrator       │ │
│  │  ResourceRegistry   │  AgentManager               │ │
│  └────────────────────────────────────────────────────┘ │
└───────────────────────┬─────────────────────────────────┘
                        │
┌───────────────────────▼─────────────────────────────────┐
│           Infrastructure Layer (Adapters)                │
│  ┌────────────────────────────────────────────────────┐ │
│  │  FileSystemAdapter  │  MCPServerAdapter           │ │
│  │  DatabaseAdapter    │  CacheAdapter               │ │
│  └────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

### UI Components

#### 1. Dashboard

```rust
use leptos::*;

#[component]
pub fn Dashboard() -> impl IntoView {
    let (stats, set_stats) = create_signal(ServerStats::default());

    // WebSocket connection for real-time updates
    create_effect(move |_| {
        // Subscribe to metrics updates
        spawn_local(async move {
            let mut ws = connect_websocket("/api/ws/metrics").await;
            while let Some(msg) = ws.next().await {
                set_stats.set(msg.parse().unwrap());
            }
        });
    });

    view! {
        <div class="dashboard">
            <h1>"Metis Mock Server"</h1>

            <div class="stats-grid">
                <StatCard
                    title="Total Requests"
                    value=move || stats().total_requests
                    icon="📊"
                />
                <StatCard
                    title="Active Agents"
                    value=move || stats().active_agents
                    icon="🤖"
                />
                <StatCard
                    title="Avg Response Time"
                    value=move || format!("{}ms", stats().avg_response_time)
                    icon="⚡"
                />
                <StatCard
                    title="Success Rate"
                    value=move || format!("{}%", stats().success_rate)
                    icon="✅"
                />
            </div>

            <RecentRequests />
            <ActiveWorkflows />
        </div>
    }
}
```

#### 2. Configuration Editor

```rust
#[component]
pub fn ConfigEditor() -> impl IntoView {
    let (config_type, set_config_type) = create_signal("resources");
    let (selected_file, set_selected_file) = create_signal(None::<String>);
    let (content, set_content) = create_signal(String::new());
    let (syntax_errors, set_syntax_errors) = create_signal(Vec::<String>::new());

    let save_config = create_action(|content: &String| {
        let content = content.clone();
        async move {
            // Call API to save and validate
            let response = post_json("/api/config/save", &content).await;
            match response {
                Ok(_) => {
                    // Trigger hot reload
                    post("/api/config/reload").await.ok();
                    show_toast("Configuration saved and reloaded");
                }
                Err(e) => show_toast(&format!("Error: {}", e)),
            }
        }
    });

    view! {
        <div class="config-editor">
            <div class="sidebar">
                <h3>"Configuration Files"</h3>
                <FileTree
                    root="config/"
                    on_select=move |path| {
                        set_selected_file.set(Some(path.clone()));
                        // Load file content
                        spawn_local(async move {
                            let content = fetch_file(&path).await.unwrap();
                            set_content.set(content);
                        });
                    }
                />

                <button on:click=move |_| {
                    // Create new config file dialog
                    show_new_file_dialog();
                }>
                    "+ New File"
                </button>
            </div>

            <div class="editor-panel">
                <div class="toolbar">
                    <select on:change=move |ev| {
                        set_config_type.set(event_target_value(&ev));
                    }>
                        <option value="resources">"Resources"</option>
                        <option value="tools">"Tools"</option>
                        <option value="prompts">"Prompts"</option>
                        <option value="models">"Models"</option>
                        <option value="agents">"Agents"</option>
                        <option value="workflows">"Workflows"</option>
                    </select>

                    <button
                        on:click=move |_| save_config.dispatch(content())
                        disabled=move || syntax_errors().len() > 0
                    >
                        "💾 Save & Reload"
                    </button>

                    <button on:click=move |_| {
                        // Format code
                        format_config(&content());
                    }>
                        "✨ Format"
                    </button>
                </div>

                <MonacoEditor
                    value=content
                    on_change=move |new_content| {
                        set_content.set(new_content.clone());
                        // Validate syntax in real-time
                        spawn_local(async move {
                            let errors = validate_config(&new_content).await.unwrap_or_default();
                            set_syntax_errors.set(errors);
                        });
                    }
                    language=move || match config_type().as_str() {
                        "workflows" => "yaml",
                        _ => detect_language(&selected_file()),
                    }
                />

                <Show when=move || !syntax_errors().is_empty()>
                    <div class="error-panel">
                        <h4>"Syntax Errors"</h4>
                        <ul>
                            <For
                                each=move || syntax_errors()
                                key=|error| error.clone()
                                children=move |error| {
                                    view! { <li class="error">{error}</li> }
                                }
                            />
                        </ul>
                    </div>
                </Show>
            </div>

            <div class="preview-panel">
                <h3>"Live Preview"</h3>
                <ConfigPreview config_content=content />
            </div>
        </div>
    }
}
```

#### 3. Visual Workflow Designer

```rust
#[component]
pub fn WorkflowDesigner() -> impl IntoView {
    let (workflow, set_workflow) = create_signal(WorkflowDefinition::default());
    let (selected_step, set_selected_step) = create_signal(None::<String>);

    view! {
        <div class="workflow-designer">
            <div class="toolbar">
                <button on:click=move |_| {
                    // Add step dialog
                    show_add_step_dialog();
                }>
                    "+ Add Step"
                </button>

                <button on:click=move |_| {
                    // Validate workflow
                    spawn_local(async move {
                        validate_workflow(&workflow()).await;
                    });
                }>
                    "✓ Validate"
                </button>

                <button on:click=move |_| {
                    // Test run workflow
                    spawn_local(async move {
                        test_workflow(&workflow()).await;
                    });
                }>
                    "▶ Test Run"
                </button>

                <button on:click=move |_| {
                    // Export workflow to YAML
                    download_workflow(&workflow());
                }>
                    "⬇ Export"
                </button>
            </div>

            <div class="canvas">
                <WorkflowCanvas
                    workflow=workflow
                    on_step_select=move |step_id| {
                        set_selected_step.set(Some(step_id));
                    }
                    on_workflow_change=move |new_workflow| {
                        set_workflow.set(new_workflow);
                    }
                />
            </div>

            <div class="properties-panel">
                <Show when=move || selected_step().is_some()>
                    <StepProperties
                        step_id=move || selected_step().unwrap()
                        workflow=workflow
                        on_update=move |updated_step| {
                            // Update step in workflow
                        }
                    />
                </Show>
            </div>

            <div class="minimap">
                <MermaidDiagram
                    definition=move || generate_mermaid(&workflow())
                />
            </div>
        </div>
    }
}
```

#### 4. Resource Browser

```rust
#[component]
pub fn ResourceBrowser() -> impl IntoView {
    let (resources, set_resources) = create_signal(Vec::<Resource>::new());
    let (search, set_search) = create_signal(String::new());
    let (selected, set_selected) = create_signal(None::<Resource>);

    // Load resources on mount
    create_effect(move |_| {
        spawn_local(async move {
            let loaded = fetch_resources().await.unwrap();
            set_resources.set(loaded);
        });
    });

    let filtered_resources = move || {
        resources()
            .into_iter()
            .filter(|r| {
                let search_lower = search().to_lowercase();
                r.name.to_lowercase().contains(&search_lower) ||
                r.uri.to_lowercase().contains(&search_lower)
            })
            .collect::<Vec<_>>()
    };

    view! {
        <div class="resource-browser">
            <div class="search-bar">
                <input
                    type="text"
                    placeholder="Search resources..."
                    on:input=move |ev| {
                        set_search.set(event_target_value(&ev));
                    }
                />
            </div>

            <div class="resource-list">
                <For
                    each=filtered_resources
                    key=|resource| resource.uri.clone()
                    children=move |resource| {
                        view! {
                            <div
                                class="resource-card"
                                on:click=move |_| {
                                    set_selected.set(Some(resource.clone()));
                                }
                            >
                                <h4>{&resource.name}</h4>
                                <p class="uri">{&resource.uri}</p>
                                <span class="badge">{&resource.mime_type}</span>
                                <span class="strategy-badge">
                                    {format!("Strategy: {}", resource.mock_strategy)}
                                </span>
                            </div>
                        }
                    }
                />
            </div>

            <Show when=move || selected().is_some()>
                <div class="resource-detail">
                    <ResourceDetail resource=move || selected().unwrap() />

                    <button on:click=move |_| {
                        // Test resource
                        spawn_local(async move {
                            let result = test_resource(&selected().unwrap()).await;
                            show_result_modal(result);
                        });
                    }>
                        "🧪 Test Resource"
                    </button>
                </div>
            </Show>
        </div>
    }
}
```

#### 5. Agent Dashboard

```rust
#[component]
pub fn AgentDashboard() -> impl IntoView {
    let (agents, set_agents) = create_signal(Vec::<AgentInfo>::new());
    let (conversations, set_conversations) = create_signal(Vec::<Conversation>::new());

    view! {
        <div class="agent-dashboard">
            <div class="agents-panel">
                <h2>"Configured Agents"</h2>
                <For
                    each=move || agents()
                    key=|agent| agent.id.clone()
                    children=move |agent| {
                        view! {
                            <div class="agent-card">
                                <h3>{&agent.name}</h3>
                                <p>{&agent.description}</p>
                                <span class="status" class:active=agent.is_active>
                                    {if agent.is_active { "🟢 Active" } else { "⚫ Inactive" }}
                                </span>
                                <div class="agent-stats">
                                    <span>"Calls: " {agent.total_calls}</span>
                                    <span>"Avg: " {agent.avg_duration} "ms"</span>
                                </div>
                                <button on:click=move |_| {
                                    spawn_local(async move {
                                        test_agent(&agent.id).await;
                                    });
                                }>
                                    "Test"
                                </button>
                            </div>
                        }
                    }
                />
            </div>

            <div class="conversations-panel">
                <h2>"Recent Conversations"</h2>
                <For
                    each=move || conversations()
                    key=|conv| conv.id.clone()
                    children=move |conv| {
                        view! {
                            <div class="conversation-card">
                                <div class="conv-header">
                                    <span class="agent-name">{&conv.agent_name}</span>
                                    <span class="timestamp">{&conv.timestamp}</span>
                                </div>
                                <div class="messages">
                                    <For
                                        each=move || conv.messages.clone()
                                        key=|msg| msg.id.clone()
                                        children=move |msg| {
                                            view! {
                                                <div class=format!("message {}", msg.role)>
                                                    <strong>{&msg.role}":"</strong>
                                                    <span>{&msg.content}</span>
                                                </div>
                                            }
                                        }
                                    />
                                </div>
                            </div>
                        }
                    }
                />
            </div>
        </div>
    }
}
```

### API Layer (Axum) - Following Hexagonal Architecture

```rust
use axum::{
    Router,
    routing::{get, post},
    extract::{State, Path, Json},
    response::IntoResponse,
};
use tower_http::cors::CorsLayer;

// Port definitions (Hexagonal Architecture)
#[async_trait]
pub trait ConfigManagementPort: Send + Sync {
    async fn list_files(&self, dir: &str) -> Result<Vec<String>, Error>;
    async fn get_file(&self, path: &str) -> Result<String, Error>;
    async fn save_file(&self, path: &str, content: &str) -> Result<(), Error>;
    async fn reload(&self) -> Result<(), Error>;
    async fn validate(&self, content: &str) -> Result<Vec<String>, Error>;
}

#[async_trait]
pub trait WorkflowExecutionPort: Send + Sync {
    async fn execute(&self, name: &str, input: serde_json::Value) -> Result<serde_json::Value, Error>;
    async fn validate(&self, definition: &WorkflowDefinition) -> Result<Vec<String>, Error>;
}

#[async_trait]
pub trait ResourceQueryPort: Send + Sync {
    async fn list_all(&self) -> Result<Vec<ResourceInfo>, Error>;
    async fn get_by_uri(&self, uri: &str) -> Result<ResourceInfo, Error>;
    async fn test(&self, uri: &str) -> Result<serde_json::Value, Error>;
}

#[async_trait]
pub trait MetricsCollectionPort: Send + Sync {
    async fn get_current_metrics(&self) -> ServerStats;
    fn subscribe_updates(&self) -> tokio::sync::broadcast::Receiver<ServerStats>;
}

pub struct AppState {
    config_manager: Arc<dyn ConfigManagementPort>,
    workflow_engine: Arc<dyn WorkflowExecutionPort>,
    resource_registry: Arc<dyn ResourceQueryPort>,
    metrics_collector: Arc<dyn MetricsCollectionPort>,
}

pub fn create_ui_router(state: AppState) -> Router {
    Router::new()
        // Static files
        .nest_service("/", ServeDir::new("ui/dist"))

        // API routes
        .route("/api/config/list", get(list_config_files))
        .route("/api/config/:path", get(get_config_file))
        .route("/api/config/save", post(save_config_file))
        .route("/api/config/reload", post(reload_config))
        .route("/api/config/validate", post(validate_config))

        .route("/api/resources", get(get_all_resources))
        .route("/api/resources/:uri/test", post(test_resource))

        .route("/api/tools", get(get_all_tools))
        .route("/api/tools/:name/test", post(test_tool))

        .route("/api/workflows", get(get_all_workflows))
        .route("/api/workflows/:name/execute", post(execute_workflow))
        .route("/api/workflows/:name/validate", post(validate_workflow))

        .route("/api/agents", get(get_all_agents))
        .route("/api/agents/:id/conversations", get(get_agent_conversations))

        .route("/api/metrics", get(get_metrics))
        .route("/api/ws/metrics", get(websocket_metrics))

        .layer(CorsLayer::permissive())
        .with_state(state)
}

// Handlers
async fn save_config_file(
    State(state): State<AppState>,
    Path(path): Path<String>,
    Json(content): Json<String>,
) -> impl IntoResponse {
    match state.config_manager.save_file(&path, &content).await {
        Ok(_) => {
            // Trigger hot reload
            state.config_manager.reload().await.ok();
            Json(json!({"status": "success"}))
        }
        Err(e) => Json(json!({"status": "error", "message": e.to_string()})),
    }
}

async fn reload_config(
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state.config_manager.reload().await {
        Ok(_) => Json(json!({"status": "reloaded"})),
        Err(e) => Json(json!({"status": "error", "message": e.to_string()})),
    }
}

async fn websocket_metrics(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_metrics_websocket(socket, state))
}

async fn handle_metrics_websocket(
    mut socket: WebSocket,
    state: AppState,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));

    loop {
        interval.tick().await;

        let metrics = state.metrics_collector.get_current_metrics().await;
        let json = serde_json::to_string(&metrics).unwrap();

        if socket.send(Message::Text(json)).await.is_err() {
            break;
        }
    }
}
```

### Configuration Hot Reload

```rust
pub struct ConfigWatcher {
    watcher: RecommendedWatcher,
    reload_tx: tokio::sync::broadcast::Sender<ConfigReloadEvent>,
}

impl ConfigWatcher {
    pub fn new(config_dir: PathBuf) -> Result<Self, Error> {
        let (reload_tx, _) = tokio::sync::broadcast::channel(100);
        let tx = reload_tx.clone();

        let watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    if event.kind.is_modify() {
                        tx.send(ConfigReloadEvent {
                            path: event.paths[0].clone(),
                            timestamp: Instant::now(),
                        }).ok();
                    }
                }
            },
            Config::default(),
        )?;

        watcher.watch(&config_dir, RecursiveMode::Recursive)?;

        Ok(Self { watcher, reload_tx })
    }

    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<ConfigReloadEvent> {
        self.reload_tx.subscribe()
    }
}
```

### Testing Strategy

```rust
#[cfg(test)]
mod ui_tests {
    use super::*;

    #[tokio::test]
    async fn test_config_save_and_reload() {
        let app = create_test_app().await;

        let new_config = r#"
            name: "test_resource"
            uri: "test://example"
        "#;

        // Save config via API
        let response = app
            .post("/api/config/resources/test.yaml")
            .json(&new_config)
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);

        // Verify auto-reload happened
        tokio::time::sleep(Duration::from_millis(100)).await;

        let resources = app.get("/api/resources").await.unwrap();
        assert!(resources.iter().any(|r| r.name == "test_resource"));
    }

    #[tokio::test]
    async fn test_workflow_execution_via_ui() {
        let app = create_test_app().await;

        let workflow_input = json!({
            "user_ids": ["user1", "user2"]
        });

        let response = app
            .post("/api/workflows/data_processing/execute")
            .json(&workflow_input)
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);

        let result: serde_json::Value = response.json().await.unwrap();
        assert_eq!(result["success"], true);
    }

    #[tokio::test]
    async fn test_websocket_metrics_stream() {
        let app = create_test_app().await;

        let mut ws_client = app.websocket("/api/ws/metrics").await.unwrap();

        // Receive first metrics update
        let msg = ws_client.receive().await.unwrap();
        let metrics: ServerStats = serde_json::from_str(&msg).unwrap();

        assert!(metrics.total_requests >= 0);
    }

    #[test]
    fn test_leptos_component_rendering() {
        // Component test
        let dashboard = Dashboard();
        let html = leptos::ssr::render_to_string(|| dashboard);

        assert!(html.contains("Metis Mock Server"));
        assert!(html.contains("Total Requests"));
    }
}
```

### Deployment

```dockerfile
# Dockerfile for Metis with UI
FROM rust:1.75 as builder

WORKDIR /app

# Build backend
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --features ui

# Build frontend
COPY ui ./ui
RUN cd ui && trunk build --release

FROM debian:bookworm-slim

COPY --from=builder /app/target/release/metis /usr/local/bin/
COPY --from=builder /app/ui/dist /usr/local/share/metis/ui

EXPOSE 3000 8080

CMD ["metis", "serve", "--ui-port", "8080", "--mcp-port", "3000"]
```

### Benefits
- **Visual Configuration**: No need to manually edit YAML/JSON files
- **Live Feedback**: See changes immediately without restart
- **Workflow Visualization**: Understand complex workflows at a glance
- **Testing Integration**: Test resources, tools, and workflows from the UI
- **Real-time Monitoring**: Track performance and debug issues
- **Developer Friendly**: Modern, intuitive interface built with Rust
- **Type Safety**: Leptos provides compile-time guarantees
- **Performance**: Compiled to WebAssembly for native speed
- **Hexagonal Architecture**: Clear separation of concerns with ports and adapters
- **SOLID Principles**: Dependency inversion through port interfaces

---

## Mock Data Generation Strategies

### 1. Random Generation

**Use Case**: Quick prototyping, load testing, unpredictable data

**Features**
- Generate random primitives (strings, numbers, booleans)
- Fake data (names, addresses, emails, etc.)
- Random selection from lists
- Distribution controls (uniform, normal, weighted)

**Configuration**
```toml
[mock]
strategy = "random"

[mock.random]
type = "object"
schema = {
    id = { type = "uuid" },
    name = { type = "fake", fake_type = "name.full_name" },
    age = { type = "integer", min = 18, max = 80 },
    email = { type = "fake", fake_type = "internet.email" },
    status = { type = "choice", values = ["active", "inactive"], weights = [0.8, 0.2] }
}
```

### 2. Template-Based Generation

**Use Case**: Structured responses, consistent patterns, parameterized data

**Features**
- Handlebars/Tera template engine
- Variable substitution
- Control flow (if/else, loops)
- Helper functions
- Template inheritance

**Configuration**
```toml
[mock]
strategy = "template"
template_file = "templates/response.json"
template_engine = "tera"  # or "handlebars"

[mock.variables]
user_count = { type = "integer", min = 10, max = 100 }
timestamp = { type = "now" }
environment = { type = "static", value = "production" }
```

**Template Example**
```jinja2
{
  "users": [
    {% for i in range(end=user_count) %}
    {
      "id": "{{ uuid() }}",
      "name": "{{ fake_name() }}",
      "created_at": "{{ timestamp }}"
    }{% if not loop.last %},{% endif %}
    {% endfor %}
  ],
  "total": {{ user_count }},
  "environment": "{{ environment }}"
}
```

### 3. LLM-Generated Responses

**Use Case**: Natural language content, creative responses, context-aware data

**Features**
- Multiple LLM providers (OpenAI, Anthropic, local models)
- Prompt templates
- Response caching
- Fallback strategies
- Cost controls

**Configuration**
```toml
[mock]
strategy = "llm"

[mock.llm]
provider = "anthropic"  # openai, anthropic, ollama, custom
model = "claude-3-5-sonnet-20241022"
api_key_env = "ANTHROPIC_API_KEY"
temperature = 0.7
max_tokens = 1000

[mock.llm.prompt]
system = "You are a helpful assistant generating mock data"
user_template = "Generate a {{data_type}} with these properties: {{properties}}"

[mock.llm.cache]
enabled = true
ttl_seconds = 3600
cache_key_fields = ["data_type", "properties"]

[mock.llm.cost_control]
max_requests_per_minute = 10
max_cost_per_hour_usd = 5.0
```

### 4. Script-Generated Responses

**Use Case**: Complex logic, conditional responses, stateful behavior

**Features**
- Embedded scripting (Rhai or Lua)
- Access to request context
- Stateful execution
- Custom business logic
- Helper functions

**Configuration**
```toml
[mock]
strategy = "script"
script_file = "scripts/generate_response.rhai"
# Or inline
script = """
fn generate(ctx) {
    let response = #{};

    if ctx.input.type == "user" {
        response.id = uuid();
        response.name = fake_name();
        response.role = choose(["admin", "user"], [0.1, 0.9]);
    } else {
        response.error = "Unknown type";
    }

    return response;
}
"""

[mock.script.context]
# Additional context available to scripts
session = true
request_history = true
custom_state = { counter = 0 }
```

### 5. Pattern-Based Generation

**Use Case**: Regex patterns, structured formats, validation testing

**Features**
- Generate strings matching regex patterns
- Support for named groups
- Multiple pattern strategies
- Format validation

**Configuration**
```toml
[mock]
strategy = "pattern"

[mock.pattern]
format = "regex"
pattern = "^[A-Z]{3}-\\d{4}-[a-f0-9]{8}$"
# Or predefined patterns
# format = "email" | "url" | "ipv4" | "uuid" | "phone" | "credit_card"

[mock.pattern.examples]
# Ensure generated values match these examples
samples = ["ABC-1234-deadbeef", "XYZ-5678-cafebabe"]
```

### 6. Database-Backed Responses

**Use Case**: Real data, complex queries, integration testing

**Features**
- Support multiple databases (PostgreSQL, MySQL, SQLite)
- Query templates
- Connection pooling
- Query result transformation

**Configuration**
```toml
[mock]
strategy = "database"

[mock.database]
driver = "postgres"  # postgres, mysql, sqlite
connection_string = "postgresql://user:pass@localhost/dbname"
# Or use env
connection_string_env = "DATABASE_URL"

[mock.database.query]
sql = """
SELECT id, name, email, created_at
FROM users
WHERE status = $1
ORDER BY created_at DESC
LIMIT $2
"""
parameters = [
    { value = "active", type = "string" },
    { value = 10, type = "integer" }
]

[mock.database.connection_pool]
max_connections = 10
min_connections = 2
connection_timeout_sec = 5
```

### 7. File-Based Responses

**Use Case**: Static content, test fixtures, recorded responses

**Features**
- Load from JSON/YAML/TOML files
- Directory scanning
- File selection strategies (sequential, random, weighted)
- Template processing of files

**Configuration**
```toml
[mock]
strategy = "file"

[mock.file]
path = "fixtures/responses/*.json"
selection = "random"  # sequential, random, weighted, round_robin
template_processing = true  # Process files as templates

[mock.file.rotation]
enabled = true
reset_on_exhaustion = true  # For sequential mode
```

### 8. Static Responses

**Use Case**: Simple, fixed responses, health checks, constant values

**Configuration**
```toml
[mock]
strategy = "static"

[mock.static]
content = { status = "ok", version = "1.0.0" }
# Or
content_file = "static/health_response.json"
```

### 9. Hybrid/Composite Strategies

**Use Case**: Combine multiple strategies, fallback chains, A/B testing

**Configuration**
```toml
[mock]
strategy = "composite"

[mock.composite]
mode = "fallback"  # fallback, weighted, sequential, conditional

# Fallback chain
[[mock.composite.strategies]]
strategy = "database"
timeout_ms = 100
on_error = "next"

[[mock.composite.strategies]]
strategy = "llm"
timeout_ms = 500
on_error = "next"

[[mock.composite.strategies]]
strategy = "template"
# Final fallback

# Or weighted (A/B testing)
[mock.composite.weighted]
strategies = [
    { strategy = "llm", weight = 0.1 },
    { strategy = "template", weight = 0.9 }
]
```

---

## Configuration System

### Configuration File Structure

**Main Configuration File** (`metis.toml`)

```toml
[server]
name = "Metis Mock Server"
version = "1.0.0"
host = "127.0.0.1"
port = 3000
log_level = "info"  # trace, debug, info, warn, error

[server.transport]
type = "stdio"  # stdio, http, websocket
# For HTTP/WebSocket
http_port = 3000
websocket_path = "/ws"

[server.performance]
max_concurrent_requests = 1000
request_timeout_sec = 30
worker_threads = 4  # Auto-detect if not specified

# Authentication configuration
[auth]
# See Authentication System section

# Global mock behavior defaults
[defaults.behavior]
latency_ms = 0  # No artificial latency by default
latency_variance = 0.2  # 20% variance
error_rate = 0.0  # No errors by default
timeout_probability = 0.0

# Resources configuration
[[resources]]
uri = "config://server/info"
name = "Server Information"
description = "Mock server configuration and status"
mime_type = "application/json"

[resources.mock]
strategy = "static"
content = { name = "Metis", version = "1.0.0" }

# Import additional resource definitions
resources_dir = "config/resources/"

# Tools configuration
[[tools]]
name = "echo"
description = "Echo back the input"

[tools.input_schema]
type = "object"
properties = { message = { type = "string" } }

[tools.mock]
strategy = "script"
script = "fn execute(input) { return input; }"

# Import additional tool definitions
tools_dir = "config/tools/"

# Prompts configuration
[[prompts]]
name = "greeting"
description = "Generate a greeting message"

[prompts.mock]
strategy = "template"
template = "Hello, {{name}}!"

# Import additional prompt definitions
prompts_dir = "config/prompts/"

# Logging configuration
[logging]
level = "info"
format = "json"  # json, pretty, compact
output = "stdout"  # stdout, stderr, file
file_path = "logs/metis.log"

[logging.filters]
# Filter by target
targets = ["metis", "mcp"]
exclude_targets = ["hyper"]

# Metrics configuration
[metrics]
enabled = true
type = "prometheus"  # prometheus, opentelemetry, none
endpoint = "/metrics"
port = 9090

# Cache configuration
[cache]
enabled = true
backend = "memory"  # memory, redis
max_size_mb = 100
default_ttl_sec = 300

[cache.redis]
url = "redis://localhost:6379"
key_prefix = "metis:"

# Live reload configuration
[reload]
enabled = true
watch_directories = ["config/", "templates/", "scripts/"]
debounce_ms = 500  # Wait 500ms after last change

# Development mode
[development]
enabled = false
hot_reload = true
debug_endpoints = true
mock_delays = false  # Disable artificial delays in dev mode
```

### Modular Configuration

**Resource Definition** (`config/resources/users.toml`)

```toml
uri = "db://users"
name = "Users Database"
description = "Mock user database"
mime_type = "application/json"

[mock]
strategy = "random"

[mock.random]
type = "array"
min_length = 5
max_length = 20

[mock.random.item]
type = "object"
schema = {
    id = { type = "uuid" },
    username = { type = "fake", fake_type = "internet.username" },
    email = { type = "fake", fake_type = "internet.email" },
    full_name = { type = "fake", fake_type = "name.full_name" },
    created_at = { type = "timestamp", format = "rfc3339" },
    is_active = { type = "boolean", probability = 0.9 }
}

[behavior]
latency_ms = "range:50-200"
error_rate = 0.02
cache_duration_sec = 60
```

**Tool Definition** (`config/tools/search.toml`)

```toml
name = "search"
description = "Search for items"

[input_schema]
type = "object"
required = ["query"]
properties.query = { type = "string", description = "Search query" }
properties.limit = { type = "integer", minimum = 1, maximum = 100, default = 10 }
properties.filters = { type = "object" }

[mock]
strategy = "composite"

[mock.composite]
mode = "weighted"

[[mock.composite.strategies]]
strategy = "database"
weight = 0.3
[mock.composite.strategies.database]
driver = "postgres"
connection_string_env = "DATABASE_URL"
[mock.composite.strategies.database.query]
sql = "SELECT * FROM items WHERE title ILIKE $1 LIMIT $2"
parameters = [
    { from_input = "query", transform = "prepend:%,append:%" },
    { from_input = "limit" }
]

[[mock.composite.strategies]]
strategy = "template"
weight = 0.7
[mock.composite.strategies.template]
template_file = "templates/search_results.json"

[behavior]
execution_time_ms = "range:100-1000"
success_rate = 0.98
```

### Environment Variables

Support for environment variable substitution:

```toml
[auth.api_key]
valid_keys = ["${METIS_API_KEY_1}", "${METIS_API_KEY_2}"]

[mock.llm]
api_key = "${ANTHROPIC_API_KEY}"

[cache.redis]
url = "${REDIS_URL:-redis://localhost:6379}"  # With default
```

### Configuration Validation

**Validation Rules**
1. Schema validation for all configuration sections
2. Cross-reference validation (e.g., referenced templates exist)
3. Strategy-specific validation
4. Performance constraint checks
5. Security validation (no hardcoded secrets in non-secure contexts)

**Validation Errors**
```rust
pub enum ConfigError {
    ParseError(String),
    ValidationError { field: String, reason: String },
    MissingRequired(String),
    InvalidReference { reference: String, target: String },
    SchemaError(String),
}
```

### Live Reload Mechanism

**Implementation Strategy**
1. File system watcher using `notify` crate
2. Debouncing to avoid rapid reloads
3. Atomic configuration updates
4. Validation before applying
5. Rollback on error
6. WebSocket notifications to clients

**Reload Process**
```rust
async fn reload_configuration(&self) -> Result<(), ReloadError> {
    // 1. Load new configuration
    let new_config = ConfigLoader::load_from_file("metis.toml")?;

    // 2. Validate
    new_config.validate()?;

    // 3. Apply atomically
    let mut config = self.config.write().await;
    *config = new_config;

    // 4. Notify observers
    self.notify_reload().await?;

    Ok(())
}
```

---

## Performance Optimization

### 1. Concurrency & Parallelism

**Async Runtime**
- Tokio runtime with configurable worker threads
- Work-stealing scheduler
- Async I/O for all network operations

**Connection Pooling**
- Database connection pools (SQLx)
- HTTP client connection pooling
- LLM API connection reuse

### 2. Caching Strategy

**Multi-Level Cache**
```rust
pub struct CacheLayer {
    // L1: In-memory LRU cache
    memory_cache: Arc<Mutex<LruCache<String, CachedResponse>>>,

    // L2: Redis cache (optional)
    redis_cache: Option<RedisClient>,

    // Cache configuration
    config: CacheConfig,
}
```

**Cache Keys**
- Deterministic key generation
- Content-based hashing
- TTL per strategy
- Cache invalidation on config reload

**Cacheable Responses**
- Template-rendered content
- LLM-generated responses (expensive)
- Database query results
- File-based responses

### 3. Memory Management

**Memory Limits**
- Maximum response size limits
- Streaming for large responses
- Memory pool for frequently allocated objects
- Bounded queues for request processing

### 4. Response Optimization

**Serialization**
- Zero-copy serialization where possible
- Lazy evaluation of mock data
- Stream processing for large datasets

**Compression**
- Optional gzip/brotli compression
- Configurable compression levels
- Content-type aware compression

### 5. Benchmarking & Profiling

**Performance Targets**
- Throughput: >10,000 requests/second (simple mocks)
- Latency: p50 <5ms, p99 <50ms (excluding artificial delays)
- Memory: <100MB baseline, <1GB under load
- CPU: <50% on 4 cores at 10k req/s

**Profiling Tools**
- Flamegraph generation
- `perf` integration
- Memory profiling with `heaptrack`
- Continuous benchmarking in CI

---

## Development Phases

**Total Timeline**: 36 weeks (9 months) from start to v1.3 release
**Checkpoints**: After Phase 3 (Week 11) and Phase 6 (Week 18)

### Week 0: Pre-Planning & Technology Validation (Week 0)

**Goals**
- Validate critical technology assumptions before committing to development
- Reduce technical risk through targeted prototypes
- Establish architecture decision record (ADR) process
- Create realistic project plan with team buy-in

**Critical Validation Tasks**

**1. Rust MCP SDK Verification**
- [ ] Research official Rust MCP SDK status and maturity
- [ ] Review SDK documentation and examples
- [ ] Test basic MCP protocol operations (initialize, resources, tools, prompts)
- [ ] **Decision Point**: Use official SDK OR build protocol handler from spec
- [ ] **Timeline**: 2 days

**2. Multi-Language Scripting Spike**
- [ ] Prototype Rhai script execution with sandboxing
  - Test timeout enforcement
  - Test memory limits
  - Measure performance overhead
- [ ] Prototype Python (pyo3) integration
  - Test GIL handling
  - Test sandboxing capabilities
  - Measure FFI overhead
- [ ] Prototype Lua (mlua) integration
  - Test ease of use
  - Compare performance vs Rhai and Python
- [ ] **Decision Point**: Languages to support in v1.0 (recommend: Rhai only)
- [ ] **Timeline**: 3 days

**3. Web UI Technology Validation**
- [ ] Create minimal Leptos application
- [ ] Integrate Monaco editor in WASM
- [ ] Test WASM bundle size and load times
- [ ] Verify hot reload functionality
- [ ] Test WebSocket connection for real-time updates
- [ ] **Decision Point**: Leptos vs simpler alternatives (htmx, Yew)
- [ ] **Timeline**: 3 days

**4. Performance Baseline Establishment**
- [ ] Create simple benchmark harness using criterion
- [ ] Benchmark basic request/response cycle
- [ ] Establish target: >10k req/s for simple mock strategies
- [ ] Identify potential bottlenecks early
- [ ] **Timeline**: 1 day

**5. Architecture Decision Records Setup**
- [ ] Create `docs/adr/` directory structure
- [ ] Write ADR template
- [ ] Document initial architecture decisions:
  - ADR-001: Hexagonal Architecture
  - ADR-002: SOLID Principles Application
  - ADR-003: Mock Strategy Pattern
  - ADR-004: Multi-Language Scripting Approach
  - ADR-005: Testing Strategy
  - ADR-006: Rust MCP SDK vs Custom Implementation
- [ ] **Timeline**: 2 days

**Deliverables**
- [ ] Technology validation report with go/no-go recommendations
- [ ] Prototype code for validated technologies
- [ ] Initial ADRs documenting architecture decisions
- [ ] Performance baseline measurements
- [ ] Revised project plan with realistic timeline
- [ ] Risk register with mitigation strategies

**Success Criteria**
- All critical technologies validated or alternatives identified
- Team alignment on technology choices
- Realistic timeline established (30-36 weeks)
- Clear definition of v1.0 MVP scope

**Risk Mitigation**
- If Rust MCP SDK insufficient: Plan to build from spec (add 2-3 weeks)
- If multi-language scripting too complex: Start with Rhai only
- If Leptos problematic: Fall back to simpler UI technology
- If performance targets unmet: Identify optimization strategies early

---

### Phase 1: Core Foundation & Testing Infrastructure (Weeks 1-3)

**Goals**
- Project setup with testing from day one
- MCP protocol implementation with full test coverage
- Basic server infrastructure
- Configuration loading with validation tests
- Establish CI/CD pipeline early

**Test-Driven Development Approach**
- Write tests before implementation
- Minimum 80% code coverage from start
- Integration tests for all public APIs
- Property-based testing for core logic

**Deliverables**
- [ ] Rust project with proper structure and workspace organization
- [ ] CI/CD pipeline (GitHub Actions) with automated testing
- [ ] Test infrastructure and utilities
  - [ ] Test fixtures and mocks
  - [ ] Property-based test framework setup
  - [ ] Integration test harness
- [ ] Integration with official Rust MCP SDK
  - [ ] Unit tests for SDK integration layer
  - [ ] Mock MCP client for testing
- [ ] Basic MCP server (stdio transport)
  - [ ] Unit tests for transport layer
  - [ ] Integration tests for message handling
- [ ] Configuration file parsing (TOML/YAML/JSON)
  - [ ] Unit tests for each format parser
  - [ ] Property tests for config validation
  - [ ] Tests for malformed config handling
- [ ] Basic resource handler (static responses)
  - [ ] Unit tests for resource registration
  - [ ] Integration tests for resource/list and resource/read
  - [ ] Tests for error conditions
- [ ] Basic tool handler (echo tool)
  - [ ] Unit tests for tool registration
  - [ ] Integration tests for tools/call
  - [ ] Tests for invalid tool calls
- [ ] Basic prompt handler (static prompts)
  - [ ] Unit tests for prompt formatting
  - [ ] Integration tests for prompts/get
- [ ] Logging infrastructure with structured logging
  - [ ] Tests for log output validation
- [ ] Error handling framework
  - [ ] Tests for all error types and conversions

**Testing Standards**
```rust
// Test organization
#[cfg(test)]
mod tests {
    use super::*;
    use test_utils::{MockMcpClient, TestFixtures};

    // Unit tests - test individual functions/methods
    mod unit {
        #[test]
        fn test_config_validation() {
            let config = ServerConfig {
                // ...
            };
            assert!(config.validate().is_ok());
        }

        #[test]
        fn test_invalid_config_fails() {
            let config = ServerConfig {
                port: 0, // Invalid
            };
            assert!(config.validate().is_err());
        }
    }

    // Integration tests - test component interactions
    mod integration {
        #[tokio::test]
        async fn test_server_initialization() {
            let config = TestFixtures::minimal_config();
            let server = MetisServer::new(config).await;
            assert!(server.is_ok());
        }

        #[tokio::test]
        async fn test_echo_tool_execution() {
            let server = setup_test_server().await;
            let result = server.call_tool("echo", json!({
                "message": "test"
            })).await.unwrap();

            assert_eq!(result.content[0].text, "test");
        }
    }

    // Property-based tests
    mod property {
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn config_roundtrip(port in 1024u16..65535) {
                let config = ServerConfig { port, ..default() };
                let serialized = toml::to_string(&config).unwrap();
                let deserialized: ServerConfig = toml::from_str(&serialized).unwrap();
                assert_eq!(config.port, deserialized.port);
            }
        }
    }
}
```

**Technical Implementation with Tests**
```rust
// Core types with comprehensive test coverage
pub struct MetisServer {
    config: Arc<ServerConfig>,
    mcp_server: McpServer,
    runtime: Runtime,
}

impl MetisServer {
    pub async fn new(config: ServerConfig) -> Result<Self, ServerError> {
        config.validate()?;
        // Implementation
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_server_config_validation() {
        // Test valid config
        let config = ServerConfig::default();
        assert!(config.validate().is_ok());

        // Test invalid port
        let mut config = ServerConfig::default();
        config.port = 0;
        assert!(config.validate().is_err());
    }

    #[tokio::test]
    async fn test_server_starts_and_stops() {
        let config = TestFixtures::minimal_config();
        let server = MetisServer::new(config).await.unwrap();

        let handle = tokio::spawn(async move {
            server.run().await
        });

        // Wait a bit
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Server should be running
        assert!(!handle.is_finished());

        // Stop server
        handle.abort();
    }
}
```

**CI/CD Pipeline** (`.github/workflows/ci.yml`)
```yaml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          components: clippy, rustfmt

      - name: Check formatting
        run: cargo fmt -- --check

      - name: Clippy
        run: cargo clippy -- -D warnings

      - name: Run tests
        run: cargo test --all-features

      - name: Generate coverage
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --out Xml --output-dir coverage

      - name: Upload coverage
        uses: codecov/codecov-action@v3
        with:
          files: ./coverage/cobertura.xml
          fail_ci_if_error: true

  integration:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Run integration tests
        run: cargo test --test '*' --all-features
```

### Phase 2: Mock Strategies with Comprehensive Testing (Weeks 4-6)

**Goals**
- Implement all mock data generation strategies with TDD
- Strategy configuration system with validation tests
- Response behavior controls with integration tests
- External file loading (JSON/YAML/TOML) with format tests

**Test-Driven Development Focus**
- Write tests for each strategy before implementation
- Test strategy composition and fallback chains
- Test edge cases and error conditions
- Property-based tests for data generation consistency

**Deliverables**
- [ ] Random generation strategy
  - [ ] Unit tests for each random type (uuid, integer, string, etc.)
  - [ ] Property tests: generated data always matches schema
  - [ ] Tests for distribution (uniform, weighted)
  - [ ] Benchmark tests for generation speed
- [ ] Template-based generation (Tera)
  - [ ] Unit tests for template parsing
  - [ ] Integration tests with variable substitution
  - [ ] Tests for template errors and fallbacks
  - [ ] Tests for template inheritance
- [ ] Script-based generation (multi-language)
  - [ ] Unit tests for each script engine (Rhai, Python, Lua, Ruby, JS)
  - [ ] Sandbox security tests
  - [ ] Tests for script timeout and memory limits
  - [ ] Tests for script caching
  - [ ] Integration tests with context passing
- [ ] Pattern-based generation (Regex)
  - [ ] Unit tests for pattern matching
  - [ ] Property tests: generated strings match regex
  - [ ] Tests for complex patterns
  - [ ] Performance tests for pattern generation
- [ ] File-based responses
  - [ ] Unit tests for file loading (JSON, YAML, TOML)
  - [ ] Tests for file selection strategies (random, sequential, weighted)
  - [ ] Tests for missing file handling
  - [ ] Tests for file watching and reload
- [ ] Static responses
  - [ ] Unit tests for static content loading
  - [ ] Tests for content validation
- [ ] Behavior controls (latency, errors)
  - [ ] Unit tests for latency injection
  - [ ] Tests for error rate simulation
  - [ ] Integration tests for timeout behavior
  - [ ] Tests for cache duration
- [ ] Strategy factory pattern
  - [ ] Unit tests for strategy registration
  - [ ] Tests for strategy selection
  - [ ] Tests for unknown strategy handling
- [ ] Composite/hybrid strategies
  - [ ] Unit tests for fallback chains
  - [ ] Integration tests for weighted selection
  - [ ] Tests for conditional strategies
  - [ ] Tests for strategy timeout and retry

**Test Suite Structure**
```rust
#[cfg(test)]
mod strategy_tests {
    use super::*;

    mod random_strategy {
        #[test]
        fn test_uuid_generation() {
            let strategy = RandomStrategy::new(RandomConfig {
                field_type: FieldType::Uuid,
            });

            let value = strategy.generate_sync();
            assert!(uuid::Uuid::parse_str(value.as_str()).is_ok());
        }

        #[test]
        fn test_integer_range() {
            let strategy = RandomStrategy::new(RandomConfig {
                field_type: FieldType::Integer { min: 10, max: 20 },
            });

            for _ in 0..100 {
                let value = strategy.generate_sync().as_i64().unwrap();
                assert!(value >= 10 && value <= 20);
            }
        }

        // Property-based test
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn random_integer_always_in_range(min in 0i64..1000, max in 1001i64..10000) {
                let strategy = RandomStrategy::new(RandomConfig {
                    field_type: FieldType::Integer { min, max },
                });

                let value = strategy.generate_sync().as_i64().unwrap();
                prop_assert!(value >= min && value <= max);
            }
        }
    }

    mod template_strategy {
        #[tokio::test]
        async fn test_variable_substitution() {
            let template = "Hello, {{name}}!";
            let strategy = TemplateStrategy::new(template);

            let context = json!({ "name": "World" });
            let result = strategy.generate(&context).await.unwrap();

            assert_eq!(result.as_str().unwrap(), "Hello, World!");
        }

        #[tokio::test]
        async fn test_template_error_handling() {
            let template = "Hello, {{undefined_var}}!";
            let strategy = TemplateStrategy::new(template);

            let context = json!({});
            let result = strategy.generate(&context).await;

            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), StrategyError::TemplateError(_)));
        }

        #[tokio::test]
        async fn test_template_with_loops() {
            let template = "{% for i in items %}{{i}}{% endfor %}";
            let strategy = TemplateStrategy::new(template);

            let context = json!({ "items": [1, 2, 3] });
            let result = strategy.generate(&context).await.unwrap();

            assert_eq!(result.as_str().unwrap(), "123");
        }
    }

    mod script_strategy {
        #[tokio::test]
        async fn test_python_script_execution() {
            let script = r#"
def generate(ctx):
    return {"result": ctx["value"] * 2}
"#;
            let strategy = ScriptStrategy::new(ScriptLanguage::Python, script);

            let context = json!({ "value": 21 });
            let result = strategy.generate(&context).await.unwrap();

            assert_eq!(result["result"], 42);
        }

        #[tokio::test]
        async fn test_script_timeout() {
            let script = r#"
def generate(ctx):
    import time
    time.sleep(10)  # Should timeout
    return {}
"#;
            let strategy = ScriptStrategy::new_with_timeout(
                ScriptLanguage::Python,
                script,
                Duration::from_millis(100)
            );

            let context = json!({});
            let result = strategy.generate(&context).await;

            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), StrategyError::Timeout));
        }

        #[tokio::test]
        async fn test_script_sandbox_restrictions() {
            let script = r#"
def generate(ctx):
    import os
    os.system("rm -rf /")  # Should be blocked
    return {}
"#;
            let strategy = ScriptStrategy::new(ScriptLanguage::Python, script);

            let context = json!({});
            let result = strategy.generate(&context).await;

            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), StrategyError::SecurityViolation));
        }

        #[tokio::test]
        async fn test_multi_language_consistency() {
            let python_script = "def generate(ctx): return {'value': 42}";
            let lua_script = "function generate(ctx) return {value = 42} end";
            let rhai_script = "fn generate(ctx) { #{value: 42} }";

            let py_strategy = ScriptStrategy::new(ScriptLanguage::Python, python_script);
            let lua_strategy = ScriptStrategy::new(ScriptLanguage::Lua, lua_script);
            let rhai_strategy = ScriptStrategy::new(ScriptLanguage::Rhai, rhai_script);

            let context = json!({});

            let py_result = py_strategy.generate(&context).await.unwrap();
            let lua_result = lua_strategy.generate(&context).await.unwrap();
            let rhai_result = rhai_strategy.generate(&context).await.unwrap();

            assert_eq!(py_result["value"], 42);
            assert_eq!(lua_result["value"], 42);
            assert_eq!(rhai_result["value"], 42);
        }
    }

    mod file_strategy {
        #[tokio::test]
        async fn test_json_file_loading() {
            let temp_file = create_temp_json_file(json!({"test": "data"}));
            let strategy = FileStrategy::new(temp_file.path());

            let result = strategy.generate(&RequestContext::default()).await.unwrap();

            assert_eq!(result["test"], "data");
        }

        #[tokio::test]
        async fn test_missing_file_error() {
            let strategy = FileStrategy::new("/nonexistent/file.json");

            let result = strategy.generate(&RequestContext::default()).await;

            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), StrategyError::FileNotFound(_)));
        }

        #[tokio::test]
        async fn test_file_format_detection() {
            // Test JSON, YAML, TOML detection
            let json_file = create_temp_file("test.json", r#"{"key": "value"}"#);
            let yaml_file = create_temp_file("test.yaml", "key: value");
            let toml_file = create_temp_file("test.toml", "key = \"value\"");

            let json_strategy = FileStrategy::new(json_file.path());
            let yaml_strategy = FileStrategy::new(yaml_file.path());
            let toml_strategy = FileStrategy::new(toml_file.path());

            let json_result = json_strategy.generate(&RequestContext::default()).await.unwrap();
            let yaml_result = yaml_strategy.generate(&RequestContext::default()).await.unwrap();
            let toml_result = toml_strategy.generate(&RequestContext::default()).await.unwrap();

            assert_eq!(json_result["key"], "value");
            assert_eq!(yaml_result["key"], "value");
            assert_eq!(toml_result["key"], "value");
        }

        #[tokio::test]
        async fn test_file_selection_random() {
            let dir = create_temp_dir_with_files(&[
                ("file1.json", r#"{"id": 1}"#),
                ("file2.json", r#"{"id": 2}"#),
                ("file3.json", r#"{"id": 3}"#),
            ]);

            let strategy = FileStrategy::new_with_selection(
                dir.path().join("*.json"),
                SelectionMode::Random
            );

            // Generate multiple times, should see different files
            let mut seen_ids = std::collections::HashSet::new();
            for _ in 0..20 {
                let result = strategy.generate(&RequestContext::default()).await.unwrap();
                seen_ids.insert(result["id"].as_i64().unwrap());
            }

            assert!(seen_ids.len() > 1); // Should see multiple different files
        }
    }

    mod composite_strategy {
        #[tokio::test]
        async fn test_fallback_chain() {
            let strategies = vec![
                // First strategy always fails
                Box::new(FailingStrategy) as Box<dyn MockStrategy>,
                // Second strategy succeeds
                Box::new(StaticStrategy::new(json!({"result": "success"}))),
            ];

            let composite = CompositeStrategy::new(CompositeMode::Fallback, strategies);

            let result = composite.generate(&RequestContext::default()).await.unwrap();

            assert_eq!(result["result"], "success");
        }

        #[tokio::test]
        async fn test_weighted_selection() {
            let strategies = vec![
                (Box::new(StaticStrategy::new(json!({"type": "A"}))) as Box<dyn MockStrategy>, 0.7),
                (Box::new(StaticStrategy::new(json!({"type": "B"}))), 0.3),
            ];

            let composite = CompositeStrategy::new(CompositeMode::Weighted, strategies);

            // Generate many times and check distribution
            let mut type_a_count = 0;
            let mut type_b_count = 0;

            for _ in 0..1000 {
                let result = composite.generate(&RequestContext::default()).await.unwrap();
                if result["type"] == "A" {
                    type_a_count += 1;
                } else {
                    type_b_count += 1;
                }
            }

            // Should be roughly 70/30 split (with some variance)
            let ratio = type_a_count as f64 / 1000.0;
            assert!(ratio > 0.65 && ratio < 0.75);
        }

        #[tokio::test]
        async fn test_conditional_strategy() {
            let strategies = vec![
                (
                    Box::new(StaticStrategy::new(json!({"response": "morning"}))) as Box<dyn MockStrategy>,
                    Box::new(|ctx: &RequestContext| ctx.get("time") == "am") as Box<dyn Fn(&RequestContext) -> bool>,
                ),
                (
                    Box::new(StaticStrategy::new(json!({"response": "evening"}))),
                    Box::new(|ctx: &RequestContext| ctx.get("time") == "pm"),
                ),
            ];

            let composite = CompositeStrategy::new(CompositeMode::Conditional, strategies);

            let mut am_context = RequestContext::default();
            am_context.insert("time", "am");
            let am_result = composite.generate(&am_context).await.unwrap();
            assert_eq!(am_result["response"], "morning");

            let mut pm_context = RequestContext::default();
            pm_context.insert("time", "pm");
            let pm_result = composite.generate(&pm_context).await.unwrap();
            assert_eq!(pm_result["response"], "evening");
        }
    }

    mod behavior_tests {
        #[tokio::test]
        async fn test_latency_injection() {
            let strategy = StaticStrategy::new(json!({"test": "data"}))
                .with_latency(Duration::from_millis(100));

            let start = Instant::now();
            strategy.generate(&RequestContext::default()).await.unwrap();
            let duration = start.elapsed();

            assert!(duration >= Duration::from_millis(95)); // Allow small variance
            assert!(duration < Duration::from_millis(150));
        }

        #[tokio::test]
        async fn test_error_rate_simulation() {
            let strategy = StaticStrategy::new(json!({"test": "data"}))
                .with_error_rate(0.5); // 50% error rate

            let mut success_count = 0;
            let mut error_count = 0;

            for _ in 0..100 {
                match strategy.generate(&RequestContext::default()).await {
                    Ok(_) => success_count += 1,
                    Err(_) => error_count += 1,
                }
            }

            // Should be roughly 50/50 split
            assert!(success_count > 30 && success_count < 70);
            assert!(error_count > 30 && error_count < 70);
        }

        #[tokio::test]
        async fn test_timeout_behavior() {
            let strategy = SlowStrategy::new(Duration::from_secs(10))
                .with_timeout(Duration::from_millis(100));

            let result = strategy.generate(&RequestContext::default()).await;

            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), StrategyError::Timeout));
        }
    }
}
```

**Integration Tests** (`tests/strategy_integration.rs`)
```rust
#[tokio::test]
async fn test_end_to_end_strategy_execution() {
    // Setup server with various strategies
    let config = TestConfig::with_strategies(vec![
        ("random", RandomStrategyConfig::default()),
        ("template", TemplateStrategyConfig::default()),
        ("script", ScriptStrategyConfig::default()),
    ]);

    let server = MetisServer::new(config).await.unwrap();

    // Test each strategy through MCP protocol
    let client = TestMcpClient::connect(&server).await.unwrap();

    // Call tool with random strategy
    let result = client.call_tool("test_random", json!({})).await.unwrap();
    assert!(result.content.is_some());

    // Call tool with template strategy
    let result = client.call_tool("test_template", json!({
        "name": "Test"
    })).await.unwrap();
    assert!(result.content[0].text.contains("Test"));

    // Call tool with script strategy
    let result = client.call_tool("test_script", json!({
        "value": 10
    })).await.unwrap();
    assert_eq!(result.content[0].parsed_json["result"], 20);
}
```

**Benchmark Tests** (`benches/strategy_benchmarks.rs`)
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

fn benchmark_strategies(c: &mut Criterion) {
    let mut group = c.benchmark_group("strategies");

    // Benchmark random strategy
    group.bench_function("random_uuid", |b| {
        let strategy = RandomStrategy::new(RandomConfig {
            field_type: FieldType::Uuid,
        });

        b.iter(|| {
            strategy.generate_sync()
        });
    });

    // Benchmark template strategy
    group.bench_function("template_simple", |b| {
        let strategy = TemplateStrategy::new("Hello, {{name}}!");
        let context = json!({"name": "World"});

        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(strategy.generate(&context))
        });
    });

    // Benchmark different script languages
    for lang in &[ScriptLanguage::Python, ScriptLanguage::Lua, ScriptLanguage::Rhai] {
        group.bench_with_input(
            BenchmarkId::new("script", format!("{:?}", lang)),
            lang,
            |b, lang| {
                let script = get_benchmark_script(*lang);
                let strategy = ScriptStrategy::new(*lang, script);
                let context = json!({});

                b.iter(|| {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(strategy.generate(&context))
                });
            }
        );
    }

    group.finish();
}

criterion_group!(benches, benchmark_strategies);
criterion_main!(benches);
```

### Phase 3: Advanced Features with Testing (Weeks 7-9)

**Goals**
- LLM integration with cost tracking tests
- Database support with transaction tests
- Composite strategies with integration tests
- Multi-level caching with performance tests
- File access tools with security tests
- RAG tools with vector search tests

**Test Coverage Requirements**
- Unit tests for all LLM providers
- Integration tests for database operations
- Security tests for file access controls
- Performance tests for RAG vector operations
- Cost simulation tests for LLM usage

**Deliverables**
- [ ] LLM strategy (OpenAI, Anthropic, local models)
  - [ ] Unit tests for each provider
  - [ ] Integration tests with real/mock APIs
  - [ ] Tests for streaming responses
  - [ ] Cost tracking and limit tests
  - [ ] Fallback provider tests
  - [ ] Response caching tests
- [ ] Database strategy (PostgreSQL, MySQL, SQLite)
  - [ ] Unit tests for query building
  - [ ] Integration tests with real databases (using testcontainers)
  - [ ] Transaction rollback tests
  - [ ] Connection pool tests
  - [ ] Query timeout tests
  - [ ] SQL injection prevention tests
- [ ] Composite/hybrid strategies
  - [ ] Already covered in Phase 2
- [ ] Multi-level caching
  - [ ] Unit tests for cache key generation
  - [ ] Integration tests for cache invalidation
  - [ ] Tests for cache coherence (memory + Redis)
  - [ ] Performance tests for cache hit rates
  - [ ] Tests for cache size limits
- [ ] File access tools
  - [ ] Unit tests for path validation
  - [ ] Security tests for directory traversal prevention
  - [ ] Tests for file type filtering
  - [ ] Integration tests for file watching
  - [ ] Tests for concurrent file access
- [ ] RAG (Retrieval-Augmented Generation) tools
  - [ ] Unit tests for document chunking
  - [ ] Integration tests with vector databases
  - [ ] Tests for embedding generation
  - [ ] Semantic search accuracy tests
  - [ ] Performance tests for vector similarity
  - [ ] Tests for context window management
- [ ] Connection pooling
  - [ ] Tests for pool exhaustion
  - [ ] Tests for connection timeouts
  - [ ] Tests for connection reuse
- [ ] Cost control for LLM calls
  - [ ] Unit tests for cost calculation
  - [ ] Integration tests for cost limits
  - [ ] Tests for cost tracking across sessions
- [ ] Performance optimization
  - [ ] Benchmark tests for all strategies
  - [ ] Memory profiling tests
  - [ ] Latency tests
- [ ] Load testing framework
  - [ ] Tests with 100, 1000, 10000 concurrent requests
  - [ ] Sustained load tests
  - [ ] Spike tests

**LLM Integration Tests**
```rust
#[cfg(test)]
mod llm_tests {
    use super::*;

    mod unit {
        #[tokio::test]
        async fn test_llm_config_validation() {
            let config = LlmConfig {
                provider: "openai",
                model: "gpt-4",
                api_key: "test-key",
                max_tokens: 1000,
                temperature: 0.7,
            };

            assert!(config.validate().is_ok());
        }

        #[test]
        fn test_cost_calculation() {
            let usage = TokenUsage {
                input_tokens: 100,
                output_tokens: 200,
            };

            let cost = calculate_cost("gpt-4", &usage);

            // GPT-4: $0.03/1K input, $0.06/1K output
            let expected = (100.0 * 0.03 / 1000.0) + (200.0 * 0.06 / 1000.0);
            assert!((cost - expected).abs() < 0.001);
        }

        #[tokio::test]
        async fn test_cost_limit_enforcement() {
            let mut strategy = LlmStrategy::new(LlmConfig::default())
                .with_cost_limit(0.01); // $0.01 limit

            // First call should succeed
            let result1 = strategy.generate(&RequestContext::default()).await;
            assert!(result1.is_ok());

            // Subsequent calls should fail when limit reached
            for _ in 0..100 {
                let result = strategy.generate(&RequestContext::default()).await;
                if result.is_err() {
                    assert!(matches!(result.unwrap_err(), StrategyError::CostLimitExceeded));
                    return;
                }
            }

            panic!("Cost limit should have been exceeded");
        }
    }

    mod integration {
        #[tokio::test]
        async fn test_openai_integration() {
            let api_key = env::var("OPENAI_API_KEY").ok();
            if api_key.is_none() {
                return; // Skip if no API key
            }

            let strategy = LlmStrategy::new(LlmConfig {
                provider: "openai",
                model: "gpt-3.5-turbo",
                api_key: api_key.unwrap(),
                temperature: 0.7,
                max_tokens: 100,
            });

            let context = json!({
                "prompt": "Say hello in one word"
            });

            let result = strategy.generate(&RequestContext::with_data(context)).await.unwrap();

            assert!(!result.as_str().unwrap().is_empty());
        }

        #[tokio::test]
        async fn test_llm_response_caching() {
            let strategy = LlmStrategy::new(LlmConfig::default())
                .with_cache(CacheConfig {
                    enabled: true,
                    ttl: Duration::from_secs(60),
                });

            let context = json!({"prompt": "test"});

            // First call - cache miss
            let start1 = Instant::now();
            let result1 = strategy.generate(&RequestContext::with_data(context.clone())).await.unwrap();
            let duration1 = start1.elapsed();

            // Second call - cache hit (should be much faster)
            let start2 = Instant::now();
            let result2 = strategy.generate(&RequestContext::with_data(context)).await.unwrap();
            let duration2 = start2.elapsed();

            assert_eq!(result1, result2);
            assert!(duration2 < duration1 / 10); // Cache should be 10x+ faster
        }

        #[tokio::test]
        async fn test_llm_streaming() {
            let strategy = LlmStrategy::new(LlmConfig::default())
                .with_streaming(true);

            let context = json!({"prompt": "Count from 1 to 10"});

            let mut stream = strategy.generate_stream(&RequestContext::with_data(context)).await.unwrap();

            let mut chunks = Vec::new();
            while let Some(chunk) = stream.next().await {
                chunks.push(chunk.unwrap());
            }

            assert!(chunks.len() > 1); // Should have multiple chunks
        }

        #[tokio::test]
        async fn test_llm_provider_fallback() {
            let strategy = LlmStrategy::new_with_fallback(vec![
                (LlmProvider::OpenAI, LlmConfig { /* failing config */ }),
                (LlmProvider::Anthropic, LlmConfig { /* working config */ }),
            ]);

            let context = json!({"prompt": "test"});

            // Should fallback to Anthropic when OpenAI fails
            let result = strategy.generate(&RequestContext::with_data(context)).await;

            assert!(result.is_ok());
        }
    }
}
```

**Database Integration Tests**
```rust
#[cfg(test)]
mod database_tests {
    use super::*;
    use testcontainers::*;

    mod unit {
        #[test]
        fn test_query_parameterization() {
            let query = "SELECT * FROM users WHERE id = $1 AND status = $2";
            let params = vec![Value::from(123), Value::from("active")];

            let sanitized = sanitize_query(query, &params);

            // Should not allow SQL injection
            assert!(!sanitized.contains("'; DROP TABLE"));
        }

        #[test]
        fn test_connection_pool_config() {
            let config = PoolConfig {
                max_connections: 10,
                min_connections: 2,
                connection_timeout: Duration::from_secs(5),
            };

            assert!(config.validate().is_ok());

            let invalid = PoolConfig {
                max_connections: 1,
                min_connections: 10, // Invalid: min > max
                connection_timeout: Duration::from_secs(5),
            };

            assert!(invalid.validate().is_err());
        }
    }

    mod integration {
        #[tokio::test]
        async fn test_postgres_query_execution() {
            let postgres = testcontainers::postgres();
            let connection_string = postgres.connection_string();

            let strategy = DatabaseStrategy::new(DatabaseConfig {
                driver: "postgres",
                connection_string,
                query: "SELECT * FROM users WHERE status = $1",
                parameters: vec![Value::from("active")],
            }).await.unwrap();

            // Setup test data
            setup_test_table(&strategy).await.unwrap();

            let result = strategy.generate(&RequestContext::default()).await.unwrap();

            assert!(result.is_array());
        }

        #[tokio::test]
        async fn test_database_transaction_rollback() {
            let postgres = testcontainers::postgres();
            let connection_string = postgres.connection_string();

            let strategy = DatabaseStrategy::new(DatabaseConfig {
                driver: "postgres",
                connection_string,
                use_transactions: true,
            }).await.unwrap();

            // Start transaction
            let tx = strategy.begin_transaction().await.unwrap();

            // Insert data
            tx.execute("INSERT INTO users (name) VALUES ('test')").await.unwrap();

            // Rollback
            tx.rollback().await.unwrap();

            // Verify data was rolled back
            let count = strategy.execute_query("SELECT COUNT(*) FROM users").await.unwrap();
            assert_eq!(count[0]["count"], 0);
        }

        #[tokio::test]
        async fn test_connection_pool_exhaustion() {
            let postgres = testcontainers::postgres();

            let strategy = DatabaseStrategy::new(DatabaseConfig {
                driver: "postgres",
                connection_string: postgres.connection_string(),
                pool: PoolConfig {
                    max_connections: 2,
                    ..Default::default()
                },
            }).await.unwrap();

            // Acquire all connections
            let conn1 = strategy.get_connection().await.unwrap();
            let conn2 = strategy.get_connection().await.unwrap();

            // Third connection should timeout
            let result = tokio::time::timeout(
                Duration::from_millis(100),
                strategy.get_connection()
            ).await;

            assert!(result.is_err()); // Timeout
        }

        #[tokio::test]
        async fn test_sql_injection_prevention() {
            let postgres = testcontainers::postgres();

            let strategy = DatabaseStrategy::new(DatabaseConfig {
                driver: "postgres",
                connection_string: postgres.connection_string(),
                query: "SELECT * FROM users WHERE id = $1",
            }).await.unwrap();

            // Try SQL injection
            let malicious_context = json!({
                "id": "1; DROP TABLE users; --"
            });

            let result = strategy.generate(&RequestContext::with_data(malicious_context)).await;

            // Should fail safely, not execute DROP TABLE
            // Check table still exists
            let table_exists = strategy.execute_query(
                "SELECT EXISTS (SELECT FROM pg_tables WHERE tablename = 'users')"
            ).await.unwrap();

            assert_eq!(table_exists[0]["exists"], true);
        }
    }
}
```

**File Access & RAG Tools Tests**
```rust
#[cfg(test)]
mod file_access_tests {
    use super::*;

    mod security {
        #[tokio::test]
        async fn test_directory_traversal_prevention() {
            let file_access = FileAccessTool::new(FileAccessConfig {
                allowed_dirs: vec!["/tmp/test".into()],
                allowed_extensions: vec!["txt", "json"],
            });

            // Try to access parent directory
            let result = file_access.read_file("../../etc/passwd").await;

            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), FileAccessError::Unauthorized));
        }

        #[tokio::test]
        async fn test_file_type_restriction() {
            let file_access = FileAccessTool::new(FileAccessConfig {
                allowed_dirs: vec!["/tmp/test".into()],
                allowed_extensions: vec!["txt"],
            });

            // Try to access non-allowed extension
            let result = file_access.read_file("/tmp/test/file.exe").await;

            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), FileAccessError::ForbiddenType));
        }

        #[test]
        fn test_path_normalization() {
            let normalized = normalize_path("/tmp/test/../../../etc/passwd");

            // Should prevent traversal
            assert!(!normalized.starts_with("/etc"));
        }
    }

    mod integration {
        #[tokio::test]
        async fn test_file_reading() {
            let temp_dir = create_temp_dir();
            let test_file = temp_dir.join("test.txt");
            std::fs::write(&test_file, "test content").unwrap();

            let file_access = FileAccessTool::new(FileAccessConfig {
                allowed_dirs: vec![temp_dir.clone()],
                allowed_extensions: vec!["txt"],
            });

            let content = file_access.read_file(test_file.to_str().unwrap()).await.unwrap();

            assert_eq!(content, "test content");
        }

        #[tokio::test]
        async fn test_file_watching() {
            let temp_dir = create_temp_dir();
            let test_file = temp_dir.join("watch.txt");

            let file_watcher = FileWatcher::new(FileWatchConfig {
                paths: vec![test_file.clone()],
                debounce_ms: 100,
            });

            let mut events = file_watcher.watch().await.unwrap();

            // Modify file
            std::fs::write(&test_file, "new content").unwrap();

            // Should receive event
            let event = tokio::time::timeout(
                Duration::from_secs(1),
                events.recv()
            ).await.unwrap().unwrap();

            assert_eq!(event.path, test_file);
            assert_eq!(event.kind, EventKind::Modified);
        }
    }
}

#[cfg(test)]
mod rag_tests {
    use super::*;

    mod unit {
        #[test]
        fn test_document_chunking() {
            let text = "A".repeat(10000);

            let chunks = chunk_document(&text, ChunkConfig {
                max_chunk_size: 1000,
                overlap: 100,
            });

            assert!(chunks.len() > 5);
            assert!(chunks.iter().all(|c| c.len() <= 1100)); // max + overlap
        }

        #[test]
        fn test_embedding_cache_key() {
            let text1 = "test document";
            let text2 = "test document";
            let text3 = "different document";

            let key1 = generate_cache_key(text1);
            let key2 = generate_cache_key(text2);
            let key3 = generate_cache_key(text3);

            assert_eq!(key1, key2);
            assert_ne!(key1, key3);
        }
    }

    mod integration {
        #[tokio::test]
        async fn test_vector_store_insertion_and_search() {
            let vector_store = VectorStore::new(VectorStoreConfig {
                backend: "memory", // or "chromadb", "pinecone", etc.
                embedding_model: "openai/text-embedding-3-small",
            }).await.unwrap();

            // Insert documents
            let docs = vec![
                "The quick brown fox jumps over the lazy dog",
                "A journey of a thousand miles begins with a single step",
                "To be or not to be, that is the question",
            ];

            for (i, doc) in docs.iter().enumerate() {
                vector_store.insert(i.to_string(), doc).await.unwrap();
            }

            // Search
            let results = vector_store.search("animal", 2).await.unwrap();

            // Should find the fox document first
            assert_eq!(results[0].id, "0");
            assert!(results[0].score > 0.5);
        }

        #[tokio::test]
        async fn test_rag_context_retrieval() {
            let rag = RagTool::new(RagConfig {
                vector_store: VectorStoreConfig::default(),
                top_k: 3,
                similarity_threshold: 0.7,
            }).await.unwrap();

            // Index documents
            rag.index_documents(vec![
                Document { id: "1", content: "Python is a programming language" },
                Document { id: "2", content: "JavaScript is used for web development" },
                Document { id: "3", content: "Rust is a systems programming language" },
            ]).await.unwrap();

            // Retrieve context for query
            let context = rag.retrieve_context("What is Rust?").await.unwrap();

            assert!(context.documents.len() > 0);
            assert_eq!(context.documents[0].id, "3"); // Rust document should be first
        }

        #[tokio::test]
        async fn test_rag_with_metadata_filtering() {
            let rag = RagTool::new(RagConfig::default()).await.unwrap();

            rag.index_with_metadata(vec![
                (Document { id: "1", content: "..."}, Metadata { category: "tech" }),
                (Document { id: "2", content: "..."}, Metadata { category: "science" }),
            ]).await.unwrap();

            // Search with filter
            let results = rag.search_with_filter(
                "test query",
                Filter::new().category("tech")
            ).await.unwrap();

            assert!(results.iter().all(|r| r.metadata.category == "tech"));
        }

        #[tokio::test]
        async fn test_embedding_generation_caching() {
            let rag = RagTool::new(RagConfig {
                cache_embeddings: true,
                ..Default::default()
            }).await.unwrap();

            let text = "test document for embedding";

            // First call - cache miss
            let start1 = Instant::now();
            let embedding1 = rag.generate_embedding(text).await.unwrap();
            let duration1 = start1.elapsed();

            // Second call - cache hit
            let start2 = Instant::now();
            let embedding2 = rag.generate_embedding(text).await.unwrap();
            let duration2 = start2.elapsed();

            assert_eq!(embedding1, embedding2);
            assert!(duration2 < duration1 / 5); // Cache should be 5x+ faster
        }
    }

    mod performance {
        #[tokio::test]
        async fn test_vector_search_performance() {
            let rag = RagTool::new(RagConfig::default()).await.unwrap();

            // Index 10,000 documents
            for i in 0..10000 {
                rag.index_document(i.to_string(), &format!("Document {}", i)).await.unwrap();
            }

            // Search should still be fast
            let start = Instant::now();
            let results = rag.search("query", 10).await.unwrap();
            let duration = start.elapsed();

            assert!(duration < Duration::from_millis(100)); // < 100ms for 10k docs
            assert_eq!(results.len(), 10);
        }
    }
}
```

**Enhanced Security & Cost Controls for Phase 3**

**Database Strategy Security Hardening**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStrategyConfig {
    pub driver: DatabaseDriver,
    pub connection_string: String,
    pub query: String,

    // Security Controls
    #[serde(default = "default_read_only")]
    pub read_only: bool,  // Default: true

    #[serde(default)]
    pub allowed_tables: Vec<String>,  // Whitelist only, empty = all tables

    #[serde(default = "default_query_timeout")]
    pub query_timeout_ms: u64,  // Default: 5000ms

    #[serde(default = "default_max_rows")]
    pub max_rows: usize,  // Default: 1000

    #[serde(default)]
    pub allowed_operations: Vec<SqlOperation>,  // Default: [SELECT]
}

fn default_read_only() -> bool { true }
fn default_query_timeout() -> u64 { 5000 }
fn default_max_rows() -> usize { 1000 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SqlOperation {
    Select,
    Insert,
    Update,
    Delete,
}

impl DatabaseStrategy {
    fn validate_query(&self, sql: &str) -> Result<(), DatabaseError> {
        // 1. Check if query contains only allowed operations
        let sql_lower = sql.to_lowercase();

        if self.config.read_only {
            if sql_lower.contains("insert")
                || sql_lower.contains("update")
                || sql_lower.contains("delete")
                || sql_lower.contains("drop")
                || sql_lower.contains("alter")
                || sql_lower.contains("create") {
                return Err(DatabaseError::UnauthorizedOperation(
                    "Only SELECT operations allowed in read-only mode".into()
                ));
            }
        }

        // 2. Validate table whitelist
        if !self.config.allowed_tables.is_empty() {
            // Parse SQL to extract table names (simplified)
            // In production, use proper SQL parser
            let tables_in_query = extract_table_names(sql)?;
            for table in tables_in_query {
                if !self.config.allowed_tables.contains(&table) {
                    return Err(DatabaseError::UnauthorizedTable(table));
                }
            }
        }

        // 3. Must use parameterized queries, not string interpolation
        if sql.contains("'") || sql.contains("\"") {
            return Err(DatabaseError::UnsafeQuery(
                "Use parameterized queries, not string literals".into()
            ));
        }

        Ok(())
    }
}

// Security Tests
#[cfg(test)]
mod database_security_tests {
    #[tokio::test]
    async fn test_read_only_mode_blocks_write_operations() {
        let config = DatabaseStrategyConfig {
            read_only: true,
            ..Default::default()
        };
        let strategy = DatabaseStrategy::new(config).await.unwrap();

        let malicious_queries = vec![
            "DROP TABLE users;",
            "DELETE FROM users WHERE id = 1;",
            "UPDATE users SET role = 'admin';",
            "INSERT INTO users VALUES ('hacker', 'admin');",
        ];

        for query in malicious_queries {
            let result = strategy.validate_query(query);
            assert!(result.is_err());
        }
    }

    #[tokio::test]
    async fn test_table_whitelist_enforcement() {
        let config = DatabaseStrategyConfig {
            allowed_tables: vec!["users".into(), "posts".into()],
            ..Default::default()
        };
        let strategy = DatabaseStrategy::new(config).await.unwrap();

        // Allowed
        assert!(strategy.validate_query("SELECT * FROM users").is_ok());
        assert!(strategy.validate_query("SELECT * FROM posts").is_ok());

        // Blocked
        assert!(strategy.validate_query("SELECT * FROM admin_secrets").is_err());
    }

    #[tokio::test]
    async fn test_query_timeout_enforcement() {
        let config = DatabaseStrategyConfig {
            query_timeout_ms: 100,
            ..Default::default()
        };
        let strategy = DatabaseStrategy::new(config).await.unwrap();

        // Slow query should timeout
        let result = strategy.execute_query(
            "SELECT pg_sleep(10);"  // Sleep for 10 seconds
        ).await;

        assert!(matches!(result.unwrap_err(), DatabaseError::Timeout));
    }
}
```

**LLM Strategy Cost Controls**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmStrategyConfig {
    pub provider: LlmProvider,
    pub model: String,
    pub temperature: f32,

    // Cost Controls
    #[serde(default = "default_max_cost_per_request")]
    pub max_cost_per_request: f64,  // Default: $0.10

    #[serde(default = "default_daily_budget")]
    pub daily_budget: f64,  // Default: $10.00

    #[serde(default = "default_cost_alert_threshold")]
    pub cost_alert_threshold: f64,  // Default: 80% of daily budget

    // Local LLM Support
    #[serde(default)]
    pub use_local_model: bool,

    #[serde(default)]
    pub local_model_url: Option<String>,  // e.g., "http://localhost:11434" for Ollama

    // Rate Limiting
    #[serde(default = "default_max_requests_per_minute")]
    pub max_requests_per_minute: u32,  // Default: 10
}

fn default_max_cost_per_request() -> f64 { 0.10 }
fn default_daily_budget() -> f64 { 10.0 }
fn default_cost_alert_threshold() -> f64 { 8.0 }  // 80% of $10
fn default_max_requests_per_minute() -> u32 { 10 }

pub struct LlmStrategy {
    config: LlmStrategyConfig,
    cost_tracker: Arc<RwLock<CostTracker>>,
    rate_limiter: Arc<RateLimiter>,
}

#[derive(Debug, Default)]
pub struct CostTracker {
    pub total_cost_today: f64,
    pub requests_today: usize,
    pub last_reset: Option<Instant>,
}

impl LlmStrategy {
    pub async fn generate(&self, context: &RequestContext) -> Result<serde_json::Value, Error> {
        // 1. Check daily budget
        let current_cost = self.cost_tracker.read().await.total_cost_today;
        if current_cost >= self.config.daily_budget {
            return Err(Error::BudgetExceeded {
                budget: self.config.daily_budget,
                current: current_cost,
            });
        }

        // 2. Check alert threshold
        if current_cost >= self.config.cost_alert_threshold {
            tracing::warn!(
                "Cost alert: ${:.2} of ${:.2} budget used ({}%)",
                current_cost,
                self.config.daily_budget,
                (current_cost / self.config.daily_budget * 100.0)
            );
        }

        // 3. Rate limiting
        self.rate_limiter.wait_if_needed().await?;

        // 4. Execute LLM request
        let (response, cost) = if self.config.use_local_model {
            self.call_local_llm(context).await?
        } else {
            self.call_remote_llm(context).await?
        };

        // 5. Check per-request cost limit
        if cost > self.config.max_cost_per_request {
            return Err(Error::RequestCostExceeded {
                limit: self.config.max_cost_per_request,
                actual: cost,
            });
        }

        // 6. Update cost tracking
        self.cost_tracker.write().await.total_cost_today += cost;
        self.cost_tracker.write().await.requests_today += 1;

        tracing::info!("LLM request completed: ${:.4} (total today: ${:.2})", cost, current_cost + cost);

        Ok(response)
    }

    async fn call_local_llm(&self, context: &RequestContext) -> Result<(serde_json::Value, f64), Error> {
        // Local models have no cost
        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/api/generate", self.config.local_model_url.as_ref().unwrap()))
            .json(&json!({
                "model": &self.config.model,
                "prompt": context.prompt,
                "temperature": self.config.temperature,
            }))
            .send()
            .await?;

        let result = response.json().await?;
        Ok((result, 0.0))  // No cost for local models
    }
}

// Cost Control Tests
#[cfg(test)]
mod llm_cost_tests {
    #[tokio::test]
    async fn test_daily_budget_enforcement() {
        let config = LlmStrategyConfig {
            daily_budget: 1.0,
            max_cost_per_request: 0.50,
            ..Default::default()
        };
        let strategy = LlmStrategy::new(config).await;

        // First 2 requests should succeed (2 * $0.50 = $1.00)
        strategy.generate(&context).await.unwrap();
        strategy.generate(&context).await.unwrap();

        // Third request should fail (would exceed budget)
        let result = strategy.generate(&context).await;
        assert!(matches!(result.unwrap_err(), Error::BudgetExceeded { .. }));
    }

    #[tokio::test]
    async fn test_cost_estimation_before_execution() {
        let strategy = LlmStrategy::new(config).await;

        let estimated_cost = strategy.estimate_cost(&context);
        assert!(estimated_cost > 0.0);
        assert!(estimated_cost < strategy.config.max_cost_per_request);
    }

    #[tokio::test]
    async fn test_local_llm_fallback() {
        let config = LlmStrategyConfig {
            use_local_model: true,
            local_model_url: Some("http://localhost:11434".into()),
            ..Default::default()
        };
        let strategy = LlmStrategy::new(config).await;

        let (response, cost) = strategy.call_local_llm(&context).await.unwrap();
        assert_eq!(cost, 0.0);  // Local models are free
    }
}
```

---

**🔍 CHECKPOINT: Phase 3 Review (End of Week 11)**

**Mandatory Review Before Proceeding to Phase 4**

**Objectives**
- Assess actual progress vs planned timeline
- Validate technology choices based on real implementation experience
- Re-estimate remaining work
- Decide whether to continue, pivot, or adjust scope

**Review Areas**

1. **Technical Health Check**
   - [ ] All Phase 1-3 features implemented and tested
   - [ ] Test coverage >80%
   - [ ] Performance targets met (>10k req/s for simple strategies)
   - [ ] No critical bugs or security issues
   - [ ] Technical debt is manageable

2. **Timeline Assessment**
   - [ ] Compare actual velocity vs planned velocity
   - [ ] Calculate estimated completion date for remaining phases
   - [ ] Identify any timeline risks or bottlenecks
   - [ ] **Decision**: Continue with current timeline OR adjust

3. **Technology Validation**
   - [ ] Rust MCP SDK working as expected (or alternative validated)
   - [ ] Multi-language scripting overhead acceptable
   - [ ] Database/LLM integrations stable
   - [ ] No major technology blockers identified

4. **Scope Validation**
   - [ ] Review Phase 7 features against user needs (if early users available)
   - [ ] Identify features to prioritize vs defer
   - [ ] **Decision**: Keep all Phase 7 features OR split into multiple releases

5. **Risk Assessment**
   - [ ] Review risk register
   - [ ] Identify new risks discovered during implementation
   - [ ] Update risk mitigation strategies

**Deliverables from Checkpoint**
- [ ] Checkpoint report documenting findings
- [ ] Updated project timeline (if needed)
- [ ] Go/No-Go decision for Phase 4-8
- [ ] Revised scope for Phase 7 (if needed)
- [ ] Risk mitigation plan updates

**Success Criteria for Proceeding**
- ✅ Phases 1-3 substantially complete (>90%)
- ✅ No critical blockers identified
- ✅ Team confident in technology stack
- ✅ Realistic path to completion visible
- ✅ Stakeholder buy-in on timeline/scope

**Possible Outcomes**
1. **Green Light**: Continue to Phase 4 as planned
2. **Yellow Light**: Continue with adjusted timeline or scope
3. **Red Light**: Major pivot needed (unlikely if Week 0 done well)

---

### Phase 4: Authentication & Security (Weeks 12-13)

**Goals**
- Multiple authentication methods
- Security hardening
- Rate limiting

**Deliverables**
- [ ] Authentication middleware
- [ ] API key authentication
- [ ] Bearer token (JWT) authentication
- [ ] Basic auth
- [ ] OAuth 2.0 support
- [ ] Custom script-based auth
- [ ] mTLS support
- [ ] Rate limiting
- [ ] Security audit
- [ ] Penetration testing

**Auth Middleware**
```rust
pub struct AuthMiddleware {
    config: AuthConfig,
    validators: HashMap<AuthMode, Box<dyn AuthValidator>>,
}

#[async_trait]
pub trait AuthValidator: Send + Sync {
    async fn validate(
        &self,
        request: &Request,
    ) -> Result<AuthContext, AuthError>;
}
```

### Phase 5: Configuration & Live Reload (Weeks 14-15)

**Goals**
- Advanced configuration features
- Live reload mechanism
- Configuration validation

**Deliverables**
- [ ] Modular configuration loading
- [ ] Configuration inheritance
- [ ] Environment variable substitution
- [ ] File watcher for live reload
- [ ] Configuration validation framework
- [ ] Configuration migration tools
- [ ] Configuration documentation generator

**Live Reload**
```rust
pub struct ConfigWatcher {
    watcher: RecommendedWatcher,
    debouncer: Debouncer,
    reload_tx: mpsc::Sender<ReloadEvent>,
}

impl ConfigWatcher {
    pub async fn watch(&mut self) -> Result<(), WatchError> {
        // Watch configuration files
        // Debounce changes
        // Trigger reload
    }
}
```

### Phase 6: Observability & Operations (Weeks 16-17)

**Goals**
- Comprehensive logging
- Metrics and monitoring
- Health checks
- Admin interface

**Deliverables**
- [ ] Structured logging with tracing
- [ ] Prometheus metrics
- [ ] OpenTelemetry support
- [ ] Health check endpoints
- [ ] Admin REST API
- [ ] Dashboard (optional web UI)
- [ ] Alerting configuration
- [ ] Operational runbook

**Metrics**
```rust
pub struct MetricsCollector {
    // Request metrics
    requests_total: Counter,
    request_duration: Histogram,
    requests_in_flight: Gauge,

    // Strategy metrics
    strategy_executions: CounterVec,
    strategy_errors: CounterVec,
    strategy_duration: HistogramVec,

    // Cache metrics
    cache_hits: Counter,
    cache_misses: Counter,

    // LLM metrics
    llm_calls: Counter,
    llm_cost: Counter,
}
```

---

**🔍 CHECKPOINT: Phase 6 Review (End of Week 17)**

**Mandatory Review Before Proceeding to Phase 7**

**Objectives**
- Assess v1.0 readiness (Phases 1-6 complete the core platform)
- Validate with early users if possible
- Decide on Phase 7+ feature prioritization based on feedback
- Prepare for v1.0 release or continue to advanced features

**Review Areas**

1. **v1.0 Readiness Check**
   - [ ] All core features complete (MCP protocol, mock strategies, auth, config, observability)
   - [ ] Test coverage >80%
   - [ ] Performance targets met
   - [ ] Security audit completed (or scheduled)
   - [ ] Documentation complete for core features
   - [ ] No release-blocking bugs

2. **Early User Feedback** (if available)
   - [ ] Deploy alpha/beta to select users
   - [ ] Collect feedback on core features
   - [ ] Identify most requested Phase 7 features
   - [ ] Validate assumptions about workflow engine, Web UI, etc.
   - [ ] **Decision**: Which Phase 7 features to prioritize

3. **Release Strategy Decision**
   - **Option A**: Release v1.0 now (Phases 1-6), iterate on Phase 7 based on feedback
   - **Option B**: Continue to select Phase 7 features before v1.0
   - **Option C**: Split Phase 7 across multiple releases (v1.1, v1.2, v1.3)

4. **Technical Debt Assessment**
   - [ ] Review accumulated technical debt
   - [ ] Identify refactoring needs
   - [ ] Plan debt paydown (before or during Phase 7)

5. **Team Health Check**
   - [ ] Assess team velocity and morale
   - [ ] Identify burnout risks
   - [ ] Adjust timeline if needed

**Deliverables from Checkpoint**
- [ ] v1.0 release candidate OR decision to continue development
- [ ] Prioritized Phase 7 features based on user feedback
- [ ] Revised timeline for Phase 7 sub-phases
- [ ] Go/No-Go for each Phase 7 feature
- [ ] Release plan (marketing, documentation, support)

**Success Criteria for v1.0 Release**
- ✅ Core platform fully functional
- ✅ Production-ready (security, performance, reliability)
- ✅ Documented and testable
- ✅ At least 5 early users providing positive feedback
- ✅ No critical bugs or security issues

**Possible Outcomes**
1. **Release v1.0**: Ship core platform, gather feedback, plan Phase 7
2. **Continue to Phase 7a**: Add most requested feature before v1.0
3. **Pause & Refactor**: Address technical debt before new features

---

### Phase 7: Advanced Features (Weeks 18-35)

**Overview**

Phase 7 contains the most ambitious features: Workflow Engine, Web UI, Model System, and Multi-Language Scripting. Based on the technical review, this phase has been split into four sub-phases (7a-7d) for realistic planning.

**Total Duration**: 18 weeks
**Recommended Approach**: Ship each sub-phase as incremental release (v1.1, v1.2, v1.3, v1.4)

---

### Phase 7a: Workflow Engine & Agent System (Weeks 18-21)

**Goals**
- Workflow engine with branching and looping
- Agent orchestration (single and multi-agent)
- Model definitions and relationships
- Workflow exposed as MCP tools

**Success Criteria**
- Workflows can call MCP tools
- Support if/else, loops, parallel execution
- Agent state management working
- Model system with relationships operational
- >80% test coverage

**Deliverables**

**Workflow Engine**
- [ ] Workflow definition parser (YAML/JSON/TOML)
- [ ] Workflow execution engine
  - [ ] Unit tests for step execution
  - [ ] Integration tests for complete workflows
  - [ ] Tests for branching logic (if/else, switch)
  - [ ] Tests for looping constructs (for, while, foreach)
  - [ ] Tests for parallel execution
  - [ ] Tests for sub-workflows
- [ ] Control flow implementation
  - [ ] If/else conditional tests
  - [ ] Switch/case statement tests
  - [ ] While loop termination tests
  - [ ] For loop iteration tests
  - [ ] Foreach array iteration tests
- [ ] MCP tool integration from workflows
  - [ ] Unit tests for tool call resolution
  - [ ] Integration tests with real MCP tools
  - [ ] Tests for tool call retry logic
  - [ ] Tests for tool call error handling
- [ ] Scripted task support
  - [ ] Tests for inline Python scripts
  - [ ] Tests for inline Lua scripts
  - [ ] Tests for inline Rhai scripts
  - [ ] Tests for script timeout handling
- [ ] State management and context passing
  - [ ] Tests for variable scope
  - [ ] Tests for context immutability
  - [ ] Tests for state persistence between steps
- [ ] Error handling and retry logic
  - [ ] Tests for exponential backoff
  - [ ] Tests for max retry limits
  - [ ] Tests for error continuation
  - [ ] Tests for error propagation
- [ ] Workflow visualization (DAG generation)
  - [ ] Tests for Mermaid diagram generation
  - [ ] Tests for complex workflow graph rendering
- [ ] Workflow validation
  - [ ] Tests for circular dependency detection
  - [ ] Tests for undefined variable detection
  - [ ] Tests for type checking
- [ ] Expose workflows as MCP tools
  - [ ] Unit tests for workflow-to-tool conversion
  - [ ] Integration tests for workflow invocation via MCP
- [ ] Performance tests
  - [ ] Benchmark tests for workflow execution
  - [ ] Tests for parallel step optimization
  - [ ] Memory usage tests for large workflows
- [ ] Security tests
  - [ ] Tests for script sandboxing
  - [ ] Tests for resource access limits
  - [ ] Tests for infinite loop prevention

**Agent System**
- [ ] Single agent endpoints with tool calling
  - [ ] Unit tests for agent initialization
  - [ ] Tests for tool invocation from agents
  - [ ] Tests for agent state persistence
- [ ] Multi-agent orchestration patterns
  - [ ] Hierarchical agent tests
  - [ ] Collaborative agent tests
  - [ ] Parallel agent execution tests
- [ ] Agent exposed as MCP tools
  - [ ] Tests for agent-to-tool conversion
  - [ ] Integration tests for agent invocation

**Model System (Odoo-style)**
- [ ] Model definition schema (YAML/JSON)
- [ ] Field types and validation
- [ ] Model relationships
  - [ ] many2one relationship tests
  - [ ] one2many relationship tests
  - [ ] many2many relationship tests
  - [ ] one2one relationship tests
- [ ] Model-aware data generation
  - [ ] Tests for relationship integrity
  - [ ] Tests for foreign key constraints
- [ ] Model registry and lifecycle management

---

### Phase 7b: Multi-Language Scripting (Weeks 22-25)

**Goals**
- Add support for multiple scripting languages beyond Rhai
- Unified scripting interface across languages
- Secure sandboxing for all languages
- Performance optimization

**Success Criteria**
- 4-5 languages supported (Rhai, Python, Lua, and optionally Ruby/JS)
- All languages have equivalent sandboxing
- Performance overhead < 20% vs native
- Comprehensive security testing passed

**Deliverables**

**Multi-Language Support**
- [ ] Python (pyo3) integration
  - [ ] Basic script execution tests
  - [ ] GIL handling tests
  - [ ] Memory management tests
  - [ ] Sandboxing tests (restricted imports, no file access)
  - [ ] Timeout enforcement tests
  - [ ] Performance benchmarks
- [ ] Lua (mlua) integration
  - [ ] Script execution tests
  - [ ] Sandboxing tests
  - [ ] Performance benchmarks
- [ ] Ruby (rutie/magnus) integration (optional)
  - [ ] Basic integration tests
  - [ ] Security tests
- [ ] JavaScript (deno_core/boa) integration (optional)
  - [ ] Basic integration tests
  - [ ] WASM compatibility tests
- [ ] Unified scripting interface
  - [ ] Tests for language-agnostic API
  - [ ] Tests for context passing across languages
  - [ ] Tests for error handling consistency

**Security Enhancements**
- [ ] Per-language sandboxing configuration
  - [ ] Tests for restricted module imports
  - [ ] Tests for filesystem access denial
  - [ ] Tests for network access denial
- [ ] Resource limits per script
  - [ ] Memory limit enforcement tests
  - [ ] CPU time limit tests
  - [ ] Execution timeout tests
- [ ] Script validation and static analysis
  - [ ] Tests for malicious code detection
  - [ ] Tests for syntax validation

**Performance Optimization**
- [ ] Script compilation caching
  - [ ] Tests for cache hit rates
  - [ ] Tests for cache invalidation
- [ ] Parallel script execution
  - [ ] Tests for concurrent script running
  - [ ] Tests for resource isolation

---

### Phase 7c: Web UI (Leptos) (Weeks 26-31)

**Goals**
- Modern web interface for configuration and monitoring
- Visual workflow designer
- Live configuration editing with hot reload
- Real-time metrics dashboard

**Success Criteria**
- All major components functional
- WebSocket real-time updates working
- Monaco editor integrated
- Workflow designer usable
- Mobile-responsive design
- Accessibility compliance (WCAG 2.1 Level AA)

**Deliverables**

**Infrastructure**
- [ ] Project setup and build configuration
  - [ ] Leptos project structure
  - [ ] Trunk build configuration
  - [ ] TailwindCSS integration
- [ ] Dashboard component
  - [ ] Unit tests for stats display
  - [ ] Tests for WebSocket metrics updates
  - [ ] Tests for real-time data refresh
- [ ] Configuration editor
  - [ ] Unit tests for file tree navigation
  - [ ] Integration tests for file save/reload
  - [ ] Tests for syntax validation
  - [ ] Tests for real-time error detection
  - [ ] Tests for Monaco editor integration
- [ ] Visual workflow designer
  - [ ] Unit tests for drag-and-drop functionality
  - [ ] Tests for workflow canvas rendering
  - [ ] Tests for step property editing
  - [ ] Tests for workflow export to YAML
  - [ ] Tests for Mermaid diagram generation
- [ ] Resource browser
  - [ ] Unit tests for search functionality
  - [ ] Tests for resource detail view
  - [ ] Integration tests for resource testing
- [ ] Agent dashboard
  - [ ] Unit tests for agent status display
  - [ ] Tests for conversation history
  - [ ] Integration tests for agent testing
- [ ] API layer (Axum)
  - [ ] Unit tests for all API endpoints
  - [ ] Integration tests for hot reload
  - [ ] WebSocket tests for metrics streaming
  - [ ] Tests for CORS configuration
- [ ] Configuration hot reload
  - [ ] Unit tests for file watcher
  - [ ] Integration tests for config reload
  - [ ] Tests for partial config updates
  - [ ] Tests for reload error handling
- [ ] UI/UX tests
  - [ ] Component rendering tests
  - [ ] User flow tests
  - [ ] Accessibility tests
  - [ ] Responsive design tests
  - [ ] Performance tests (bundle size, load time)

**Deployment**
- [ ] Docker image with UI
- [ ] Standalone binary with embedded UI
- [ ] CDN deployment for static assets
- [ ] UI versioning strategy

---

### Phase 7d: Comprehensive Testing & Documentation (Weeks 32-35)

**Goals**
- Complete test coverage across all features
- Comprehensive documentation for all capabilities
- Examples and tutorials for common use cases
- Performance validation and optimization

**Success Criteria**
- >85% test coverage across entire codebase
- All features documented with examples
- Performance benchmarks published
- Tutorial videos created
- Migration guides available

**Deliverables**

**Automatic Testing Infrastructure**
- [ ] Built-in MCP test client
  - [ ] Tests for client initialization
  - [ ] Tests for protocol compliance
- [ ] Automatic test generation from configuration
  - [ ] Tests for resource configurations
  - [ ] Tests for tool configurations
  - [ ] Tests for workflow configurations
- [ ] Protocol compliance test suite
  - [ ] Tests for all MCP protocol operations
  - [ ] Tests for error handling
  - [ ] Tests for edge cases
- [ ] Contract testing framework
  - [ ] Provider contract tests
  - [ ] Consumer contract tests
  - [ ] Backward compatibility tests
- [ ] Snapshot testing
  - [ ] Tests for response formats
  - [ ] Tests for configuration formats
- [ ] Scenario-based testing
  - [ ] Common usage scenario tests
  - [ ] Complex workflow scenario tests
- [ ] Multi-client testing
  - [ ] TypeScript client tests
  - [ ] Python client tests
  - [ ] Rust client tests
- [ ] Fuzzing and property-based tests
  - [ ] Protocol parser fuzzing
  - [ ] Configuration parser fuzzing
  - [ ] Property-based workflow tests
- [ ] Test reporting
  - [ ] JUnit XML output
  - [ ] HTML reports
  - [ ] JSON reports for CI integration
- [ ] Performance benchmarks
  - [ ] Benchmark suite for all mock strategies
  - [ ] Load testing scenarios
  - [ ] Stress testing
  - [ ] Soak testing (long-running stability)

**Documentation**
- [ ] User documentation
  - [ ] Getting started guide
  - [ ] Installation instructions
  - [ ] Basic configuration tutorial
  - [ ] Advanced configuration guide
- [ ] API reference
  - [ ] MCP protocol endpoints
  - [ ] REST API documentation (if applicable)
  - [ ] WebSocket API documentation
- [ ] Configuration reference
  - [ ] Complete TOML/YAML/JSON schema documentation
  - [ ] All configuration options explained
  - [ ] Best practices and recommendations
- [ ] Feature guides
  - [ ] Mock strategies guide
  - [ ] Workflow engine guide
  - [ ] Agent orchestration guide
  - [ ] Multi-language scripting guide
  - [ ] Web UI user guide
- [ ] Tutorial examples
  - [ ] "Hello World" example
  - [ ] Basic mocking example
  - [ ] LLM integration example
  - [ ] Database mocking example
  - [ ] Workflow example
  - [ ] Agent example
- [ ] Video demonstrations
  - [ ] Product overview (5 min)
  - [ ] Quick start (10 min)
  - [ ] Advanced features (15 min)
  - [ ] Web UI walkthrough (10 min)
- [ ] Migration guides
  - [ ] Migrating from WireMock
  - [ ] Migrating from Mock Service Worker
  - [ ] Version upgrade guides
- [ ] Developer documentation
  - [ ] Architecture overview
  - [ ] Contributing guide
  - [ ] Development setup
  - [ ] Testing guidelines
- [ ] Troubleshooting guide
  - [ ] Common issues and solutions
  - [ ] Performance tuning
  - [ ] Debugging tips

---

### Phase 8: Polish & Release (Week 36)

**Goals**
- Final production readiness checks
- Release preparation
- Community launch

**Success Criteria**
- No critical bugs
- Performance targets met (>10k req/s)
- Documentation complete and published
- Release artifacts built and tested
- Community launch executed

**Deliverables**

**Release Artifacts**
- [ ] Release builds for major platforms (Linux, macOS, Windows)
- [ ] Docker images (Docker Hub, GitHub Container Registry)
- [ ] Installation scripts (Homebrew, apt, yum)
- [ ] Release notes with changelog
- [ ] Version tagging (v1.0.0)

**Repository & Community**
- [ ] GitHub repository polished
  - [ ] README with badges and quick start
  - [ ] Issue templates
  - [ ] Pull request template
  - [ ] Contributing guidelines
  - [ ] Code of conduct
- [ ] License file (MIT or Apache 2.0)
- [ ] Security policy (SECURITY.md)
- [ ] Funding options (GitHub Sponsors, Open Collective)

**Marketing & Launch**
- [ ] Blog post announcement
- [ ] Hacker News launch post
- [ ] Reddit posts (r/rust, r/programming)
- [ ] Twitter/X announcement thread
- [ ] Dev.to article
- [ ] Product Hunt launch

**Final Checks**
- [ ] Security audit report reviewed
- [ ] Performance benchmarks published
- [ ] All critical and high-priority bugs resolved
- [ ] Backward compatibility guarantees documented
- [ ] Support channels established (Discord, GitHub Discussions)

---

## Testing Strategy

### Unit Tests

**Coverage Goals**
- Minimum 80% code coverage
- 100% coverage for critical paths
- Test all public APIs

**Test Organization**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_random_strategy_generates_valid_data() {
        // Arrange
        let strategy = RandomStrategy::new(config);
        let context = RequestContext::mock();

        // Act
        let result = strategy.generate(&context).await;

        // Assert
        assert!(result.is_ok());
        assert_valid_schema(result.unwrap());
    }
}
```

### Integration Tests

**Test Scenarios**
- End-to-end MCP protocol flows
- Configuration loading and validation
- Live reload functionality
- Authentication flows
- All mock strategies with real dependencies

**Example Test**
```rust
#[tokio::test]
async fn test_llm_strategy_with_real_api() {
    // Setup
    let api_key = env::var("ANTHROPIC_API_KEY").ok();
    if api_key.is_none() {
        return; // Skip if no API key
    }

    let strategy = LlmStrategy::new(LlmConfig {
        provider: "anthropic",
        model: "claude-3-5-sonnet-20241022",
        api_key: api_key.unwrap(),
        // ...
    });

    // Execute
    let response = strategy.generate(&context).await;

    // Verify
    assert!(response.is_ok());
    // Validate response structure
}
```

### Performance Tests

**Benchmarks**
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_random_strategy(c: &mut Criterion) {
    let strategy = RandomStrategy::new(config);
    let context = RequestContext::mock();

    c.bench_function("random_strategy_generate", |b| {
        b.iter(|| {
            strategy.generate(black_box(&context))
        });
    });
}

criterion_group!(benches, benchmark_random_strategy);
criterion_main!(benches);
```

**Load Tests**
- Use `k6` or `wrk` for HTTP load testing
- Custom load generator for stdio transport
- Target: 10,000 requests/second
- Duration: sustained load for 10 minutes
- Monitor: CPU, memory, latency

### Compliance Tests

**MCP Protocol Compliance**
- Test all required protocol features
- Validate against protocol specification
- Test error handling
- Test edge cases

**Example**
```rust
#[tokio::test]
async fn test_mcp_initialize_handshake() {
    // Test proper initialize/initialized handshake
    let server = MetisServer::new(config);

    // Send initialize request
    let response = server.handle_request(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    })).await;

    // Verify response
    assert_eq!(response["result"]["protocolVersion"], "2024-11-05");
    assert!(response["result"]["capabilities"].is_object());
}
```

### Security Tests

**Security Test Cases**
- Authentication bypass attempts
- Injection attacks (SQL, script)
- Path traversal attacks
- Rate limit enforcement
- Token validation
- Configuration validation (no secrets in logs)

**Tools**
- OWASP ZAP for security scanning
- Custom fuzzing with `cargo-fuzz`
- Dependency vulnerability scanning with `cargo-audit`

---

## Automatic Testing with MCP Clients

### Overview

Metis includes a comprehensive automatic testing framework that uses real MCP clients to validate server behavior, protocol compliance, and response accuracy. This ensures that the mock server behaves correctly and matches expected MCP protocol specifications.

### Test Client Architecture

**Built-in Test Client**
```rust
pub struct MetisTestClient {
    transport: Box<dyn McpTransport>,
    config: TestClientConfig,
    session: ClientSession,
    recorder: RequestRecorder,
}

impl MetisTestClient {
    pub async fn connect(config: TestClientConfig) -> Result<Self, ClientError>;
    pub async fn initialize(&mut self) -> Result<InitializeResult, ClientError>;
    pub async fn list_resources(&self) -> Result<Vec<Resource>, ClientError>;
    pub async fn read_resource(&self, uri: &str) -> Result<ResourceContent, ClientError>;
    pub async fn call_tool(&self, name: &str, args: Value) -> Result<ToolResult, ClientError>;
    pub async fn get_prompt(&self, name: &str, args: Value) -> Result<PromptResult, ClientError>;
}
```

### Automatic Test Generation

**Test Suite Generation from Configuration**

Metis automatically generates test cases based on the server configuration:

```toml
[testing.auto_generate]
enabled = true
output_dir = "tests/generated"

# Generate tests for all resources
[testing.auto_generate.resources]
enabled = true
test_read = true
test_list = true
test_subscribe = false

# Generate tests for all tools
[testing.auto_generate.tools]
enabled = true
test_all_inputs = true
test_edge_cases = true
test_error_conditions = true

# Generate tests for all prompts
[testing.auto_generate.prompts]
enabled = true
test_argument_combinations = true
```

**Generated Test Example**
```rust
// Auto-generated from config/resources/users.toml
#[tokio::test]
async fn test_resource_db_users() {
    let mut client = MetisTestClient::connect(test_config()).await.unwrap();
    client.initialize().await.unwrap();

    // Test resource listing
    let resources = client.list_resources().await.unwrap();
    assert!(resources.iter().any(|r| r.uri == "db://users"));

    // Test resource reading
    let content = client.read_resource("db://users").await.unwrap();
    assert_eq!(content.mime_type, "application/json");

    // Validate schema
    let data: Vec<User> = serde_json::from_str(&content.text).unwrap();
    assert!(data.len() >= 5);
    assert!(data.len() <= 20);

    for user in data {
        assert!(!user.username.is_empty());
        assert!(user.email.contains('@'));
    }
}
```

### Protocol Compliance Testing

**Compliance Test Suite**

Automatic tests that validate MCP protocol compliance:

```rust
pub struct ProtocolComplianceTests {
    client: MetisTestClient,
}

impl ProtocolComplianceTests {
    // Test initialize handshake
    pub async fn test_initialize_handshake(&mut self) -> TestResult {
        let result = self.client.initialize().await?;
        assert_eq!(result.protocol_version, "2024-11-05");
        assert!(result.capabilities.is_some());
        Ok(())
    }

    // Test error handling
    pub async fn test_error_responses(&self) -> TestResult {
        // Call non-existent tool
        let result = self.client.call_tool("nonexistent", json!({})).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, ErrorCode::MethodNotFound);
        Ok(())
    }

    // Test resource lifecycle
    pub async fn test_resource_lifecycle(&self) -> TestResult {
        // List -> Read -> Validate
        let resources = self.client.list_resources().await?;
        for resource in resources {
            let content = self.client.read_resource(&resource.uri).await?;
            assert!(!content.text.is_empty());
        }
        Ok(())
    }
}
```

**Run Compliance Tests**
```bash
# Run all protocol compliance tests
cargo test --test protocol_compliance

# Run specific compliance category
cargo test --test protocol_compliance -- --exact test_initialize_handshake
```

### Response Validation

**Schema-Based Validation**

```toml
[testing.validation]
enabled = true

# Validate all responses against schemas
[testing.validation.schema]
strict_mode = true
validate_mime_types = true
validate_json_structure = true

# Custom validators
[[testing.validation.custom]]
name = "user_email_format"
script = """
fn validate(data) {
    if data.type == "user" {
        return data.email.matches("^[^@]+@[^@]+\\.[^@]+$");
    }
    return true;
}
"""
```

**Validation in Tests**
```rust
#[tokio::test]
async fn test_tool_response_validation() {
    let mut client = MetisTestClient::connect(test_config()).await.unwrap();
    client.initialize().await.unwrap();

    let result = client.call_tool("search_database", json!({
        "query": "test"
    })).await.unwrap();

    // Validate against JSON schema
    let schema = load_schema("tools/search_database.json");
    assert!(validate_json(&result.content, &schema));

    // Validate response structure
    let data: Vec<SearchResult> = serde_json::from_value(result.content).unwrap();
    for item in data {
        assert!(!item.id.is_empty());
        assert!(!item.title.is_empty());
        assert!(item.score >= 0.0 && item.score <= 1.0);
    }
}
```

### Contract Testing

**Consumer-Driven Contract Tests**

```toml
[testing.contracts]
enabled = true
contracts_dir = "tests/contracts"

# Define expected contracts
[[testing.contracts.definitions]]
name = "user_service_contract"
description = "Contract for user service client"
file = "contracts/user_service.json"
```

**Contract Definition** (`contracts/user_service.json`)
```json
{
  "provider": "metis",
  "consumer": "user_service",
  "interactions": [
    {
      "description": "Get user by ID",
      "request": {
        "method": "tools/call",
        "params": {
          "name": "get_user",
          "arguments": {
            "id": "123"
          }
        }
      },
      "response": {
        "status": "success",
        "content": {
          "id": "string",
          "username": "string",
          "email": "string"
        }
      }
    }
  ]
}
```

**Contract Test Execution**
```rust
#[tokio::test]
async fn test_user_service_contract() {
    let contract = Contract::load("contracts/user_service.json").unwrap();
    let mut client = MetisTestClient::connect(test_config()).await.unwrap();
    client.initialize().await.unwrap();

    for interaction in contract.interactions {
        let result = client.execute_request(&interaction.request).await.unwrap();
        assert!(contract.validate_response(&result, &interaction.response));
    }
}
```

### Snapshot Testing

**Response Snapshot Management**

```toml
[testing.snapshots]
enabled = true
snapshot_dir = "tests/snapshots"
update_mode = "review"  # review, auto, manual

# Snapshot comparison options
[testing.snapshots.comparison]
ignore_timestamps = true
ignore_random_ids = true
fuzzy_numbers = true
tolerance_percent = 5.0
```

**Snapshot Test**
```rust
#[tokio::test]
async fn test_search_tool_snapshot() {
    let mut client = MetisTestClient::connect(test_config()).await.unwrap();
    client.initialize().await.unwrap();

    let result = client.call_tool("search", json!({
        "query": "rust programming"
    })).await.unwrap();

    // Compare with stored snapshot
    assert_snapshot!("search_rust_programming", result.content, {
        ".**.id" => "[uuid]",
        ".**.created_at" => "[timestamp]",
        ".**.score" => "[float]"
    });
}
```

**Update Snapshots**
```bash
# Review and update snapshots
cargo test -- --update-snapshots

# Auto-accept all snapshot changes
cargo test -- --update-snapshots --accept-all
```

### Scenario-Based Testing

**Test Scenarios Definition**

```toml
[[testing.scenarios]]
name = "user_registration_flow"
description = "Complete user registration workflow"
steps = [
    { action = "call_tool", tool = "check_username", args = { username = "newuser" } },
    { action = "call_tool", tool = "create_user", args = { username = "newuser", email = "new@example.com" } },
    { action = "read_resource", uri = "db://users/{user_id}" },
    { action = "call_tool", tool = "send_welcome_email", args = { user_id = "{user_id}" } }
]

[[testing.scenarios]]
name = "data_pipeline_flow"
description = "Extract, transform, load data"
steps = [
    { action = "call_tool", tool = "extract_data", args = { source = "api" } },
    { action = "call_tool", tool = "transform_data", args = { data = "{previous_result}" } },
    { action = "call_tool", tool = "load_data", args = { data = "{previous_result}", target = "warehouse" } }
]
```

**Scenario Test Execution**
```rust
pub struct ScenarioRunner {
    client: MetisTestClient,
    context: HashMap<String, Value>,
}

impl ScenarioRunner {
    pub async fn run_scenario(&mut self, scenario: &Scenario) -> TestResult {
        for step in &scenario.steps {
            let result = match step.action {
                Action::CallTool => {
                    let args = self.interpolate_args(&step.args);
                    self.client.call_tool(&step.tool, args).await?
                },
                Action::ReadResource => {
                    let uri = self.interpolate_string(&step.uri);
                    self.client.read_resource(&uri).await?
                },
                // ... other actions
            };

            // Store result for next steps
            self.context.insert("previous_result".to_string(), result.content);

            // Validate step result
            if let Some(assertion) = &step.assert {
                self.validate_assertion(assertion, &result)?;
            }
        }
        Ok(())
    }
}

#[tokio::test]
async fn test_user_registration_scenario() {
    let scenario = Scenario::load("tests/scenarios/user_registration_flow.toml").unwrap();
    let mut runner = ScenarioRunner::new(test_config()).await.unwrap();
    runner.run_scenario(&scenario).await.unwrap();
}
```

### Fuzzing & Property-Based Testing

**Fuzz Testing Configuration**

```toml
[testing.fuzzing]
enabled = true
duration_minutes = 10
corpus_dir = "fuzz/corpus"

# Fuzz all tool inputs
[testing.fuzzing.tools]
enabled = true
max_input_size = 10000
mutation_depth = 10

# Fuzz authentication
[testing.fuzzing.auth]
enabled = true
test_invalid_tokens = true
test_malformed_headers = true
```

**Property-Based Tests**
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_search_tool_never_panics(query in "\\PC*", limit in 1..100u32) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut client = MetisTestClient::connect(test_config()).await.unwrap();
            client.initialize().await.unwrap();

            // Should never panic regardless of input
            let result = client.call_tool("search", json!({
                "query": query,
                "limit": limit
            })).await;

            // Either succeeds or returns error, but never panics
            assert!(result.is_ok() || result.is_err());
        });
    }
}
```

### Test Orchestration & CI Integration

**Test Configuration for CI**

```toml
[testing.ci]
enabled = true
fail_fast = false
parallel = true
max_parallel = 4

# Test categories
[testing.ci.categories]
unit = { enabled = true, timeout_sec = 300 }
integration = { enabled = true, timeout_sec = 600 }
contract = { enabled = true, timeout_sec = 300 }
compliance = { enabled = true, timeout_sec = 600 }
performance = { enabled = false }  # Run separately

# Coverage requirements
[testing.ci.coverage]
minimum_percent = 80
fail_below_minimum = true
```

**CI Workflow Example** (`.github/workflows/test.yml`)
```yaml
name: Automated Testing

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Cache dependencies
        uses: actions/cache@v3
        with:
          path: |
            ~/.cargo
            target/
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      - name: Run Unit Tests
        run: cargo test --lib

      - name: Run Integration Tests
        run: cargo test --test '*'

      - name: Run Protocol Compliance Tests
        run: cargo test --test protocol_compliance

      - name: Run Contract Tests
        run: cargo test --test contracts

      - name: Generate Test Client Tests
        run: |
          cargo run -- --config examples/test_server.toml &
          sleep 5
          cargo test --test client_tests

      - name: Code Coverage
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --out Xml --output-dir coverage

      - name: Upload Coverage
        uses: codecov/codecov-action@v3
        with:
          files: ./coverage/cobertura.xml
```

### Test Reporting

**Test Report Generation**

```toml
[testing.reporting]
enabled = true
format = "junit"  # junit, json, html
output_dir = "test_reports"

[testing.reporting.junit]
filename = "test_results.xml"

[testing.reporting.html]
filename = "test_report.html"
include_logs = true
include_metrics = true
```

**Custom Test Reporter**
```rust
pub struct TestReporter {
    results: Vec<TestResult>,
    start_time: Instant,
}

impl TestReporter {
    pub fn report_result(&mut self, name: &str, result: TestResult) {
        self.results.push(result);
        println!("✓ {} - {:?}", name, result.duration);
    }

    pub fn generate_report(&self, format: ReportFormat) -> String {
        match format {
            ReportFormat::JUnit => self.generate_junit(),
            ReportFormat::Html => self.generate_html(),
            ReportFormat::Json => self.generate_json(),
        }
    }
}
```

### Test CLI Commands

**Built-in Test Commands**

```bash
# Run all automatic tests
metis test --config metis.toml

# Run specific test category
metis test --category compliance

# Generate tests from configuration
metis test generate --output tests/generated

# Run with specific test client
metis test --client official-typescript

# Update snapshots
metis test --update-snapshots

# Run contract tests
metis test --contracts contracts/

# Continuous test mode (watch for changes)
metis test --watch

# Generate test report
metis test --report html --output test_report.html

# Run with multiple client implementations
metis test --clients typescript,python,rust
```

### Multi-Client Testing

**Test with Multiple MCP Client Implementations**

```toml
[testing.clients]
enabled = true

[[testing.clients.implementations]]
name = "typescript"
type = "npm"
package = "@modelcontextprotocol/sdk"
command = "node tests/run_typescript_client.js"

[[testing.clients.implementations]]
name = "python"
type = "pip"
package = "mcp"
command = "python tests/run_python_client.py"

[[testing.clients.implementations]]
name = "rust"
type = "cargo"
command = "cargo test --test rust_client"
```

**Multi-Client Test Runner**
```rust
pub async fn test_all_clients() -> TestResult {
    let clients = vec!["typescript", "python", "rust"];
    let mut results = HashMap::new();

    for client_name in clients {
        let result = run_client_tests(client_name).await;
        results.insert(client_name.to_string(), result);
    }

    // Verify all clients pass
    for (name, result) in &results {
        assert!(result.is_ok(), "Client {} failed: {:?}", name, result.err());
    }

    Ok(())
}
```

### Test Data Management

**Test Fixtures**

```toml
[testing.fixtures]
dir = "tests/fixtures"

[[testing.fixtures.sets]]
name = "sample_users"
file = "fixtures/users.json"
auto_load = true

[[testing.fixtures.sets]]
name = "test_scenarios"
file = "fixtures/scenarios/*.json"
pattern = true
```

**Fixture Usage in Tests**
```rust
#[tokio::test]
async fn test_with_fixtures() {
    let fixtures = TestFixtures::load().unwrap();
    let users = fixtures.get("sample_users").unwrap();

    let mut client = MetisTestClient::connect(test_config()).await.unwrap();
    client.initialize().await.unwrap();

    for user in users {
        let result = client.call_tool("create_user", user.clone()).await.unwrap();
        assert_eq!(result.status, "success");
    }
}
```

---

## Model Definitions & Relationships

### Overview

Metis supports defining data models with relationships similar to Odoo's ORM system. Models can define fields, relationships (one2many, many2one, one2one, many2many), and behaviors. These models can be used for generating structured mock data, database schema validation, and data consistency checks.

### Model Definition Files

Models are defined in JSON or YAML files stored in `config/models/` directory:

**`config/models/user.yaml`**
```yaml
name: "res.user"
description: "User model"
table: "users"

fields:
  - name: "id"
    type: "integer"
    primary_key: true
    auto_increment: true
    required: true

  - name: "name"
    type: "string"
    size: 100
    required: true
    index: true

  - name: "email"
    type: "string"
    size: 255
    required: true
    unique: true

  - name: "active"
    type: "boolean"
    default: true

  - name: "created_at"
    type: "datetime"
    default: "now"
    readonly: true

  - name: "updated_at"
    type: "datetime"
    default: "now"
    auto_update: true

  # Many2one relationship
  - name: "company_id"
    type: "many2one"
    relation: "res.company"
    ondelete: "restrict"  # restrict, cascade, set_null
    index: true

  # One2many relationship (inverse of many2one)
  - name: "order_ids"
    type: "one2many"
    relation: "sale.order"
    inverse_field: "user_id"
    readonly: true

  # Many2many relationship
  - name: "group_ids"
    type: "many2many"
    relation: "res.group"
    relation_table: "user_group_rel"
    column1: "user_id"
    column2: "group_id"

constraints:
  - type: "unique"
    fields: ["email"]
    name: "unique_user_email"

  - type: "check"
    condition: "LENGTH(name) >= 3"
    name: "name_min_length"

indexes:
  - fields: ["email"]
    unique: true
  - fields: ["company_id", "active"]
    name: "idx_company_active"

# Mock data generation for this model
mock:
  strategy: "model_aware"
  count: 100

  # Field-specific generators
  generators:
    name:
      type: "fake"
      fake_type: "name.full_name"

    email:
      type: "fake"
      fake_type: "internet.email"

    company_id:
      type: "foreign_key"
      from_model: "res.company"
      selection: "random"

    group_ids:
      type: "many2many"
      from_model: "res.group"
      min_records: 1
      max_records: 5
```

**`config/models/company.yaml`**
```yaml
name: "res.company"
description: "Company model"
table: "companies"

fields:
  - name: "id"
    type: "integer"
    primary_key: true

  - name: "name"
    type: "string"
    size: 200
    required: true

  - name: "vat"
    type: "string"
    size: 50

  # One2one relationship
  - name: "address_id"
    type: "one2one"
    relation: "res.address"
    ondelete: "cascade"

  # One2many (users belonging to this company)
  - name: "user_ids"
    type: "one2many"
    relation: "res.user"
    inverse_field: "company_id"

mock:
  strategy: "model_aware"
  count: 20
```

**`config/models/order.yaml`**
```yaml
name: "sale.order"
description: "Sales Order"
table: "sale_orders"

fields:
  - name: "id"
    type: "integer"
    primary_key: true

  - name: "name"
    type: "string"
    size: 64
    required: true

  - name: "user_id"
    type: "many2one"
    relation: "res.user"
    required: true

  - name: "order_line_ids"
    type: "one2many"
    relation: "sale.order.line"
    inverse_field: "order_id"

  - name: "total_amount"
    type: "float"
    computed: true
    compute_method: "script"
    compute_script: |
      # Calculate total from order lines
      sum(line.subtotal for line in self.order_line_ids)

  - name: "state"
    type: "selection"
    selection:
      - ["draft", "Draft"]
      - ["confirmed", "Confirmed"]
      - ["done", "Done"]
      - ["cancelled", "Cancelled"]
    default: "draft"

mock:
  strategy: "model_aware"
  count: 500

  generators:
    name:
      type: "pattern"
      pattern: "SO{:05d}"

    user_id:
      type: "foreign_key"
      from_model: "res.user"

    order_line_ids:
      type: "one2many"
      min_records: 1
      max_records: 10
      auto_generate: true
```

### Relationship Types

**Many2One**
```yaml
# User belongs to one Company
- name: "company_id"
  type: "many2one"
  relation: "res.company"
  required: true
  ondelete: "restrict"  # What happens when related record is deleted
```

**One2Many**
```yaml
# Company has many Users (inverse of many2one)
- name: "user_ids"
  type: "one2many"
  relation: "res.user"
  inverse_field: "company_id"  # Field on related model
  readonly: true  # Usually readonly in one2many
```

**Many2Many**
```yaml
# User can belong to many Groups, Group can have many Users
- name: "group_ids"
  type: "many2many"
  relation: "res.group"
  relation_table: "user_group_rel"  # Junction table
  column1: "user_id"
  column2: "group_id"
```

**One2One**
```yaml
# Company has one Address
- name: "address_id"
  type: "one2one"
  relation: "res.address"
  ondelete: "cascade"
```

### Model Architecture

```rust
pub struct ModelRegistry {
    models: HashMap<String, Model>,
    relationships: HashMap<String, Vec<Relationship>>,
}

pub struct Model {
    name: String,
    table: String,
    fields: Vec<Field>,
    constraints: Vec<Constraint>,
    indexes: Vec<Index>,
    mock_config: Option<MockConfig>,
}

pub struct Field {
    name: String,
    field_type: FieldType,
    required: bool,
    default: Option<Value>,
    validators: Vec<Validator>,
}

pub enum FieldType {
    // Basic types
    Integer,
    Float,
    String { size: usize },
    Boolean,
    Date,
    DateTime,
    Json,

    // Relationship types
    Many2One {
        relation: String,
        ondelete: OnDeleteAction,
    },
    One2Many {
        relation: String,
        inverse_field: String,
    },
    Many2Many {
        relation: String,
        relation_table: String,
        column1: String,
        column2: String,
    },
    One2One {
        relation: String,
        ondelete: OnDeleteAction,
    },

    // Special types
    Selection { options: Vec<(String, String)> },
    Computed { compute_fn: ComputeFunction },
}

pub enum OnDeleteAction {
    Cascade,    // Delete related records
    Restrict,   // Prevent deletion
    SetNull,    // Set foreign key to null
    SetDefault, // Set to default value
}
```

### Model-Aware Data Generation

```rust
pub struct ModelAwareGenerator {
    registry: Arc<ModelRegistry>,
    dependency_resolver: DependencyResolver,
}

impl ModelAwareGenerator {
    pub async fn generate_dataset(&self, model_name: &str, count: usize) -> Result<Vec<Value>, Error> {
        let model = self.registry.get(model_name)?;

        // 1. Resolve dependencies (create related records first)
        let dependencies = self.dependency_resolver.resolve(model)?;
        for dep in dependencies {
            self.ensure_records_exist(&dep).await?;
        }

        // 2. Generate records
        let mut records = Vec::new();
        for _ in 0..count {
            let mut record = json!({});

            for field in &model.fields {
                let value = match &field.field_type {
                    FieldType::Many2One { relation, .. } => {
                        // Select random existing record from related model
                        self.select_foreign_key(relation).await?
                    },
                    FieldType::One2Many { relation, inverse_field } => {
                        // Generate child records
                        self.generate_one2many(relation, inverse_field).await?
                    },
                    FieldType::Many2Many { relation, .. } => {
                        // Select multiple existing records
                        self.select_many2many(relation, 1, 5).await?
                    },
                    _ => {
                        // Generate regular field value
                        self.generate_field_value(field).await?
                    }
                };

                record[&field.name] = value;
            }

            records.push(record);
        }

        Ok(records)
    }
}
```

### Model Configuration

```toml
# metis.toml

[models]
enabled = true
config_dir = "config/models"
hot_reload = true

# Database backend for model storage
[models.database]
driver = "postgres"
connection_string_env = "DATABASE_URL"
auto_migrate = true  # Automatically create/update tables

# Model data generation
[models.mock]
enabled = true
auto_generate_on_startup = true
respect_relationships = true  # Honor foreign key constraints

[models.mock.defaults]
records_per_model = 100
```

### MCP Integration

Models are exposed through MCP resources and tools:

**Resources**
```toml
# Auto-generated resource for each model
[[resources]]
uri = "model://res.user"
name = "User Records"
description = "Access to user model data"
mime_type = "application/json"

[resources.mock]
strategy = "model_aware"
model = "res.user"

# Query parameters support
[[resources]]
uri = "model://res.user?company_id=5"
name = "Users filtered by company"
```

**Tools**
```toml
# Auto-generated CRUD tools for each model
[[tools]]
name = "model_create_res_user"
description = "Create a new user record"

[tools.input_schema]
type = "object"
properties = {
  name = { type = "string" },
  email = { type = "string" },
  company_id = { type = "integer" }
}

[[tools]]
name = "model_read_res_user"
description = "Read user records"

[tools.input_schema]
type = "object"
properties = {
  ids = { type = "array", items = { type = "integer" } },
  fields = { type = "array", items = { type = "string" } }
}

[[tools]]
name = "model_search_res_user"
description = "Search users with domain filter"

[tools.input_schema]
type = "object"
properties = {
  domain = { type = "array" },
  limit = { type = "integer" },
  offset = { type = "integer" }
}
```

### Multi-Language Scripting

Scripts can be written in multiple languages for computed fields, data generation, and custom logic:

**Python Scripting** (`pyo3`)
```yaml
# config/models/product.yaml
fields:
  - name: "price_with_tax"
    type: "float"
    computed: true
    compute_language: "python"
    compute_script: |
      def compute(record):
          return record['price'] * (1 + record['tax_rate'])
```

**Lua Scripting** (`mlua`)
```yaml
fields:
  - name: "full_address"
    type: "string"
    computed: true
    compute_language: "lua"
    compute_script: |
      function compute(record)
          return record.street .. ", " .. record.city .. " " .. record.zip
      end
```

**Ruby Scripting** (`rutie` / `magnus`)
```yaml
fields:
  - name: "display_name"
    type: "string"
    computed: true
    compute_language: "ruby"
    compute_script: |
      def compute(record)
        "#{record['first_name']} #{record['last_name']}".upcase
      end
```

**Rhai Scripting** (Native Rust)
```yaml
fields:
  - name: "status_badge"
    type: "string"
    computed: true
    compute_language: "rhai"
    compute_script: |
      fn compute(record) {
        if record.is_active {
          "🟢 Active"
        } else {
          "🔴 Inactive"
        }
      }
```

**JavaScript Scripting** (`deno_core` / `boa`)
```yaml
fields:
  - name: "age"
    type: "integer"
    computed: true
    compute_language: "javascript"
    compute_script: |
      function compute(record) {
        const birth = new Date(record.birth_date);
        const now = new Date();
        return now.getFullYear() - birth.getFullYear();
      }
```

**Script Configuration**
```toml
# metis.toml

[scripting]
enabled = true

# Enable specific languages
[scripting.languages]
python = { enabled = true, version = "3.11" }
lua = { enabled = true, version = "5.4" }
ruby = { enabled = true, version = "3.2" }
rhai = { enabled = true }
javascript = { enabled = true, runtime = "deno" }

# Security
[scripting.security]
sandbox = true
timeout_ms = 5000
max_memory_mb = 100
allow_network = false
allow_filesystem = false

# Script caching
[scripting.cache]
enabled = true
ttl_seconds = 300
```

**Script Execution Architecture**

```rust
pub struct ScriptExecutor {
    engines: HashMap<ScriptLanguage, Box<dyn ScriptEngine>>,
    sandbox: ScriptSandbox,
}

#[async_trait]
pub trait ScriptEngine: Send + Sync {
    async fn execute(
        &self,
        script: &str,
        context: &Value,
        timeout: Duration,
    ) -> Result<Value, ScriptError>;
}

pub struct PythonEngine {
    interpreter: Python,
    sandbox_config: SandboxConfig,
}

pub struct LuaEngine {
    lua: Lua,
    sandbox_config: SandboxConfig,
}

pub struct RubyEngine {
    vm: RubyVm,
    sandbox_config: SandboxConfig,
}
```

---

## Agent Endpoints & Orchestration

### Overview

Metis provides powerful agent endpoints that allow you to expose AI agents through both MCP protocol and HTTP/REST APIs. These agents can leverage all the MCP tools, resources, and prompts defined in the server. Agent endpoint definitions are stored in separate JSON or YAML files and loaded at runtime, allowing for flexible, hot-reloadable agent configurations.

### Single Agent Endpoints

**Concept**

Single agent endpoints expose individual AI agents that can process requests, call tools, and generate responses autonomously. Each agent has its own configuration, system prompt, and tool access.

**Architecture**

```rust
pub struct AgentEndpoint {
    id: String,
    config: AgentConfig,
    llm_client: Box<dyn LlmClient>,
    tool_executor: Arc<ToolExecutor>,
    state_manager: Arc<AgentStateManager>,
    history: ConversationHistory,
}

pub struct AgentConfig {
    name: String,
    description: String,
    endpoint_path: String,
    system_prompt: String,
    model: ModelConfig,
    tools: Vec<String>,  // Tool names agent can access
    resources: Vec<String>,  // Resources agent can access
    prompts: Vec<String>,  // Prompts agent can use
    max_iterations: u32,
    temperature: f32,
    streaming: bool,
}

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn generate(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
        config: &ModelConfig,
    ) -> Result<AgentResponse, LlmError>;

    async fn stream(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
        config: &ModelConfig,
    ) -> Result<ResponseStream, LlmError>;
}
```

**Agent Endpoint Configuration Files**

Agent definitions are stored in separate files in `config/agents/` directory:

**`config/agents/code_assistant.yaml`**
```yaml
name: "Code Assistant"
description: "AI agent that helps with code-related tasks"
endpoint: "/agents/code-assistant"
enabled: true

# LLM Configuration
model:
  provider: "anthropic"  # openai, anthropic, ollama, custom
  model: "claude-3-5-sonnet-20241022"
  api_key_env: "ANTHROPIC_API_KEY"
  temperature: 0.7
  max_tokens: 4000
  top_p: 1.0

# System Instructions
system_prompt: |
  You are a helpful code assistant with access to various development tools.
  Help users with coding tasks, debugging, and technical questions.
  Use the available tools when necessary to provide accurate information.

# Tool Access
tools:
  - "search_code"
  - "run_tests"
  - "format_code"
  - "explain_error"
  - "*"  # Or wildcard for all tools

# Resource Access
resources:
  - "file://**/*.{rs,py,js,ts}"
  - "git://commits"

# Prompt Access
prompts:
  - "code_review"
  - "refactoring_suggestions"

# Behavior Configuration
behavior:
  max_iterations: 10  # Maximum tool calling loops
  require_confirmation: false  # Ask user before tool execution
  streaming: true  # Enable streaming responses

# Context Configuration
context:
  max_messages: 20  # Maximum conversation history
  include_system: true

# Safety & Limits
limits:
  max_tool_calls_per_turn: 5
  max_execution_time_sec: 30
  cost_limit_usd: 1.0  # Per request

# Logging
logging:
  log_requests: true
  log_tool_calls: true
  log_responses: false  # May contain sensitive data
```

**`config/agents/data_analyst.json`**
```json
{
  "name": "Data Analyst",
  "description": "Agent specialized in data analysis and visualization",
  "endpoint": "/agents/data-analyst",
  "enabled": true,
  "model": {
    "provider": "openai",
    "model": "gpt-4-turbo",
    "api_key_env": "OPENAI_API_KEY",
    "temperature": 0.3,
    "max_tokens": 2000
  },
  "system_prompt": "You are a data analyst with expertise in SQL, statistics, and data visualization. Analyze data and provide insights.",
  "tools": [
    "execute_sql",
    "generate_chart",
    "statistical_analysis",
    "export_data"
  ],
  "resources": [
    "db://analytics/*"
  ],
  "behavior": {
    "max_iterations": 15,
    "streaming": false,
    "auto_visualize": true
  }
}
```

**Configuration Loader**

```rust
pub struct AgentConfigLoader {
    config_dir: PathBuf,
    watcher: Option<RecommendedWatcher>,
}

impl AgentConfigLoader {
    pub async fn load_all_agents(&self) -> Result<Vec<AgentConfig>, LoadError> {
        let mut agents = Vec::new();

        // Load from config/agents/ directory
        for entry in glob(&format!("{}/**/*.{yaml,yml,json}", self.config_dir.display()))? {
            let path = entry?;
            let config = self.load_agent_config(&path).await?;

            if config.enabled {
                agents.push(config);
            }
        }

        Ok(agents)
    }

    async fn load_agent_config(&self, path: &Path) -> Result<AgentConfig, LoadError> {
        let content = tokio::fs::read_to_string(path).await?;

        let config: AgentConfig = match path.extension().and_then(|s| s.to_str()) {
            Some("json") => serde_json::from_str(&content)?,
            Some("yaml") | Some("yml") => serde_yaml::from_str(&content)?,
            _ => return Err(LoadError::UnsupportedFormat),
        };

        config.validate()?;
        Ok(config)
    }

    pub async fn watch_for_changes(&mut self) -> Result<(), WatchError> {
        // Watch for file changes and reload configurations
        let (tx, rx) = mpsc::channel();

        let mut watcher = notify::recommended_watcher(tx)?;
        watcher.watch(&self.config_dir, RecursiveMode::Recursive)?;

        self.watcher = Some(watcher);
        Ok(())
    }
}
```

### MCP Tool Integration for Agents

**Automatic Tool Registration**

Each agent is automatically registered as an MCP tool, allowing agents to be invoked through the MCP protocol:

```rust
pub struct AgentToolRegistrar {
    tool_handler: Arc<ToolHandler>,
    agent_manager: Arc<AgentManager>,
}

impl AgentToolRegistrar {
    pub async fn register_agent_as_tool(&self, agent: &AgentConfig) -> Result<(), Error> {
        // Create MCP tool definition for single agent
        let tool_def = ToolDefinition {
            name: format!("agent_{}", agent.name.to_lowercase().replace(" ", "_")),
            description: agent.description.clone(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "message": {
                        "type": "string",
                        "description": "The message/task for the agent"
                    },
                    "context": {
                        "type": "object",
                        "description": "Additional context for the agent",
                        "properties": {
                            "session_id": { "type": "string" },
                            "user_id": { "type": "string" }
                        }
                    },
                    "streaming": {
                        "type": "boolean",
                        "description": "Enable streaming responses",
                        "default": false
                    }
                },
                "required": ["message"]
            }),
        };

        self.tool_handler.register_tool(tool_def, Box::new(AgentToolExecutor {
            agent: agent.clone(),
            executor: self.agent_manager.clone(),
        })).await?;

        Ok(())
    }
}

pub struct AgentToolExecutor {
    agent: AgentConfig,
    executor: Arc<AgentManager>,
}

#[async_trait]
impl ToolImplementation for AgentToolExecutor {
    async fn execute(&self, args: Value) -> Result<ToolResult, ToolError> {
        let message = args["message"].as_str()
            .ok_or(ToolError::InvalidInput("message required"))?;

        let context = args.get("context").cloned().unwrap_or(json!({}));
        let streaming = args.get("streaming").and_then(|v| v.as_bool()).unwrap_or(false);

        // Execute agent
        let response = self.executor.execute_agent(
            &self.agent.name,
            message,
            context,
            streaming,
        ).await?;

        Ok(ToolResult {
            content: vec![ToolContent::Text {
                text: serde_json::to_string_pretty(&response)?,
            }],
            is_error: false,
        })
    }
}
```

**MCP Tool Examples**

When `code_assistant.yaml` is loaded, it's automatically registered as:

```json
{
  "name": "agent_code_assistant",
  "description": "AI agent that helps with code-related tasks",
  "inputSchema": {
    "type": "object",
    "properties": {
      "message": {
        "type": "string",
        "description": "The message/task for the agent"
      },
      "context": {
        "type": "object"
      },
      "streaming": {
        "type": "boolean",
        "default": false
      }
    },
    "required": ["message"]
  }
}
```

**Calling Agents via MCP**

```json
// MCP tools/call request
{
  "method": "tools/call",
  "params": {
    "name": "agent_code_assistant",
    "arguments": {
      "message": "Review this Python function for potential bugs",
      "context": {
        "code": "def divide(a, b): return a / b",
        "session_id": "sess_123"
      }
    }
  }
}
```

**Response**
```json
{
  "content": [
    {
      "type": "text",
      "text": "{\n  \"response\": \"I've reviewed the function. Here are the issues...\",\n  \"tool_calls\": [...],\n  \"usage\": {...}\n}"
    }
  ]
}
```

**Multi-Agent MCP Tools**

Multi-agent orchestrators are also exposed as MCP tools:

```json
{
  "name": "multiagent_research_team",
  "description": "Multi-agent system for comprehensive research tasks",
  "inputSchema": {
    "type": "object",
    "properties": {
      "task": {
        "type": "object",
        "properties": {
          "type": { "type": "string" },
          "description": { "type": "string" },
          "requirements": {
            "type": "array",
            "items": { "type": "string" }
          }
        },
        "required": ["description"]
      },
      "config": {
        "type": "object",
        "properties": {
          "max_duration_sec": { "type": "integer" },
          "output_format": { "type": "string" }
        }
      }
    },
    "required": ["task"]
  }
}
```

**Configuration for MCP Exposure**

```yaml
# config/agents/code_assistant.yaml
name: "Code Assistant"
description: "AI agent that helps with code-related tasks"
enabled: true

# MCP Integration
mcp:
  expose_as_tool: true  # Expose this agent as an MCP tool
  tool_name: "agent_code_assistant"  # Custom tool name (optional)
  tool_category: "agents"  # Tool categorization

# Also expose via REST if needed
rest:
  enabled: true
  endpoint: "/agents/code-assistant"
```

**Listing Agent Tools**

Agents appear in the MCP `tools/list` response:

```json
{
  "tools": [
    // Regular tools
    {
      "name": "search_code",
      "description": "Search through codebase"
    },
    // Agent tools
    {
      "name": "agent_code_assistant",
      "description": "AI agent that helps with code-related tasks",
      "category": "agents"
    },
    {
      "name": "agent_data_analyst",
      "description": "Agent specialized in data analysis",
      "category": "agents"
    },
    // Multi-agent tools
    {
      "name": "multiagent_research_team",
      "description": "Multi-agent system for research",
      "category": "multi_agents"
    }
  ]
}
```

### HTTP/REST API (Optional)

For compatibility and ease of testing, agents can also be exposed via REST endpoints:

**Single Agent REST API**

```
POST /agents/{agent-name}
Content-Type: application/json

{
  "message": "Can you analyze the sales data from last quarter?",
  "context": {
    "user_id": "user123",
    "session_id": "session456"
  },
  "stream": false
}
```

**Response**
```json
{
  "agent": "data-analyst",
  "response": {
    "content": "I'll analyze the Q4 sales data for you...",
    "tool_calls": [
      {
        "tool": "execute_sql",
        "arguments": {
          "query": "SELECT * FROM sales WHERE quarter = 'Q4'"
        },
        "result": { "rows": [...] }
      },
      {
        "tool": "generate_chart",
        "arguments": {
          "type": "line",
          "data": [...]
        },
        "result": { "chart_url": "..." }
      }
    ],
    "finish_reason": "stop"
  },
  "usage": {
    "input_tokens": 150,
    "output_tokens": 420,
    "tool_calls": 2,
    "cost_usd": 0.025
  },
  "metadata": {
    "request_id": "req_abc123",
    "duration_ms": 2500,
    "model": "gpt-4-turbo"
  }
}
```

**Streaming Response**

```
POST /agents/code-assistant?stream=true

# Server-Sent Events (SSE)
data: {"type":"content","delta":"I'll help you"}
data: {"type":"content","delta":" with that"}
data: {"type":"tool_call","tool":"search_code","args":{...}}
data: {"type":"tool_result","tool":"search_code","result":{...}}
data: {"type":"content","delta":"Based on the search..."}
data: {"type":"done","finish_reason":"stop"}
```

### Multi-Agent Orchestration

**Concept**

Multi-agent endpoints coordinate multiple specialized agents working together to solve complex tasks. Agents can communicate, delegate tasks, and combine their capabilities.

**Orchestration Patterns**

1. **Sequential**: Agents execute one after another in a pipeline
2. **Parallel**: Multiple agents work simultaneously
3. **Hierarchical**: Supervisor agent delegates to worker agents
4. **Collaborative**: Agents discuss and reach consensus
5. **Competitive**: Multiple agents provide solutions, best one selected

**Multi-Agent Configuration**

**`config/multi_agents/research_team.yaml`**
```yaml
name: "Research Team"
description: "Multi-agent system for comprehensive research tasks"
endpoint: "/multi-agents/research-team"
enabled: true

# Orchestration Configuration
orchestration:
  pattern: "hierarchical"  # sequential, parallel, hierarchical, collaborative
  coordinator: "supervisor"
  max_rounds: 5
  consensus_threshold: 0.7  # For collaborative mode

# Agent Definitions
agents:
  - name: "supervisor"
    role: "coordinator"
    agent_config: "config/agents/supervisor.yaml"

  - name: "web_researcher"
    role: "worker"
    agent_config: "config/agents/web_researcher.yaml"
    capabilities: ["web_search", "content_extraction"]

  - name: "data_analyst"
    role: "worker"
    agent_config: "config/agents/data_analyst.yaml"
    capabilities: ["data_analysis", "visualization"]

  - name: "writer"
    role: "worker"
    agent_config: "config/agents/writer.yaml"
    capabilities: ["summarization", "report_generation"]

# Communication
communication:
  protocol: "message_passing"  # message_passing, shared_memory, blackboard
  message_queue: "redis"  # memory, redis
  broadcast: false

# Workflow Definition
workflow:
  - step: "initial_research"
    agent: "web_researcher"
    tools: ["web_search", "scrape_content"]

  - step: "data_analysis"
    agent: "data_analyst"
    depends_on: ["initial_research"]
    tools: ["analyze_data", "generate_charts"]

  - step: "report_generation"
    agent: "writer"
    depends_on: ["initial_research", "data_analysis"]
    tools: ["create_document", "format_report"]

  - step: "review"
    agent: "supervisor"
    depends_on: ["report_generation"]
    action: "evaluate_and_approve"

# Coordination Rules
coordination:
  # When to escalate to supervisor
  escalation_rules:
    - condition: "agent_stuck"
      threshold: 3  # After 3 failed attempts
    - condition: "cost_exceeded"
      threshold_usd: 5.0
    - condition: "time_exceeded"
      threshold_sec: 300

  # Inter-agent communication rules
  communication_rules:
    - from: "web_researcher"
      to: "data_analyst"
      filter: "data_only"
    - from: "*"
      to: "supervisor"
      filter: "status_updates"

# State Management
state:
  persistence: "redis"  # memory, redis, database
  shared_context: true
  context_keys:
    - "research_topic"
    - "findings"
    - "intermediate_results"

# Performance
performance:
  parallel_execution: true
  max_concurrent_agents: 3
  timeout_per_agent_sec: 60
```

**Multi-Agent Architecture**

```rust
pub struct MultiAgentOrchestrator {
    config: MultiAgentConfig,
    agents: HashMap<String, AgentEndpoint>,
    coordinator: Box<dyn CoordinationStrategy>,
    message_bus: Arc<MessageBus>,
    state_manager: Arc<SharedStateManager>,
}

#[async_trait]
pub trait CoordinationStrategy: Send + Sync {
    async fn execute(
        &self,
        task: &Task,
        agents: &HashMap<String, AgentEndpoint>,
        context: &ExecutionContext,
    ) -> Result<MultiAgentResponse, OrchestrationError>;
}

pub struct HierarchicalCoordinator {
    supervisor: String,
    workers: Vec<String>,
}

#[async_trait]
impl CoordinationStrategy for HierarchicalCoordinator {
    async fn execute(
        &self,
        task: &Task,
        agents: &HashMap<String, AgentEndpoint>,
        context: &ExecutionContext,
    ) -> Result<MultiAgentResponse, OrchestrationError> {
        // 1. Supervisor analyzes task and creates plan
        let supervisor = agents.get(&self.supervisor).unwrap();
        let plan = supervisor.plan_task(task).await?;

        // 2. Delegate subtasks to workers
        let mut results = Vec::new();
        for subtask in plan.subtasks {
            let worker = agents.get(&subtask.agent).unwrap();
            let result = worker.execute(&subtask).await?;
            results.push(result);
        }

        // 3. Supervisor synthesizes results
        let final_result = supervisor.synthesize(results).await?;

        Ok(final_result)
    }
}

pub struct CollaborativeCoordinator {
    agents: Vec<String>,
    consensus_threshold: f32,
}

#[async_trait]
impl CoordinationStrategy for CollaborativeCoordinator {
    async fn execute(
        &self,
        task: &Task,
        agents: &HashMap<String, AgentEndpoint>,
        context: &ExecutionContext,
    ) -> Result<MultiAgentResponse, OrchestrationError> {
        let mut round = 0;
        let mut proposals = Vec::new();

        loop {
            round += 1;

            // Each agent proposes a solution
            for agent_name in &self.agents {
                let agent = agents.get(agent_name).unwrap();
                let proposal = agent.propose_solution(task, &proposals).await?;
                proposals.push(proposal);
            }

            // Check for consensus
            if let Some(consensus) = self.check_consensus(&proposals) {
                return Ok(consensus);
            }

            if round >= context.max_rounds {
                // Force consensus or vote
                return self.force_consensus(proposals);
            }

            // Agents discuss and refine
            for agent_name in &self.agents {
                let agent = agents.get(agent_name).unwrap();
                agent.receive_feedback(&proposals).await?;
            }
        }
    }
}
```

**Multi-Agent REST API**

```
POST /multi-agents/research-team
Content-Type: application/json

{
  "task": {
    "type": "research",
    "description": "Research the impact of AI on healthcare",
    "requirements": [
      "Find recent studies",
      "Analyze trends",
      "Generate comprehensive report"
    ]
  },
  "config": {
    "max_duration_sec": 300,
    "output_format": "markdown",
    "include_sources": true
  }
}
```

**Response**
```json
{
  "orchestrator": "research-team",
  "execution_trace": [
    {
      "step": 1,
      "agent": "supervisor",
      "action": "plan_created",
      "duration_ms": 500,
      "output": {
        "plan": {
          "subtasks": [
            {"agent": "web_researcher", "task": "Search for studies"},
            {"agent": "data_analyst", "task": "Analyze findings"},
            {"agent": "writer", "task": "Generate report"}
          ]
        }
      }
    },
    {
      "step": 2,
      "agent": "web_researcher",
      "action": "research_completed",
      "duration_ms": 5000,
      "tool_calls": [
        {"tool": "web_search", "query": "AI healthcare impact 2024"},
        {"tool": "scrape_content", "urls": ["..."]}
      ],
      "output": {
        "findings": ["..."],
        "sources": ["..."]
      }
    },
    {
      "step": 3,
      "agent": "data_analyst",
      "action": "analysis_completed",
      "duration_ms": 3000,
      "tool_calls": [
        {"tool": "analyze_trends", "data": "..."}
      ],
      "output": {
        "trends": ["..."],
        "insights": ["..."]
      }
    },
    {
      "step": 4,
      "agent": "writer",
      "action": "report_generated",
      "duration_ms": 4000,
      "output": {
        "report": "# AI Impact on Healthcare\n\n..."
      }
    },
    {
      "step": 5,
      "agent": "supervisor",
      "action": "review_approved",
      "duration_ms": 1000,
      "output": {
        "status": "approved",
        "quality_score": 0.92
      }
    }
  ],
  "final_result": {
    "report": "# AI Impact on Healthcare\n\n...",
    "metadata": {
      "sources": 15,
      "agents_involved": 4,
      "total_tool_calls": 8
    }
  },
  "metrics": {
    "total_duration_ms": 13500,
    "total_cost_usd": 0.45,
    "agents_used": ["supervisor", "web_researcher", "data_analyst", "writer"],
    "success": true
  }
}
```

### Tool Calling Integration

**Tool Executor**

```rust
pub struct ToolExecutor {
    tool_handler: Arc<ToolHandler>,
    authorization: Arc<AuthManager>,
    rate_limiter: Arc<RateLimiter>,
}

impl ToolExecutor {
    pub async fn execute_tool(
        &self,
        agent_id: &str,
        tool_name: &str,
        arguments: Value,
        context: &ExecutionContext,
    ) -> Result<ToolResult, ToolError> {
        // 1. Check authorization
        self.authorization.check_tool_access(agent_id, tool_name).await?;

        // 2. Rate limiting
        self.rate_limiter.check_and_increment(agent_id).await?;

        // 3. Log tool call
        tracing::info!(
            agent = agent_id,
            tool = tool_name,
            "Tool execution started"
        );

        // 4. Execute tool via MCP handler
        let result = self.tool_handler
            .call_tool(tool_name, arguments)
            .await?;

        // 5. Record metrics
        self.record_tool_execution(agent_id, tool_name, &result).await;

        Ok(result)
    }

    pub async fn execute_parallel_tools(
        &self,
        agent_id: &str,
        tool_calls: Vec<ToolCall>,
        context: &ExecutionContext,
    ) -> Result<Vec<ToolResult>, ToolError> {
        let futures: Vec<_> = tool_calls
            .into_iter()
            .map(|call| {
                self.execute_tool(agent_id, &call.name, call.arguments, context)
            })
            .collect();

        let results = futures::future::join_all(futures).await;

        results.into_iter().collect()
    }
}
```

**Tool Call Strategies**

```yaml
# config/agents/advanced_agent.yaml
tool_calling:
  strategy: "auto"  # auto, manual, none

  # Automatic tool selection
  auto_selection:
    enabled: true
    max_attempts: 3
    retry_on_failure: true

  # Parallel execution
  parallel:
    enabled: true
    max_concurrent: 3

  # Confirmation required
  confirmation:
    always: false
    for_tools:
      - "delete_*"
      - "execute_command"

  # Error handling
  error_handling:
    retry_strategy: "exponential_backoff"
    max_retries: 3
    fallback_action: "ask_user"
```

### Agent State Management

**State Persistence**

```rust
pub struct AgentStateManager {
    store: Arc<dyn StateStore>,
    cache: Arc<Mutex<LruCache<String, AgentState>>>,
}

#[async_trait]
pub trait StateStore: Send + Sync {
    async fn save(&self, agent_id: &str, state: &AgentState) -> Result<(), StateError>;
    async fn load(&self, agent_id: &str) -> Result<Option<AgentState>, StateError>;
    async fn delete(&self, agent_id: &str) -> Result<(), StateError>;
}

pub struct AgentState {
    conversation_history: Vec<Message>,
    tool_execution_history: Vec<ToolExecution>,
    custom_state: HashMap<String, Value>,
    metadata: StateMetadata,
}
```

**Session Management**

```yaml
# Agent session configuration
session:
  # Session storage
  storage: "redis"  # memory, redis, postgres
  ttl_seconds: 3600

  # Session ID generation
  id_strategy: "uuid"  # uuid, custom

  # Context preservation
  preserve_context:
    enabled: true
    max_messages: 50
    include_tool_results: true

  # Session resumption
  resumable: true
  auto_cleanup: true
```

### Configuration Management

**Main Configuration**

```toml
# metis.toml

[agents]
enabled = true
config_dir = "config/agents"
hot_reload = true

# Agent endpoint prefix
base_path = "/agents"

# Global agent defaults
[agents.defaults]
max_iterations = 10
request_timeout_sec = 60
streaming = true

[agents.defaults.limits]
max_tokens = 4000
max_tool_calls = 10
max_cost_usd = 2.0

# Multi-agent orchestration
[multi_agents]
enabled = true
config_dir = "config/multi_agents"
hot_reload = true
base_path = "/multi-agents"

[multi_agents.defaults]
max_agents = 5
max_rounds = 10
total_timeout_sec = 300

# Agent monitoring
[agents.monitoring]
log_all_requests = true
log_tool_calls = true
track_costs = true
alert_on_errors = true

[agents.monitoring.metrics]
enabled = true
include_latency = true
include_token_usage = true
include_tool_execution = true
```

### API Endpoints Reference

**Single Agent Endpoints**

```
# Execute agent task
POST /agents/{agent-name}
POST /agents/{agent-name}?stream=true

# Get agent information
GET /agents/{agent-name}/info

# List all available agents
GET /agents

# Get agent conversation history
GET /agents/{agent-name}/history/{session-id}

# Clear agent session
DELETE /agents/{agent-name}/sessions/{session-id}
```

**Multi-Agent Endpoints**

```
# Execute multi-agent task
POST /multi-agents/{orchestrator-name}
POST /multi-agents/{orchestrator-name}?stream=true

# Get orchestrator information
GET /multi-agents/{orchestrator-name}/info

# List all orchestrators
GET /multi-agents

# Get execution trace
GET /multi-agents/executions/{execution-id}

# Cancel running execution
DELETE /multi-agents/executions/{execution-id}
```

**Admin Endpoints**

```
# Reload agent configurations
POST /admin/agents/reload

# Get agent metrics
GET /admin/agents/metrics

# Enable/disable agent
PUT /admin/agents/{agent-name}/status
```

### Security & Access Control

**Agent Authorization**

```yaml
# config/agents/restricted_agent.yaml
security:
  # Authentication required
  authentication:
    required: true
    methods: ["api_key", "jwt"]

  # Authorization
  authorization:
    type: "rbac"  # rbac, acl, custom
    roles:
      - "admin"
      - "developer"

  # Tool access control
  tool_permissions:
    allow_all: false
    allowed_tools:
      - "search_*"
      - "analyze_*"
    denied_tools:
      - "delete_*"
      - "execute_command"

  # Rate limiting
  rate_limit:
    requests_per_minute: 60
    requests_per_hour: 500

  # Input validation
  input_validation:
    max_message_length: 10000
    sanitize_html: true
    block_sql_injection: true
```

### Monitoring & Observability

**Agent Metrics**

```rust
pub struct AgentMetrics {
    // Request metrics
    requests_total: CounterVec,
    request_duration: HistogramVec,

    // Agent-specific metrics
    agent_iterations: HistogramVec,
    tool_calls_per_request: HistogramVec,

    // Cost tracking
    llm_cost_total: CounterVec,
    tokens_used: CounterVec,

    // Success/failure
    requests_successful: CounterVec,
    requests_failed: CounterVec,

    // Tool execution
    tool_executions: CounterVec,
    tool_execution_duration: HistogramVec,
}
```

**Logging**

```yaml
agents:
  logging:
    # What to log
    log_requests: true
    log_responses: false  # May contain sensitive data
    log_tool_calls: true
    log_tool_results: true
    log_errors: true

    # Log levels per component
    levels:
      agent_executor: "info"
      tool_executor: "debug"
      orchestrator: "info"

    # Structured logging fields
    include_fields:
      - agent_name
      - session_id
      - user_id
      - request_id
      - duration_ms
      - token_count
      - cost_usd
```

### Example: Complete Agent Setup

**Directory Structure**
```
config/
├── metis.toml
├── agents/
│   ├── code_assistant.yaml
│   ├── data_analyst.yaml
│   ├── researcher.yaml
│   └── customer_support.yaml
├── multi_agents/
│   ├── research_team.yaml
│   ├── dev_team.yaml
│   └── support_team.yaml
└── tools/
    ├── search.toml
    ├── database.toml
    └── code_execution.toml
```

**Running Metis with Agents**

```bash
# Start server with agent endpoints
metis --config metis.toml

# Server logs
[INFO] Loading agent configurations from config/agents/
[INFO] Loaded agent: code_assistant (endpoint: /agents/code-assistant)
[INFO] Loaded agent: data_analyst (endpoint: /agents/data-analyst)
[INFO] Loaded multi-agent: research_team (endpoint: /multi-agents/research-team)
[INFO] Agent endpoints ready
[INFO] Server listening on http://localhost:3000
```

**Using the Agent**

```bash
# Call single agent
curl -X POST http://localhost:3000/agents/code-assistant \
  -H "Content-Type: application/json" \
  -d '{
    "message": "Review this Python function for potential bugs",
    "context": {
      "code": "def divide(a, b): return a / b"
    }
  }'

# Call multi-agent orchestrator
curl -X POST http://localhost:3000/multi-agents/research-team \
  -H "Content-Type: application/json" \
  -d '{
    "task": {
      "type": "research",
      "description": "Research Rust async patterns"
    }
  }'

# Streaming response
curl -N http://localhost:3000/agents/code-assistant?stream=true \
  -H "Content-Type: application/json" \
  -d '{"message": "Explain async/await in Rust"}'
```

---

## Deployment & Operations

### Installation Methods

**1. Binary Releases**
```bash
# Linux/macOS
curl -sSL https://install.metis.dev | sh

# Or download directly
wget https://github.com/metis/metis/releases/download/v1.0.0/metis-linux-amd64
chmod +x metis-linux-amd64
./metis-linux-amd64 --config metis.toml
```

**2. Cargo Install**
```bash
cargo install metis-server
metis --config metis.toml
```

**3. Docker**
```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/metis /usr/local/bin/
COPY --from=builder /app/config /etc/metis/config
ENTRYPOINT ["metis"]
CMD ["--config", "/etc/metis/metis.toml"]
```

```bash
# Run with Docker
docker run -v $(pwd)/config:/config metis/metis --config /config/metis.toml

# Docker Compose
docker-compose up -d
```

**4. Kubernetes**
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: metis
spec:
  replicas: 3
  selector:
    matchLabels:
      app: metis
  template:
    metadata:
      labels:
        app: metis
    spec:
      containers:
      - name: metis
        image: metis/metis:v1.0.0
        ports:
        - containerPort: 3000
          name: http
        - containerPort: 9090
          name: metrics
        volumeMounts:
        - name: config
          mountPath: /etc/metis
        env:
        - name: RUST_LOG
          value: "info"
        resources:
          requests:
            memory: "128Mi"
            cpu: "100m"
          limits:
            memory: "512Mi"
            cpu: "1000m"
      volumes:
      - name: config
        configMap:
          name: metis-config
```

### Configuration Management

**Environment-Specific Configs**
```bash
config/
├── metis.base.toml          # Base configuration
├── metis.development.toml   # Development overrides
├── metis.staging.toml       # Staging overrides
├── metis.production.toml    # Production overrides
└── environments/
    ├── dev/
    ├── staging/
    └── production/
```

**Configuration Merging**
```bash
# Merge base + environment config
metis --config metis.base.toml --config metis.production.toml

# Or use environment variable
METIS_ENV=production metis
```

### Monitoring & Alerting

**Prometheus Metrics**
```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'metis'
    static_configs:
      - targets: ['localhost:9090']
    metrics_path: '/metrics'
```

**Key Metrics to Monitor**
- `metis_requests_total` - Total requests
- `metis_request_duration_seconds` - Request latency histogram
- `metis_errors_total` - Error count by type
- `metis_strategy_executions_total` - Executions per strategy
- `metis_llm_cost_total` - LLM API costs
- `metis_cache_hit_rate` - Cache efficiency

**Alerting Rules**
```yaml
groups:
  - name: metis
    rules:
      - alert: HighErrorRate
        expr: rate(metis_errors_total[5m]) > 0.05
        for: 5m
        annotations:
          summary: "High error rate detected"

      - alert: HighLatency
        expr: histogram_quantile(0.99, metis_request_duration_seconds) > 1
        for: 5m
        annotations:
          summary: "p99 latency > 1s"
```

### Logging

**Log Levels**
- `TRACE` - Very verbose, all operations
- `DEBUG` - Debug information
- `INFO` - Normal operations (default)
- `WARN` - Warnings, degraded performance
- `ERROR` - Errors, failures

**Structured Logging**
```rust
tracing::info!(
    request_id = %context.request_id,
    strategy = %strategy_name,
    duration_ms = execution_time.as_millis(),
    "Request completed successfully"
);
```

**Log Aggregation**
- Output JSON logs for parsing
- Integration with ELK stack, Loki, Datadog
- Log sampling for high-volume environments

### Health Checks

**Endpoints**
- `GET /health` - Basic health check
- `GET /health/ready` - Readiness check (Kubernetes)
- `GET /health/live` - Liveness check (Kubernetes)

**Response Format**
```json
{
  "status": "healthy",
  "version": "1.0.0",
  "uptime_seconds": 3600,
  "checks": {
    "config": "ok",
    "cache": "ok",
    "database": "ok",
    "llm_provider": "ok"
  }
}
```

### Backup & Recovery

**Configuration Backup**
- Version control for configuration files
- Automated backups before live reload
- Configuration history tracking

**State Management**
- Stateless design (no persistent state by default)
- Optional state persistence for script strategies
- State export/import for migration

### Scaling

**Horizontal Scaling**
- Stateless design allows multiple instances
- Load balancer for HTTP transport
- Shared Redis cache across instances
- Database connection pooling

**Vertical Scaling**
- Increase worker threads
- Increase memory limits
- Larger cache sizes

**Scaling Strategy**
```
Single Instance (Development)
    ↓
Multi-Instance + Load Balancer (Staging)
    ↓
Multi-Region + Redis Cache + DB Pool (Production)
```

---

## API Reference

### Command-Line Interface

```bash
metis [OPTIONS]

OPTIONS:
    -c, --config <FILE>           Configuration file path(s) (can be specified multiple times)
    -H, --host <HOST>             Override server host
    -p, --port <PORT>             Override server port
    -l, --log-level <LEVEL>       Log level (trace, debug, info, warn, error)
    -t, --transport <TYPE>        Transport type (stdio, http, websocket)
    -v, --validate                Validate configuration and exit
    -d, --dev                     Enable development mode
    -h, --help                    Print help information
    -V, --version                 Print version information

EXAMPLES:
    # Start with default config
    metis --config metis.toml

    # Multiple config files (merged)
    metis -c base.toml -c prod.toml

    # Validate configuration
    metis --config metis.toml --validate

    # Development mode with hot reload
    metis --config metis.toml --dev

    # Specify transport
    metis --config metis.toml --transport http --port 3000
```

### Admin API

**Management Endpoints** (when `development.debug_endpoints = true`)

```
GET /admin/config          - Get current configuration
POST /admin/config/reload  - Trigger configuration reload
GET /admin/stats           - Get server statistics
GET /admin/cache/stats     - Get cache statistics
DELETE /admin/cache/clear  - Clear cache
GET /admin/strategies      - List all strategies
GET /admin/logs            - Stream logs (SSE)
```

### MCP Protocol Endpoints

Implements standard MCP endpoints:
- `initialize` - Initialize connection
- `resources/list` - List resources
- `resources/read` - Read resource
- `resources/subscribe` - Subscribe to updates (if supported)
- `tools/list` - List tools
- `tools/call` - Execute tool
- `prompts/list` - List prompts
- `prompts/get` - Get prompt

---

## Security Considerations

### 1. Configuration Security

**Best Practices**
- Never commit secrets to version control
- Use environment variables for sensitive data
- Encrypt configuration files at rest
- Restrict file permissions (0600)

**Secrets Management**
```toml
# Bad - hardcoded secret
[auth.api_key]
valid_keys = ["secret123"]

# Good - environment variable
[auth.api_key]
keys_env = "METIS_API_KEYS"  # Comma-separated

# Good - external file with restricted permissions
[auth.api_key]
keys_file = "/etc/metis/secrets/api_keys.txt"
```

### 2. Network Security

**TLS/HTTPS**
```toml
[server.tls]
enabled = true
cert_file = "/etc/metis/tls/cert.pem"
key_file = "/etc/metis/tls/key.pem"
# Optional: mTLS
client_ca_file = "/etc/metis/tls/ca.pem"
```

**Rate Limiting**
```toml
[security.rate_limit]
enabled = true
requests_per_second = 100
burst = 200
# Per-IP or per-auth-token
strategy = "per_ip"
```

### 3. Input Validation

**Request Validation**
- Schema validation for all inputs
- Size limits on requests
- Content-type validation
- Sanitization of user inputs in templates

**Script Sandbox**
- Restricted script execution environment
- No file system access from scripts (unless explicitly allowed)
- CPU time limits
- Memory limits

### 4. Audit Logging

**Security Events**
```rust
tracing::warn!(
    auth_mode = %auth_mode,
    ip = %client_ip,
    "Authentication failed"
);
```

**Audit Trail**
- Authentication attempts (success/failure)
- Configuration changes
- Admin API access
- Suspicious patterns

### 5. Dependency Security

**Supply Chain Security**
- Regular `cargo audit` runs
- Pinned dependency versions
- Review of new dependencies
- Minimal dependency tree

---

## Future Enhancements

### Phase 2 Features (Post-1.0)

**Advanced Mocking**
- Stateful mock sequences with state machines
- Advanced scenario branching (conditional flows)
- Record/replay functionality for real traffic
- GraphQL schema and resolver mocking
- gRPC service mocking
- WebSocket mocking
- SSE (Server-Sent Events) mocking
- Streaming response mocking

**Enhanced LLM Integration**
- Local LLM support (llama.cpp, GGUF)
- Multi-provider fallback
- Response quality scoring
- Fine-tuned models for specific domains

**Collaboration Features**
- Multi-user configuration management
- Shared mock servers
- Team workspaces
- Cloud-hosted option

**Advanced Testing Tools**
- Visual test report dashboard
- AI-powered test generation
- Mutation testing for configuration
- Chaos engineering features (network faults, latency injection)
- Performance regression detection
- Load testing recorder/replayer
- Test impact analysis
- Distributed testing across multiple servers
- Real-time test execution monitoring
- Test coverage heatmaps

**Enterprise Features**
- SSO/SAML authentication
- Advanced RBAC
- Audit compliance reports
- SLA monitoring

### Plugin System

**Extensibility**
```rust
pub trait MetisPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;

    fn on_initialize(&mut self, server: &MetisServer) -> Result<(), PluginError>;
    fn on_request(&self, context: &RequestContext) -> Result<(), PluginError>;
    fn on_response(&self, response: &mut Response) -> Result<(), PluginError>;
}
```

**Plugin Examples**
- Custom authentication providers
- Custom mock strategies
- Response transformers
- Logging plugins
- Metrics plugins

---

## Appendix

### A. MCP Protocol Overview

Model Context Protocol (MCP) is a standardized protocol for communication between AI assistants and external tools/data sources.

**Key Concepts**
- **Resources**: Data sources (files, databases, APIs)
- **Tools**: Executable functions
- **Prompts**: Reusable prompt templates
- **Transport**: stdio, HTTP, WebSocket

**Protocol Flow**
```
Client                    Server
  |                         |
  |---- initialize -------->|
  |<--- initialized --------|
  |                         |
  |---- resources/list ---->|
  |<--- resources list -----|
  |                         |
  |---- tools/call -------->|
  |<--- tool result --------|
```

### B. Example Configurations

**Example 1: Development Server**
```toml
[server]
name = "Dev Mock Server"
host = "127.0.0.1"
port = 3000

[auth]
enabled = false

[[resources]]
uri = "dev://test"
name = "Test Resource"
[resources.mock]
strategy = "static"
content = { message = "Hello, World!" }

[[tools]]
name = "test_tool"
[tools.mock]
strategy = "echo"
```

**Example 2: Integration Testing Server**
```toml
[server]
name = "Integration Test Server"

[auth]
enabled = true
mode = "api_key"
[auth.api_key]
valid_keys = ["test-key-1", "test-key-2"]

[[resources]]
uri = "db://users"
[resources.mock]
strategy = "database"
[resources.mock.database]
driver = "sqlite"
connection_string = "test.db"
[resources.mock.database.query]
sql = "SELECT * FROM users"

[defaults.behavior]
latency_ms = 10
error_rate = 0.01
```

**Example 3: Load Testing Server**
```toml
[server]
name = "Load Test Server"

[server.performance]
max_concurrent_requests = 10000
worker_threads = 8

[auth]
enabled = false

[[tools]]
name = "high_volume_tool"
[tools.mock]
strategy = "random"
[tools.mock.random]
type = "object"
schema = { id = { type = "uuid" }, timestamp = { type = "timestamp" } }

[defaults.behavior]
latency_ms = 1  # Minimal latency
error_rate = 0.0

[cache]
enabled = true
max_size_mb = 1000
```

**Example 4: Automatic Testing Configuration**
```toml
[server]
name = "Test Server with Auto-Testing"

[auth]
enabled = false

# Enable automatic test generation
[testing.auto_generate]
enabled = true
output_dir = "tests/generated"

[testing.auto_generate.resources]
enabled = true
test_read = true
test_list = true

[testing.auto_generate.tools]
enabled = true
test_all_inputs = true
test_edge_cases = true

# Protocol compliance testing
[testing.compliance]
enabled = true
strict_mode = true

# Contract testing
[testing.contracts]
enabled = true
contracts_dir = "tests/contracts"

# Snapshot testing
[testing.snapshots]
enabled = true
snapshot_dir = "tests/snapshots"
update_mode = "review"

[testing.snapshots.comparison]
ignore_timestamps = true
ignore_random_ids = true

# Multi-client testing
[testing.clients]
enabled = true

[[testing.clients.implementations]]
name = "typescript"
type = "npm"
package = "@modelcontextprotocol/sdk"
command = "node tests/run_typescript_client.js"

[[testing.clients.implementations]]
name = "python"
type = "pip"
package = "mcp"
command = "python tests/run_python_client.py"

# Test reporting
[testing.reporting]
enabled = true
format = "html"
output_dir = "test_reports"

# CI integration
[testing.ci]
enabled = true
fail_fast = false
parallel = true

[testing.ci.coverage]
minimum_percent = 80
fail_below_minimum = true

# Define a sample resource for testing
[[resources]]
uri = "test://sample"
name = "Sample Resource"
[resources.mock]
strategy = "static"
content = { message = "Test data" }

# Define a sample tool for testing
[[tools]]
name = "sample_tool"
description = "Sample tool for testing"
[tools.input_schema]
type = "object"
properties = { input = { type = "string" } }
[tools.mock]
strategy = "echo"
```

### C. Performance Tuning Guide

**CPU-Bound Workloads**
```toml
[server.performance]
worker_threads = 16  # Match CPU cores
request_timeout_sec = 60
```

**Memory-Intensive Workloads**
```toml
[cache]
max_size_mb = 2000
[server.performance]
max_concurrent_requests = 500
```

**High Throughput**
```toml
[server.performance]
worker_threads = 8
max_concurrent_requests = 5000

[cache]
enabled = true
backend = "redis"

[defaults.behavior]
latency_ms = 0
```

### D. Migration from Other Mock Servers

**From Mockoon**
- Configuration converter tool
- Import Mockoon JSON configs
- Map routes to MCP tools/resources

**From WireMock**
- Mapping file converter
- Support for WireMock stubbing format
- Migration guide

**From Postman Mock Server**
- Collection importer
- Convert Postman examples to mock strategies

### E. Contributing Guide

**Development Setup**
```bash
# Clone repository
git clone https://github.com/metis/metis
cd metis

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.org | sh

# Build
cargo build

# Run tests
cargo test

# Run with example config
cargo run -- --config examples/basic.toml

# Development mode with auto-reload
cargo watch -x 'run -- --config examples/basic.toml'
```

**Code Style**
```bash
# Format code
cargo fmt

# Lint
cargo clippy -- -D warnings

# Check for common mistakes
cargo check
```

**Pull Request Process**
1. Fork the repository
2. Create a feature branch
3. Make changes with tests
4. Run full test suite
5. Update documentation
6. Submit PR with description

### F. Glossary

- **MCP**: Model Context Protocol
- **Mock Strategy**: Method for generating mock responses
- **Resource**: Data source in MCP (files, databases, etc.)
- **Tool**: Executable function in MCP
- **Prompt**: Reusable prompt template in MCP
- **Transport**: Communication method (stdio, HTTP, WebSocket)
- **Live Reload**: Automatic configuration reloading on file change
- **LLM**: Large Language Model
- **TTL**: Time To Live (cache expiration)
- **Strategy**: Pattern for generating mock data

### G. References

**MCP Protocol**
- [MCP Specification](https://spec.modelcontextprotocol.io/)
- [MCP TypeScript SDK](https://github.com/modelcontextprotocol/typescript-sdk)
- [MCP Python SDK](https://github.com/modelcontextprotocol/python-sdk)

**Rust Resources**
- [Rust Official Documentation](https://doc.rust-lang.org/)
- [Tokio Documentation](https://tokio.rs/)
- [Serde Documentation](https://serde.rs/)

**Tools & Libraries**
- [SQLx](https://github.com/launchbadge/sqlx)
- [Tera Template Engine](https://tera.netlify.app/)
- [Rhai Scripting](https://rhai.rs/)

---

## Summary

Metis will be a comprehensive, high-performance MCP mock server that provides developers with a powerful tool for testing, development, and integration. Key differentiators:

1. **Flexibility**: Multiple mock strategies covering all use cases
2. **Performance**: Built in Rust for speed and efficiency
3. **Configuration**: Declarative, modular, with live reload
4. **Automatic Testing**: Built-in test client with auto-generation, protocol compliance, and multi-client support
5. **Model System**: Odoo-style model definitions with relationships (one2many, many2one, one2one, many2many)
6. **Multi-Language Scripting**: Support for Python, Lua, Ruby, Rhai, and JavaScript
7. **Agent Orchestration**: Single and multi-agent endpoints exposed via MCP tools
8. **Extensibility**: Plugin system and custom script support
9. **Production-Ready**: Authentication, monitoring, security built-in

**Success Metrics**
- GitHub stars: 1000+ in first year
- Active users: 500+ organizations
- Performance: >10k req/s sustained
- Test coverage: >80%
- Documentation: Complete with tutorials

**Timeline**: 36 weeks (9 months) from start to v1.3 release

**Phased Release Strategy**:
- **v1.0 (Week 17)**: Core platform - MCP protocol, mock strategies, authentication, configuration, observability
- **v1.1 (Week 21)**: Workflow engine and agent orchestration
- **v1.2 (Week 25)**: Multi-language scripting support
- **v1.3 (Week 31)**: Web UI (Leptos)
- **v1.4 (Week 35)**: Comprehensive testing and documentation complete

**Critical Success Factors**:
- ✅ Week 0 technology validation completed successfully
- ✅ Checkpoint reviews at Week 11 and Week 17
- ✅ Incremental releases with user feedback loops
- ✅ Security audits before each major release
- ✅ Continuous testing (>85% coverage throughout)

---

*Metis Implementation Plan v2.0 - Revised with Realistic Timeline & Risk Mitigation*
*Last Updated: 2025-11-19*
*Changes: Added Week 0 validation, security hardening, cost controls, checkpoint gates, split Phase 7, updated to 36-week timeline*
