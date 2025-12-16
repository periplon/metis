//! Execution context for cross-tool invocations.
//!
//! This module provides the `ExecutionContext` type used to propagate
//! state across tool, agent, workflow, and resource invocations.
//! It enables depth tracking (recursion limits), access control,
//! timeout management, and session continuity.

use serde::{Deserialize, Serialize};

/// Default maximum call depth for recursive tool invocations.
pub const DEFAULT_MAX_CALL_DEPTH: u32 = 10;

/// Default timeout in milliseconds for tool/agent calls.
pub const DEFAULT_TIMEOUT_MS: u64 = 30_000;

/// Execution context propagated through cross-tool invocations.
///
/// This context carries state needed for:
/// - **Recursion limits**: Prevents infinite loops via `call_depth` tracking
/// - **Access control**: Restricts which tools/agents/resources a script can call
/// - **Timeout management**: Per-call timeout with budget tracking
/// - **Session continuity**: Maintains agent session across calls
/// - **Self-invocation prevention**: Tracks the calling tool name
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Current call depth (incremented on each nested invocation)
    pub call_depth: u32,
    /// Maximum allowed call depth before failing with recursion error
    pub max_call_depth: u32,
    /// Timeout in milliseconds for the current operation
    pub timeout_ms: u64,
    /// Name of the tool that initiated this context (for self-invocation prevention)
    pub calling_tool: Option<String>,
    /// Access control policy for tools, agents, resources, and workflows
    pub access_policy: AccessPolicy,
    /// Session ID for agent conversation continuity
    pub session_id: Option<String>,
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self {
            call_depth: 0,
            max_call_depth: DEFAULT_MAX_CALL_DEPTH,
            timeout_ms: DEFAULT_TIMEOUT_MS,
            calling_tool: None,
            access_policy: AccessPolicy::default(),
            session_id: None,
        }
    }
}

impl ExecutionContext {
    /// Create a new execution context with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new context with specified limits.
    pub fn with_limits(max_call_depth: u32, timeout_ms: u64) -> Self {
        Self {
            max_call_depth,
            timeout_ms,
            ..Self::default()
        }
    }

    /// Create a new context with an access policy.
    pub fn with_access_policy(access_policy: AccessPolicy) -> Self {
        Self {
            access_policy,
            ..Self::default()
        }
    }

    /// Increment the call depth and return a new context.
    ///
    /// This should be called before making a nested tool/agent call.
    #[must_use]
    pub fn increment_depth(&self) -> Self {
        Self {
            call_depth: self.call_depth + 1,
            ..self.clone()
        }
    }

    /// Set the calling tool name (for self-invocation prevention).
    #[must_use]
    pub fn with_calling_tool(mut self, tool_name: String) -> Self {
        self.calling_tool = Some(tool_name);
        self
    }

    /// Set a session ID for agent conversation continuity.
    #[must_use]
    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    /// Set a custom timeout.
    #[must_use]
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Check if the current depth exceeds the maximum allowed.
    pub fn is_depth_exceeded(&self) -> bool {
        self.call_depth >= self.max_call_depth
    }

    /// Check if calling a specific tool would be self-invocation.
    pub fn is_self_invocation(&self, tool_name: &str) -> bool {
        self.calling_tool.as_deref() == Some(tool_name)
    }

    /// Check if a tool is allowed by the access policy.
    pub fn is_tool_allowed(&self, tool_name: &str) -> bool {
        self.access_policy.tools.allows(tool_name)
    }

    /// Check if an agent is allowed by the access policy.
    pub fn is_agent_allowed(&self, agent_name: &str) -> bool {
        self.access_policy.agents.allows(agent_name)
    }

    /// Check if a resource is allowed by the access policy.
    pub fn is_resource_allowed(&self, resource_uri: &str) -> bool {
        self.access_policy.resources.allows(resource_uri)
    }

    /// Check if a workflow is allowed by the access policy.
    pub fn is_workflow_allowed(&self, workflow_name: &str) -> bool {
        self.access_policy.workflows.allows(workflow_name)
    }
}

/// Access control policy for script cross-invocations.
///
/// Defines which tools, agents, resources, and workflows a script
/// is allowed to call. Each category has an independent access level.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AccessPolicy {
    /// Access level for tools
    #[serde(default)]
    pub tools: AccessLevel,
    /// Access level for agents
    #[serde(default)]
    pub agents: AccessLevel,
    /// Access level for resources
    #[serde(default)]
    pub resources: AccessLevel,
    /// Access level for workflows
    #[serde(default)]
    pub workflows: AccessLevel,
}

impl AccessPolicy {
    /// Create a policy that allows all access (default).
    pub fn allow_all() -> Self {
        Self::default()
    }

    /// Create a policy that denies all access.
    pub fn deny_all() -> Self {
        Self {
            tools: AccessLevel::None,
            agents: AccessLevel::None,
            resources: AccessLevel::None,
            workflows: AccessLevel::None,
        }
    }
}

/// Access level for a category of resources.
///
/// Supports four modes:
/// - `All`: Allow access to all items (default)
/// - `None`: Deny access to all items
/// - `AllowList`: Only allow items in the list
/// - `DenyList`: Allow all except items in the list
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AccessLevel {
    /// Allow access to all items in this category
    All,
    /// Deny access to all items in this category
    None,
    /// Only allow items explicitly listed
    #[serde(rename = "allow_list")]
    AllowList(Vec<String>),
    /// Allow all items except those listed
    #[serde(rename = "deny_list")]
    DenyList(Vec<String>),
}

impl Default for AccessLevel {
    fn default() -> Self {
        Self::All
    }
}

impl AccessLevel {
    /// Check if a given name is allowed by this access level.
    ///
    /// Note: Comparisons are **case-sensitive**. Ensure tool/agent/resource
    /// names are normalized before checking access.
    pub fn allows(&self, name: &str) -> bool {
        match self {
            Self::All => true,
            Self::None => false,
            Self::AllowList(list) => list.iter().any(|item| item == name),
            Self::DenyList(list) => !list.iter().any(|item| item == name),
        }
    }

    /// Create an AllowList access level.
    pub fn allow_only(items: Vec<String>) -> Self {
        Self::AllowList(items)
    }

    /// Create a DenyList access level.
    pub fn deny_only(items: Vec<String>) -> Self {
        Self::DenyList(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_access_level_all() {
        let level = AccessLevel::All;
        assert!(level.allows("anything"));
        assert!(level.allows(""));
    }

    #[test]
    fn test_access_level_none() {
        let level = AccessLevel::None;
        assert!(!level.allows("anything"));
        assert!(!level.allows(""));
    }

    #[test]
    fn test_access_level_allow_list() {
        let level = AccessLevel::AllowList(vec!["tool_a".into(), "tool_b".into()]);
        assert!(level.allows("tool_a"));
        assert!(level.allows("tool_b"));
        assert!(!level.allows("tool_c"));
        assert!(!level.allows(""));
    }

    #[test]
    fn test_access_level_deny_list() {
        let level = AccessLevel::DenyList(vec!["dangerous".into()]);
        assert!(!level.allows("dangerous"));
        assert!(level.allows("safe"));
        assert!(level.allows(""));
    }

    #[test]
    fn test_execution_context_depth() {
        let ctx = ExecutionContext::new();
        assert_eq!(ctx.call_depth, 0);
        assert!(!ctx.is_depth_exceeded());

        let ctx2 = ctx.increment_depth();
        assert_eq!(ctx2.call_depth, 1);

        // Create context at max depth
        let ctx_max = ExecutionContext {
            call_depth: 10,
            max_call_depth: 10,
            ..ExecutionContext::default()
        };
        assert!(ctx_max.is_depth_exceeded());
    }

    #[test]
    fn test_execution_context_self_invocation() {
        let ctx = ExecutionContext::new().with_calling_tool("my_tool".into());
        assert!(ctx.is_self_invocation("my_tool"));
        assert!(!ctx.is_self_invocation("other_tool"));
    }

    #[test]
    fn test_execution_context_access_policy() {
        let policy = AccessPolicy {
            tools: AccessLevel::AllowList(vec!["allowed_tool".into()]),
            agents: AccessLevel::None,
            resources: AccessLevel::All,
            workflows: AccessLevel::DenyList(vec!["blocked_workflow".into()]),
        };

        let ctx = ExecutionContext {
            access_policy: policy,
            ..ExecutionContext::default()
        };

        assert!(ctx.is_tool_allowed("allowed_tool"));
        assert!(!ctx.is_tool_allowed("other_tool"));
        assert!(!ctx.is_agent_allowed("any_agent"));
        assert!(ctx.is_resource_allowed("any_resource"));
        assert!(ctx.is_workflow_allowed("good_workflow"));
        assert!(!ctx.is_workflow_allowed("blocked_workflow"));
    }

    #[test]
    fn test_access_level_serialization() {
        // Test All
        let all = AccessLevel::All;
        let json = serde_json::to_string(&all).unwrap();
        assert_eq!(json, "\"all\"");

        // Test AllowList
        let allow = AccessLevel::AllowList(vec!["a".into(), "b".into()]);
        let json = serde_json::to_string(&allow).unwrap();
        assert!(json.contains("allow_list"));

        // Test deserialization
        let parsed: AccessLevel = serde_json::from_str("\"all\"").unwrap();
        assert_eq!(parsed, AccessLevel::All);

        let parsed: AccessLevel = serde_json::from_str("\"none\"").unwrap();
        assert_eq!(parsed, AccessLevel::None);
    }

    #[test]
    fn test_access_policy_helpers() {
        let allow_all = AccessPolicy::allow_all();
        assert!(allow_all.tools.allows("any"));

        let deny_all = AccessPolicy::deny_all();
        assert!(!deny_all.tools.allows("any"));
        assert!(!deny_all.agents.allows("any"));
    }

    #[test]
    fn test_execution_context_with_timeout() {
        let ctx = ExecutionContext::new().with_timeout(5000);
        assert_eq!(ctx.timeout_ms, 5000);

        let ctx2 = ExecutionContext::with_limits(5, 10000);
        assert_eq!(ctx2.max_call_depth, 5);
        assert_eq!(ctx2.timeout_ms, 10000);
    }

    #[test]
    fn test_execution_context_session_id() {
        let ctx = ExecutionContext::new().with_session_id("test-session-123".into());
        assert_eq!(ctx.session_id, Some("test-session-123".to_string()));
    }
}
