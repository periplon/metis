# Metis Implementation - Priority Action Items

**Based on**: Technical Review 2025-11-19
**Status**: 🔴 CRITICAL ACTIONS REQUIRED BEFORE PHASE 1

---

## 🔴 CRITICAL - Before Starting Phase 1

### 1. Timeline Revision (MANDATORY)

**Current**: 19 weeks total
**Realistic**: 32-36 weeks total

**Action Required**:
```
Week 0: Pre-planning
├─ Research Rust MCP SDK status
├─ Prototype multi-language scripting
└─ Technology spike on Leptos + Monaco

Weeks 1-16: v1.0 (MVP)
├─ Phase 1-6 only
├─ NO workflow engine
├─ NO web UI
└─ NO advanced agent features

Weeks 17-32: Post-v1.0 Features
├─ v1.1: Web UI (6 weeks)
├─ v1.2: Workflow Engine (6 weeks)
└─ v1.3: Advanced Features (4 weeks)
```

**Deliverable**: Revised project schedule with realistic milestones

### 2. Verify Rust MCP SDK Exists

**Risk**: Plan assumes SDK exists, but may not be mature/official

**Action Required**:
- [ ] Research official Rust MCP SDK status
- [ ] Test basic MCP protocol implementation
- [ ] **Decision**: Use SDK OR build from spec
- [ ] **Deadline**: Week 0 (before Phase 1 starts)

**If SDK doesn't exist**:
- Option A: Build protocol handler from MCP spec (add 2-3 weeks)
- Option B: Use TypeScript SDK via FFI (not ideal)
- Option C: Wait for official SDK (delays project)

### 3. Define MVP Scope Explicitly

**Action Required**:
Create `MVP-SCOPE.md` document listing EXACTLY what's in v1.0:

**IN v1.0**:
- ✅ Core MCP protocol handler
- ✅ 6 mock strategies: Random, Template, Script (Rhai only), File, Database, Static
- ✅ Authentication (API key, JWT)
- ✅ Configuration system with hot reload
- ✅ CLI interface
- ✅ Observability (metrics, logging)

**OUT of v1.0** (deferred to v1.1+):
- ❌ LLM mock strategy (expensive, complex)
- ❌ Composite strategies
- ❌ Multi-language scripting (Python, Lua, Ruby, JS)
- ❌ Web UI
- ❌ Workflow engine
- ❌ Model system
- ❌ Advanced agent orchestration

**Deliverable**: MVP-SCOPE.md with go/no-go criteria for each feature

---

## 🟡 HIGH PRIORITY - Week 0 (Pre-Phase 1)

### 4. Technology Validation Spikes

**Multi-Language Scripting Spike** (2 days):
```rust
// Validate pyo3 (Python)
// Test basic script execution
// Verify sandboxing works
// Measure performance overhead

// Validate mlua (Lua)
// Compare ease of use vs Python
```

**Deliverable**: Spike report recommending Rhai-only OR Rhai+Python for v1.0

**Leptos + Monaco Spike** (3 days):
```rust
// Create basic Leptos app
// Integrate Monaco editor
// Test WASM bundle size
// Verify hot reload works
```

**Deliverable**: Proof-of-concept OR decision to defer Web UI

### 5. Set Up Architecture Decision Records (ADRs)

**Action Required**:
Create `docs/adr/` directory with template:

```markdown
# ADR-001: Use Hexagonal Architecture

Date: 2025-11-19
Status: Accepted

## Context
[Why this decision is needed]

## Decision
[What we decided]

## Consequences
[Trade-offs and implications]
```

**Key ADRs to Write**:
- [ ] ADR-001: Hexagonal Architecture
- [ ] ADR-002: SOLID Principles Application
- [ ] ADR-003: Mock Strategy Pattern
- [ ] ADR-004: Multi-Language Scripting Approach
- [ ] ADR-005: Testing Strategy

**Deliverable**: 5 initial ADRs documenting core architecture

---

## 🟢 MEDIUM PRIORITY - Phase 1

### 6. Simplify Scripting Language Support

**Current Plan**: 5 languages (Python, Lua, Ruby, Rhai, JS)
**Recommended**: Start with 1-2 languages

**Decision Tree**:
```
v1.0: Rhai ONLY
  ├─ Pro: Native Rust, fast, safe, no FFI
  ├─ Pro: Easy sandboxing
  └─ Con: Less familiar to users

v1.0: Rhai + Python
  ├─ Pro: Python is widely known
  ├─ Con: pyo3 complexity (GIL, memory)
  └─ Con: Harder to sandbox

v1.1+: Add other languages based on demand
```

**Action Required**:
- [ ] Decide: Rhai-only OR Rhai+Python for v1.0
- [ ] Defer: Lua, Ruby, JavaScript to v1.1+
- [ ] Document decision in ADR

### 7. Database Strategy Security Hardening

**Current Risk**: SQL injection, unauthorized access

**Action Required**:
Add to Phase 1 deliverables:

```rust
pub struct DatabaseStrategyConfig {
    connection_string: String,

    // NEW: Security constraints
    read_only: bool,              // Default: true
    allowed_tables: Vec<String>,  // Whitelist only
    query_timeout_ms: u64,        // Default: 5000ms
    max_rows: usize,              // Default: 1000
}

// Validation logic
impl DatabaseStrategy {
    fn validate_query(&self, sql: &str) -> Result<(), Error> {
        // Block: DROP, DELETE, UPDATE, INSERT
        // Allow: SELECT only
        // Validate: Parameterized queries only
    }
}
```

**Deliverable**: Security specification for database strategy

### 8. Performance Baseline Early

**Action Required**:
Week 2-3 (Phase 1), establish performance baseline:

```bash
# Create simple benchmark
cargo bench

# Target: >10k req/s for Random strategy
# Measure: 50th, 90th, 99th percentile latency
# Tool: criterion.rs
```

**Deliverable**: Baseline performance report with targets for each strategy

---

## 🔵 LOWER PRIORITY - Phase 2-3

### 9. LLM Strategy Cost Controls

**Action Required**:
Before implementing LLM strategy (Phase 3):

```rust
pub struct LlmStrategyConfig {
    provider: LlmProvider,
    model: String,

    // NEW: Cost controls
    max_cost_per_request: f64,     // e.g., $0.10
    daily_budget: f64,              // e.g., $50.00
    cost_alert_threshold: f64,      // e.g., $40.00

    // NEW: Local LLM support
    use_local_model: bool,
    local_model_url: Option<String>, // Ollama, llama.cpp
}
```

**Deliverable**: Cost control specification before implementing LLM strategy

### 10. Workflow Engine Scope Reduction

**Current Scope**: Full workflow engine with branching, looping, parallel execution
**Realistic**: Too complex for v1.0

**Recommendation**:
```
v1.0: No workflow engine
  └─ Use script strategy instead

v1.1: Simple sequential workflows
  ├─ Linear step execution
  ├─ No branching
  └─ No loops

v1.2: Add control flow
  ├─ If/else branching
  └─ Basic error handling

v1.3: Advanced features
  ├─ Loops
  ├─ Parallel execution
  └─ Sub-workflows
```

**Action Required**:
- [ ] Remove workflow engine from Phase 7
- [ ] Create separate roadmap for workflow features
- [ ] Consider: Integration with Temporal instead of building

### 11. Web UI Technology Decision

**Current Plan**: Leptos (cutting edge, risky)

**Action Required**:
Evaluate alternatives in Week 0:

| Option | Pros | Cons | Risk |
|--------|------|------|------|
| **Leptos** | Rust, WASM, reactive | New (v0.6), breaking changes | 🟡 Medium |
| **Yew** | Mature Rust WASM | Verbose, slower DX | 🟢 Low |
| **Dioxus** | React-like, good DX | Smaller community | 🟡 Medium |
| **htmx + Tailwind** | Simple, proven | Server-side rendering only | 🟢 Low |
| **No UI (CLI only)** | Simplest | Users want UI | N/A |

**Recommendation**: htmx + Tailwind for v1.1 (faster to build, less risky)

**Deliverable**: UI technology decision documented in ADR

---

## 📋 Checklist - Before Starting Phase 1

### Week -1: Pre-Planning
- [ ] Revise project timeline to 32-36 weeks
- [ ] Define explicit MVP scope (MVP-SCOPE.md)
- [ ] Research Rust MCP SDK status
- [ ] Create architecture decision record template
- [ ] Set up ADR directory structure

### Week 0: Technology Validation
- [ ] Spike: Multi-language scripting (Rhai vs Python)
- [ ] Spike: Leptos + Monaco editor (or decide to defer UI)
- [ ] Spike: Basic MCP protocol implementation
- [ ] Write ADR-001 through ADR-005
- [ ] Create performance baseline plan
- [ ] Security specification for database strategy

### Week 1: Phase 1 Kickoff (Only if all above complete)
- [ ] All spikes completed with decisions made
- [ ] Revised timeline approved by stakeholders
- [ ] MVP scope clearly defined and agreed upon
- [ ] Technology choices documented in ADRs
- [ ] Team aligned on realistic expectations

---

## 🎯 Success Criteria

**v1.0 Release** (Week 16):
- [ ] Core MCP server operational
- [ ] 6 mock strategies working
- [ ] >80% test coverage
- [ ] >10k req/s for simple strategies
- [ ] Security audit passed
- [ ] Documentation complete
- [ ] 10+ GitHub stars (early validation)

**v1.1 Release** (Week 22):
- [ ] Web UI launched
- [ ] User feedback incorporated
- [ ] Additional mock strategies if needed

**v1.2 Release** (Week 28):
- [ ] Workflow engine (basic) if validated by users
- [ ] OR other features based on feedback

---

## 🚫 What NOT to Do

**Don't**:
- ❌ Start Phase 1 without completing Week 0 validation
- ❌ Try to build all Phase 7 features in 3 weeks
- ❌ Implement features without user validation first
- ❌ Skip security testing "to save time"
- ❌ Build workflow engine from scratch without considering alternatives
- ❌ Commit to 19-week timeline publicly
- ❌ Add "just one more feature" to v1.0 scope

**Do**:
- ✅ Ship early, get feedback, iterate
- ✅ Maintain code quality over speed
- ✅ Document all architecture decisions
- ✅ Test continuously (TDD approach)
- ✅ Re-evaluate after Phase 3
- ✅ Be honest about timeline with stakeholders
- ✅ Celebrate incremental progress

---

## 📊 Revised Timeline Summary

```
Week -1:  Pre-planning
Week 0:   Technology spikes & validation
Week 1-3: Phase 1 - Core Foundation
Week 4-6: Phase 2 - Mock Strategies
Week 7-9: Phase 3 - Advanced Features
  └─ [CHECKPOINT: Re-evaluate scope & timeline]
Week 10-11: Phase 4 - Security
Week 12-13: Phase 5 - Configuration
Week 14-15: Phase 6 - Observability
Week 16:    v1.0 RELEASE 🎉
  └─ [CHECKPOINT: Gather user feedback]
Week 17-22: v1.1 - Web UI (if validated)
Week 23-28: v1.2 - Workflow Engine (if validated)
Week 29-32: v1.3 - Advanced Features (based on feedback)
```

**Total**: 32 weeks (8 months) for full vision
**MVP**: 16 weeks (4 months) for usable product

---

**Next Steps**:
1. Review this document with team
2. Make go/no-go decision on revised timeline
3. Begin Week -1 pre-planning activities
4. Schedule Week 0 technology validation spikes

**Document Owner**: Technical Lead
**Last Updated**: 2025-11-19
**Status**: 🔴 PENDING APPROVAL
