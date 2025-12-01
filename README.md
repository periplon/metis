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
│   │   ├── api_handler.rs      # REST API endpoints
│   │   ├── resource_handler.rs
│   │   ├── tool_handler.rs
│   │   ├── prompt_handler.rs
│   │   ├── mock_strategy.rs
│   │   ├── state_manager.rs
│   │   ├── workflow_engine.rs
│   │   └── agent_executor.rs   # AI Agent execution engine
│   ├── application/        # Application layer
│   │   └── mod.rs
│   ├── domain/            # Domain layer (Business logic)
│   │   └── mcp_types.rs
│   ├── config/            # Configuration management
│   │   └── mod.rs
│   ├── persistence/       # Database persistence layer
│   │   ├── mod.rs
│   │   ├── migrations.rs      # Migration runner
│   │   ├── archetype_repository.rs
│   │   └── commit_repository.rs
│   ├── cli.rs             # CLI commands and argument parsing
│   ├── lib.rs
│   └── main.rs
├── ui/                    # Web UI (Leptos CSR)
│   ├── src/
│   │   ├── components/    # UI components
│   │   ├── api.rs         # API client
│   │   └── types.rs       # Shared types
│   └── Cargo.toml
├── migrations/            # SQL migration files
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

### 💾 Database Persistence & Version Control
- **Multi-Database Support**: SQLite, PostgreSQL, MySQL via SQLx
- **Git-Style Versioning**: Commits, changesets, snapshots, and tags
- **Version History**: Track all changes to resources, tools, prompts, workflows, and agents
- **Rollback**: Restore to any previous commit or tagged version
- **Export/Import**: Full configuration backup and restore

### 💻 Developer Experience
- **Web UI**: Full dashboard with configuration editor and database management
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
metis [OPTIONS] [COMMAND]

Commands:
  encrypt-secret  Encrypt a secret value using AGE passphrase encryption
  decrypt-secret  Decrypt an AGE-encrypted secret value
  migrate         Run database migrations
  migrate-status  Show migration status
  export          Export configuration to JSON file
  import          Import configuration from JSON file
  version-list    List version history commits
  tag-list        List all tags
  tag-create      Create a new tag at HEAD
  rollback        Rollback to a specific commit
  help            Print help for commands

Options:
  -c, --config <CONFIG>               Path to config file [default: metis.toml]
      --host <HOST>                   Server host address
      --port <PORT>                   Server port
      --secret-passphrase <PASS>      Passphrase for decrypting AGE-encrypted secrets
      --s3-enabled                    Enable S3 configuration source
      --s3-bucket <BUCKET>            S3 bucket name for configuration files
      --s3-prefix <PREFIX>            S3 key prefix (e.g., "config/")
      --s3-region <REGION>            AWS region for S3
      --s3-endpoint <ENDPOINT>        S3 endpoint URL (for MinIO/LocalStack)
      --s3-poll-interval <SECS>       S3 polling interval in seconds
      --db-url <URL>                  Database connection URL
  -h, --help                          Print help
  -V, --version                       Print version
```

All CLI options can also be set via environment variables:
- `METIS_CONFIG`, `METIS_HOST`, `METIS_PORT`
- `METIS_SECRET_PASSPHRASE` - Passphrase for AGE-encrypted secrets
- `METIS_S3_ENABLED`, `METIS_S3_BUCKET`, `METIS_S3_PREFIX`
- `METIS_S3_REGION`, `METIS_S3_ENDPOINT`, `METIS_S3_POLL_INTERVAL`
- `METIS_DATABASE_URL` - Database connection URL for persistence

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

### Database Persistence Configuration (Optional)

Enable database persistence to store all archetypes (resources, tools, prompts, workflows, agents) with full version history:

```toml
[database]
url = "sqlite://metis.db"       # SQLite, PostgreSQL, or MySQL
max_connections = 5             # Connection pool size
auto_migrate = true             # Run migrations on startup
seed_on_startup = false         # Import config files into database on first run
snapshot_interval = 10          # Create full snapshot every N commits
```

**Supported Database URLs:**
- SQLite: `sqlite://metis.db` or `sqlite://:memory:`
- PostgreSQL: `postgres://user:pass@localhost/metis`
- MySQL: `mysql://user:pass@localhost/metis`

**CLI Commands for Database Management:**

```bash
# Run database migrations
metis migrate --db-url sqlite://metis.db

# Check migration status
metis migrate-status --db-url sqlite://metis.db

# Export all configuration to JSON
metis export --output backup.json

# Import configuration from JSON
metis import --input backup.json

# List version history
metis version-list --limit 20

# List all tags
metis tag-list

# Create a tag at current HEAD
metis tag-create --name "v1.0.0" --message "Production release"

# Rollback to a specific commit
metis rollback --commit abc123
```

**Version History Features:**

The database tracks all changes with git-style versioning:

- **Commits**: Each modification creates a commit with author, message, and timestamp
- **Changesets**: Individual changes within a commit (create, update, delete operations)
- **Snapshots**: Periodic full state captures for efficient rollback
- **Tags**: Named references to specific commits (e.g., releases)

Example API endpoints:
- `GET /api/database/status` - Database health and HEAD commit
- `GET /api/database/commits` - List commit history
- `GET /api/database/commits/:hash` - Get commit details with changesets
- `POST /api/database/rollback` - Rollback to a specific commit
- `GET /api/database/tags` - List all tags
- `POST /api/database/tags` - Create a new tag
- `DELETE /api/database/tags/:name` - Delete a tag

### Configuration Precedence

Configuration values are merged from multiple sources with increasing precedence (higher sources override lower):

```
┌─────────────────────────────────────────────────────────────┐
│  5. UI Configuration (Web UI changes)          [HIGHEST]   │
├─────────────────────────────────────────────────────────────┤
│  4. S3 Configuration (remote config files)                 │
├─────────────────────────────────────────────────────────────┤
│  3. CLI Arguments (--host, --port, etc.)                   │
├─────────────────────────────────────────────────────────────┤
│  2. Local Config File (metis.toml)                         │
├─────────────────────────────────────────────────────────────┤
│  1. Environment Variables (METIS_*, AWS_*)     [LOWEST]    │
└─────────────────────────────────────────────────────────────┘
```

**How it works:**
1. **Environment Variables** - Base configuration, always checked first as defaults
2. **Local Config File** - `metis.toml` values override environment variables
3. **CLI Arguments** - Command-line flags override config file values
4. **S3 Configuration** - Remote config from S3 bucket overrides local config
5. **UI Configuration** - Changes made via Web UI have highest priority (in-memory)

**Example scenario:**
```bash
# Environment variable sets default
export METIS_PORT=8080

# Config file overrides to 3000
# metis.toml: port = 3000

# CLI overrides to 4000
metis --port 4000

# Result: Server runs on port 4000
```

**Merging behavior:**
- Configuration sources are merged, not replaced entirely
- Only explicitly set values override; unset values inherit from lower precedence sources
- Arrays (tools, resources, agents, etc.) are merged by name/identifier
- S3 watcher polls for changes and merges updates automatically

### Secrets Management

Metis provides flexible API key and credential management with multiple options:

#### Priority Order for Secrets/API Keys

Secrets follow the same precedence as configuration (lowest to highest):

1. **Environment variables** - `OPENAI_API_KEY`, `AWS_ACCESS_KEY_ID`, etc.
2. **Config file secrets** - `[secrets]` section (plain or AGE-encrypted)
3. **S3 config secrets** - Secrets in remote S3 config file
4. **In-memory secrets** (set via Web UI) - highest priority

When saving config to disk, secrets from the UI are encrypted (if passphrase is set) and stored in the config file.

#### In-Memory Secrets (Web UI)

The Web UI provides a secure way to enter API keys that are stored only in server memory:

- Navigate to **Configuration** → **API Keys & Credentials**
- Enter keys for AI providers (OpenAI, Anthropic, Google) and AWS/S3
- Keys are **write-only** - never displayed after entry
- Keys are **lost on server restart** (ephemeral storage)

Available secret keys:
- `OPENAI_API_KEY` - OpenAI API key
- `ANTHROPIC_API_KEY` - Anthropic API key
- `GEMINI_API_KEY` - Google Gemini API key
- `AWS_ACCESS_KEY_ID` - AWS access key
- `AWS_SECRET_ACCESS_KEY` - AWS secret key
- `AWS_REGION` - AWS region

#### Config File Secrets

Store secrets directly in your config file (plain text or encrypted):

```toml
[secrets]
openai_api_key = "sk-..."                    # Plain text
anthropic_api_key = "age:YWdlLWVuY3J5..."    # AGE-encrypted
gemini_api_key = "AIza..."
aws_access_key_id = "AKIA..."
aws_secret_access_key = "age:YWdlLWVuY3J5..." # AGE-encrypted
aws_region = "us-east-1"
```

#### AGE Encryption

Encrypt sensitive values using [AGE](https://github.com/FiloSottile/age) passphrase encryption:

**Encrypt a secret:**
```bash
# With passphrase flag
metis encrypt-secret "sk-your-api-key" -p "your-passphrase"

# Interactive (prompts for passphrase)
metis encrypt-secret "sk-your-api-key"
```

Output: `age:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IHNjcnlwdC...`

**Decrypt a secret:**
```bash
metis decrypt-secret "age:YWdlLWVuY3J5cHRpb24..." -p "your-passphrase"
```

**Use encrypted secrets in config:**
```toml
[secrets]
openai_api_key = "age:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IHNjcnlwdC..."
```

**Provide passphrase at runtime:**
```bash
# Via CLI flag
metis --secret-passphrase "your-passphrase"

# Via environment variable
export METIS_SECRET_PASSPHRASE="your-passphrase"
metis
```

#### Environment Variables

Environment variables are always checked as a fallback:

```bash
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."
export GEMINI_API_KEY="AIza..."
export AWS_ACCESS_KEY_ID="AKIA..."
export AWS_SECRET_ACCESS_KEY="..."
export AWS_REGION="us-east-1"
```

### Web UI

The embedded Web UI provides a full-featured interface for managing your Metis server:

**Dashboard** (`/`)
- Server status and configuration overview
- Database health and version history status
- Quick links to all archetype counts

**Configuration Editor** (`/config`)
- Edit authentication settings (API Key, JWT, Basic Auth, OAuth2)
- Configure rate limiting
- Set up S3 storage integration
- Configure database persistence settings
- Import/Export configuration as JSON
- Save to memory, disk, or S3

**Archetype Management**
- **Resources** (`/resources`) - View, create, edit, delete, and test static resources
- **Resource Templates** (`/resource-templates`) - Manage dynamic URI template resources
- **Tools** (`/tools`) - Configure mock tools with various strategies
- **Prompts** (`/prompts`) - Define and test prompt templates
- **Workflows** (`/workflows`) - Build multi-step tool chains
- **Agents** (`/agents`) - Configure AI agents with LLM providers
- **Schemas** (`/schemas`) - Define reusable JSON Schema definitions

**Features in Each Archetype Editor:**
- Table and card view modes
- Inline JSON Schema editor with validation
- Mock strategy configuration (all 9 strategies)
- Test modal for live testing with custom input
- Delete confirmation dialogs

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

### Workflow Configuration

Workflows allow you to chain multiple tool calls together with conditional logic, loops, and data passing between steps.

```toml
[[workflows]]
name = "process_items"
description = "Process a list of items with validation"
input_schema = { type = "object", properties = { items = { type = "array" } } }
on_error = "fail"  # Options: "fail", "continue", { retry = { max_attempts = 3, delay_ms = 1000 } }, { fallback = { value = {} } }

[[workflows.steps]]
id = "validate"
tool = "validator"
args = { data = "{{ input.items }}" }

[[workflows.steps]]
id = "process_each"
tool = "processor"
condition = "steps.validate.success == true"  # Rhai expression
loop_over = "input.items"                      # Array to iterate over
loop_var = "item"                              # Variable name for current item (default: "item")
loop_concurrency = 4                           # Parallel execution (0 = sequential)
args = { item = "{{ item }}", index = "{{ loop.index }}" }

[[workflows.steps]]
id = "summarize"
tool = "summarizer"
args = { results = "{{ steps.process_each }}" }
```

**Workflow Features:**
- **Sequential Execution**: Steps run in order with data passing via `{{ steps.<id> }}`
- **Conditional Steps**: Skip steps based on Rhai expressions (`condition` field)
- **Looping**: Iterate over arrays with `loop_over`, access current item via `loop_var`
- **Parallel Loops**: Set `loop_concurrency > 0` for concurrent processing
- **Error Handling**: Per-step or workflow-level error strategies
- **Template Arguments**: Use Tera templates to reference input and previous step results

**Available Context in Templates:**
- `{{ input }}` - Original workflow input
- `{{ steps.<step_id> }}` - Result from a previous step
- `{{ item }}` - Current loop item (when using `loop_over`)
- `{{ loop.index }}` - Current loop index (0-based)

### AI Agent Configuration

AI Agents are autonomous LLM-powered components that can use tools, access resources, and maintain conversation memory.

```toml
[[agents]]
name = "research_assistant"
description = "An AI assistant that can search and summarize information"
agent_type = "react"  # Options: "single_turn", "multi_turn", "react"

# Input/Output schemas
input_schema = { type = "object", properties = { query = { type = "string" } } }
output_schema = { type = "object", properties = { answer = { type = "string" } } }

# LLM Configuration
[agents.llm]
provider = "openai"           # Options: "openai", "anthropic", "gemini", "ollama", "azureopenai"
model = "gpt-4"
api_key_env = "OPENAI_API_KEY"
temperature = 0.7
max_tokens = 2000
stream = true

# System prompt
system_prompt = """
You are a helpful research assistant. Use the available tools to find information
and provide comprehensive answers to user queries.
"""

# Optional: Prompt template for formatting user input
prompt_template = "Please research the following: {{ query }}"

# Available tools (local tool names)
available_tools = ["web_search", "summarize"]

# MCP tools from external servers (format: "server_name:tool_name" or "server_name:*")
mcp_tools = ["browser:*", "filesystem:read_file"]

# Other agents that can be called as tools
agent_tools = ["fact_checker"]

# Available resources
available_resources = ["file:///knowledge_base.json"]
available_resource_templates = ["postgres://db/documents/{id}"]

# Memory configuration
[agents.memory]
backend = "in_memory"         # Options: "in_memory", "file", "database"
strategy = { type = "sliding_window", size = 20 }  # Or "full", { type = "first_last", first = 5, last = 10 }
max_messages = 100
file_path = "memory/research_agent.json"  # For "file" backend

# Execution limits
max_iterations = 10
timeout_seconds = 120
```

**Agent Types:**
- `single_turn`: One-shot response, no tool use
- `multi_turn`: Conversational with memory, no tool use
- `react`: Full ReAct loop with reasoning, tool use, and observation

**Memory Strategies:**
- `full`: Keep all messages (up to `max_messages`)
- `sliding_window`: Keep last N messages
- `first_last`: Keep first N and last M messages

### Multi-Agent Orchestration

Orchestrations coordinate multiple agents to work together on complex tasks.

```toml
[[orchestrations]]
name = "document_analysis_pipeline"
description = "Analyze documents using multiple specialized agents"
pattern = "sequential"        # Options: "sequential", "hierarchical", "collaborative"

input_schema = { type = "object", properties = { document = { type = "string" } } }
output_schema = { type = "object", properties = { analysis = { type = "object" } } }

# Agent configuration
[[orchestrations.agents]]
agent = "extractor"           # Agent name to invoke
depends_on = []               # Wait for these agents to complete first
condition = "true"            # Rhai expression to conditionally run
input_transform = "{ document: input.document }"  # Transform input for this agent

[[orchestrations.agents]]
agent = "analyzer"
depends_on = ["extractor"]
input_transform = "{ data: steps.extractor.output }"

[[orchestrations.agents]]
agent = "summarizer"
depends_on = ["analyzer"]

# For hierarchical pattern: specify manager agent
manager_agent = "coordinator"

# For collaborative pattern: how to merge results
[orchestrations.merge_strategy]
type = "concat"               # Options: "concat", "vote", { type = "custom", script = "..." }

timeout_seconds = 300
```

**Orchestration Patterns:**
- `sequential`: Agents run in order based on dependencies (DAG execution)
- `hierarchical`: Manager agent coordinates worker agents
- `collaborative`: All agents work in parallel, results merged

### Reusable JSON Schema Definitions

Define schemas once and reference them across resources, tools, prompts, and agents using `$ref`:

```toml
[[schemas]]
name = "User"
description = "Standard user object schema"
schema = '''
{
  "type": "object",
  "properties": {
    "id": { "type": "string", "format": "uuid" },
    "email": { "type": "string", "format": "email" },
    "name": { "type": "string" },
    "created_at": { "type": "string", "format": "date-time" }
  },
  "required": ["id", "email", "name"]
}
'''

[[schemas]]
name = "PaginatedResponse"
description = "Generic paginated response wrapper"
schema = '''
{
  "type": "object",
  "properties": {
    "items": { "type": "array" },
    "total": { "type": "integer" },
    "page": { "type": "integer" },
    "per_page": { "type": "integer" }
  }
}
'''
```

**Using Schema References:**

```toml
[[resources]]
uri = "file:///users.json"
name = "User List"
output_schema = { "$ref": "#/schemas/PaginatedResponse" }

[[tools]]
name = "create_user"
description = "Create a new user"
input_schema = { "$ref": "#/schemas/User" }
output_schema = { "$ref": "#/schemas/User" }
```

Schema references are resolved at configuration load time, allowing you to maintain consistent data structures across your entire API surface.

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
- ✅ Authentication (API Key, JWT Bearer, Basic Auth, OAuth2/JWKS)
- ✅ Secrets Management (Web UI + AGE encryption)
- ✅ Advanced Workflow engine
- ✅ AI Agents (Single-turn, Multi-turn, ReAct)
- ✅ Multi-agent Orchestration (Sequential, Hierarchical, Collaborative)
- ✅ Database Persistence with Version History
- ✅ Enhanced Web UI with Configuration Editor
- ✅ Reusable JSON Schema Definitions

### Planned Features
- [ ] Performance optimizations (>10k req/s)
- [ ] WebSocket support for real-time updates
- [ ] Distributed mode with Redis backend

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
