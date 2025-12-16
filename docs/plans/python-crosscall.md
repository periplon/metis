# Python Tools Cross-Invocation System

**Status**: Ready for Implementation
**Quality Score**: 29/30
**Planning Rounds**: 2 (Adversarial Collaboration)
**Date**: 2025-12-16

---

## Overview

Enable Python scripts executed via the Script mock strategy to call agents, tools, workflows, and resources through native Python functions. Uses `ExecutionContext` for depth tracking, access control, and timeout management. Fully integrated with existing `ToolPort` interface via optional method override.

## Goals

- **Primary**: Python scripts can invoke agents via `call_agent(name, input)` and tools via `call_tool(name, args)`
- **Secondary**: Access resources via `get_resource(uri)` and resource templates via `get_resource_template(name, params)`
- **Secondary**: Support workflow invocation via `call_workflow(name, input)`
- **Tertiary**: Maintain session continuity for multi-turn agent conversations

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     Python Script Execution                      │
├─────────────────────────────────────────────────────────────────┤
│  Native Functions: call_tool, call_agent, get_resource,         │
│                    get_resource_template, call_workflow          │
├─────────────────────────────────────────────────────────────────┤
│                     ExecutionContext                             │
│  ┌─────────────┬─────────────┬──────────────┬─────────────────┐ │
│  │ call_depth  │ timeout_ms  │ calling_tool │ access_policy   │ │
│  │    u32      │    u64      │  Option<Str> │ AccessPolicy    │ │
│  └─────────────┴─────────────┴──────────────┴─────────────────┘ │
├─────────────────────────────────────────────────────────────────┤
│           ToolPort.execute_tool_with_context()                   │
│           (default impl → execute_tool for compatibility)        │
├─────────────────────────────────────────────────────────────────┤
│  BasicToolHandler │ AgentHandler │ WorkflowEngine │ Resources   │
└─────────────────────────────────────────────────────────────────┘
```

### Data Flow

```
User Request → MockStrategyHandler.generate()
                    │
                    ├── Creates ExecutionContext { depth: 0, ... }
                    │
                    └── execute_python_script(script, input, ctx)
                            │
                            ├── Registers native functions with ctx.clone()
                            │
                            └── Python: call_tool("calc", {})
                                    │
                                    ├── Check: depth < max_call_depth
                                    ├── Check: "calc" allowed by policy
                                    ├── Check: "calc" != calling_tool
                                    │
                                    └── tool_handler.execute_tool_with_ctx(
                                            "calc",
                                            args,
                                            ctx.increment_depth()
                                        )
                                            │
                                            └── If calc is also Python script:
                                                    │
                                                    └── execute_python_script(..., ctx)
                                                        (depth now = 2)
```

## Key Components

| Component | File | Purpose |
|-----------|------|---------|
| ExecutionContext | `src/adapters/execution_context.rs` | Propagating context for cross-invocations |
| AccessPolicy | `src/adapters/execution_context.rs` | Allow/Deny lists for tools/agents/resources |
| Native Functions | `src/adapters/mock_strategy.rs` | Python ↔ Rust bridge functions |
| ScriptAccessConfig | `src/config/schema.rs` | Configuration schema for access control |
| Error Hierarchy | `src/adapters/mock_strategy.rs` | Typed Python exceptions |

## Implementation Details

### ExecutionContext Structure

```rust
// src/adapters/execution_context.rs (new file)

#[derive(Clone)]
pub struct ExecutionContext {
    pub call_depth: u32,
    pub max_call_depth: u32,           // Default: 10
    pub timeout_ms: u64,                // Remaining timeout budget
    pub calling_tool: Option<String>,   // For self-invocation prevention
    pub access_policy: AccessPolicy,    // Allowed tools/agents/resources
    pub session_id: Option<String>,     // For agent session continuity
}

#[derive(Clone, Default)]
pub struct AccessPolicy {
    pub tools: AccessLevel,
    pub agents: AccessLevel,
    pub resources: AccessLevel,
    pub workflows: AccessLevel,
}

#[derive(Clone, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessLevel {
    #[default]
    All,                        // Allow all
    None,                       // Deny all
    #[serde(rename = "allow_list")]
    AllowList(Vec<String>),     // Only these
    #[serde(rename = "deny_list")]
    DenyList(Vec<String>),      // All except these
}
```

### Extended ToolPort Interface

```rust
// src/domain/mod.rs - Add new method to ToolPort trait

#[async_trait]
pub trait ToolPort: Send + Sync {
    async fn execute_tool(&self, name: &str, args: Value) -> anyhow::Result<Value>;

    // Default: delegate to execute_tool, ignoring context (backward compatible)
    async fn execute_tool_with_context(
        &self,
        name: &str,
        args: Value,
        _ctx: ExecutionContext
    ) -> anyhow::Result<Value> {
        self.execute_tool(name, args).await
    }

    async fn list_tools(&self) -> anyhow::Result<Vec<Tool>>;
}
```

### Configuration Schema

```rust
// src/config/schema.rs - Update MockConfig

pub struct ScriptMockConfig {
    pub language: ScriptLanguage,  // Python, Rhai, Lua, JavaScript
    pub script: String,

    // Access control
    #[serde(default)]
    pub access: ScriptAccessConfig,

    // Execution limits
    #[serde(default = "default_max_depth")]
    pub max_call_depth: u32,       // Default: 10
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,           // Default: 30000
}

#[derive(Default, Deserialize)]
pub struct ScriptAccessConfig {
    #[serde(default)]
    pub tools: AccessLevel,
    #[serde(default)]
    pub agents: AccessLevel,
    #[serde(default)]
    pub resources: AccessLevel,
    #[serde(default)]
    pub workflows: AccessLevel,
}
```

### Native Function Implementation

```rust
// In src/adapters/mock_strategy.rs

fn register_python_functions(
    vm: &VirtualMachine,
    scope: &Scope,
    tool_handler: Arc<dyn ToolPort>,
    agent_handler: Option<Arc<dyn AgentPort>>,
    ctx: ExecutionContext,
) -> PyResult<()> {
    let th = tool_handler.clone();
    let call_ctx = ctx.clone();

    let call_tool_fn = vm.ctx.new_function(
        "call_tool",
        move |name: PyStrRef, args: PyDictRef, timeout_ms: OptionalArg<u64>, vm: &VirtualMachine| {
            // 1. Depth check
            if call_ctx.call_depth >= call_ctx.max_call_depth {
                return Err(vm.new_runtime_error(
                    "RecursionLimitError: Maximum call depth exceeded".to_string()
                ));
            }

            // 2. Self-invocation check
            if call_ctx.calling_tool.as_deref() == Some(name.as_str()) {
                return Err(vm.new_runtime_error(
                    "RecursionLimitError: Self-invocation not allowed".to_string()
                ));
            }

            // 3. Access policy check
            if !call_ctx.access_policy.tools.allows(name.as_str()) {
                return Err(vm.new_runtime_error(
                    format!("PermissionDeniedError: Tool '{}' not in allowed list", name)
                ));
            }

            // 4. Execute with context propagation
            let new_ctx = call_ctx.increment_depth()
                .with_calling_tool(name.as_str().to_string());

            let timeout = timeout_ms.unwrap_or(call_ctx.timeout_ms);

            tokio::task::block_in_place(|| {
                let rt = tokio::runtime::Handle::current();
                rt.block_on(async {
                    tokio::time::timeout(
                        Duration::from_millis(timeout),
                        th.execute_tool_with_context(name.as_str(), args_json, new_ctx)
                    ).await
                    .map_err(|_| "TimeoutError: Tool call timed out")
                    .and_then(|r| r.map_err(|e| format!("InvocationError: {}", e)))
                })
            })
            .map(|v| json_to_pyobject(vm, v))
            .map_err(|e| vm.new_runtime_error(e))
        }
    );

    scope.globals.set_item("call_tool", call_tool_fn, vm)?;

    // Similar for call_agent, get_resource, call_workflow...
    Ok(())
}
```

### Agent Response Format

```python
# call_agent returns:
{
    "content": "...",           # Primary response content (string or structured)
    "tool_calls": [...],        # Optional: tool calls made by agent
    "session_id": "uuid",       # Session ID used/created
    "iterations": 3,            # For ReAct: iteration count
    "status": "success"         # "success", "max_iterations", "error"
}
```

## Configuration Example

```toml
[[tools]]
name = "smart_processor"
description = "Python tool with AI and tool access"

[tools.mock]
strategy = "script"
language = "python"
max_call_depth = 5
timeout_ms = 60000
script = """
# Analyze with AI agent
analysis = call_agent("analyzer", {"data": input}, timeout_ms=10000)

# Transform with another tool
result = call_tool("transformer", {"analysis": analysis["content"]})

# Access a resource
config = get_resource("config://processing/rules")

output = {
    "processed": result,
    "rules_applied": config["rules"]
}
"""

[tools.mock.access]
agents = { allow_list = ["analyzer", "summarizer"] }
tools = { allow_list = ["transformer", "validator"] }
resources = "all"
workflows = "none"
```

## Security Controls

| Control | Implementation |
|---------|---------------|
| Recursion Limit | `max_call_depth` (default: 10) |
| Timeout | Per-call timeout with budget tracking |
| Access Control | AllowList/DenyList per resource type |
| Self-Invocation | Prevented via `calling_tool` tracking |

## Error Handling

### Python Exception Hierarchy

```python
class MetisError(Exception): pass
class ToolNotFoundError(MetisError): pass
class AgentNotFoundError(MetisError): pass
class TimeoutError(MetisError): pass
class RecursionLimitError(MetisError): pass
class PermissionDeniedError(MetisError): pass
class InvocationError(MetisError): pass
```

### Error Mapping

| Python Exception | Trigger |
|-----------------|---------|
| `ToolNotFoundError` | Unknown tool name |
| `AgentNotFoundError` | Unknown agent name |
| `TimeoutError` | Call exceeds timeout |
| `RecursionLimitError` | Depth exceeded or self-invocation |
| `PermissionDeniedError` | Access policy violation |
| `InvocationError` | Other execution errors |

## Implementation Steps

1. [ ] **Create ExecutionContext** - `src/adapters/execution_context.rs`
   - Define `ExecutionContext`, `AccessPolicy`, `AccessLevel`
   - Implement `increment_depth()`, `with_calling_tool()`, helper methods
   - Implement `AccessLevel::allows(&str) -> bool`

2. [ ] **Extend ToolPort trait** - `src/domain/mod.rs`
   - Add `execute_tool_with_context` method with default impl
   - Implement override in `BasicToolHandler` and `InnerToolHandler`

3. [ ] **Update MockConfig schema** - `src/config/schema.rs`
   - Add `ScriptAccessConfig` struct
   - Add `max_call_depth` and `timeout_ms` fields

4. [ ] **Implement call_tool native function** - `src/adapters/mock_strategy.rs`
   - Depth checking, access control, self-invocation prevention
   - Timeout wrapping with `tokio::time::timeout`
   - Error mapping to Python exceptions

5. [ ] **Implement call_agent native function**
   - Similar structure to `call_tool`
   - Session ID management (auto-generate if not provided)
   - Return structured response with content, session_id, status

6. [ ] **Implement get_resource and get_resource_template**
   - Route through tool_handler with resource prefixes
   - Access control validation

7. [ ] **Implement call_workflow native function**
   - Access control for workflows
   - Context propagation into workflow steps

8. [ ] **Register Python exception classes**
   - Create exception class hierarchy in Python scope
   - Map Rust errors to appropriate exception types

9. [ ] **Update MockStrategyHandler**
   - Create ExecutionContext from ScriptMockConfig
   - Pass context through Python execution chain
   - Handle agent_handler absence gracefully

10. [ ] **Integration tests** - `tests/python_crosscall_test.rs`
    - Depth limit enforcement
    - Access control validation
    - Timeout behavior
    - Error type propagation
    - Multi-turn session continuity

## Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `src/adapters/execution_context.rs` | Create | ExecutionContext, AccessPolicy structs |
| `src/domain/mod.rs` | Modify | Add execute_tool_with_context to ToolPort |
| `src/adapters/tool_handler.rs` | Modify | Implement new trait method |
| `src/adapters/mock_strategy.rs` | Modify | Native functions, context integration |
| `src/config/schema.rs` | Modify | ScriptAccessConfig, execution limits |
| `tests/python_crosscall_test.rs` | Create | Integration tests |

## Risk Mitigation

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Recursive deadlock | Medium | High | Depth limit (default 10) |
| Unrestricted access | Medium | High | AccessPolicy with allow/deny lists |
| Hung system | Medium | Medium | Per-call timeouts with budget |
| API breakage | Low | Medium | Default trait implementation |
| Self-invocation | Medium | Medium | calling_tool tracking |
| Type conversion issues | Medium | Low | Comprehensive test coverage |

## Success Criteria

- [ ] Python `call_tool` works with context propagation
- [ ] Python `call_agent` works with session management
- [ ] Recursion depth limit enforced (test: depth=3, max=2 → error)
- [ ] Access control enforced (test: denied tool → PermissionDeniedError)
- [ ] Self-invocation prevented (Python tool calling itself → error)
- [ ] Timeout enforced (test: 100ms timeout, 200ms tool → TimeoutError)
- [ ] Errors surface as typed Python exceptions
- [ ] Backward compatible (existing scripts work without access config)
- [ ] No performance regression

## Dependencies

- No new external crates required
- Uses existing: `rustpython_vm`, `tokio`, `serde_json`
- Leverages existing `block_in_place` pattern from `datafusion_query`

---

## Adversarial Planning Notes

This plan was developed through 2 rounds of adversarial collaboration:

**Round 1 Issues Addressed**:
- Recursive invocation deadlock risk → Added depth tracking
- Unrestricted tool access → Added AccessPolicy
- Missing timeout handling → Added per-call timeouts
- Agent response format ambiguity → Defined explicit return schema
- Error categorization → Implemented exception hierarchy

**Round 2 Refinements**:
- ToolPort backward compatibility → Default trait implementation
- AccessLevel serialization → Proper serde attributes
- Removed premature trace_id field

**Quality Scores**:
- Completeness: 5/5
- Correctness: 5/5
- Security: 5/5
- Performance: 4/5
- Maintainability: 5/5
- Feasibility: 5/5
- **Total: 29/30**
