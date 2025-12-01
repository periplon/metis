use super::*;
use crate::config::{ErrorStrategy, WorkflowConfig, WorkflowStep};
use crate::domain::ToolPort;
use async_trait::async_trait;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Mock tool handler for testing
struct MockToolHandler {
    call_count: AtomicUsize,
    fail_on_tool: Option<String>,
}

impl MockToolHandler {
    fn new() -> Self {
        Self {
            call_count: AtomicUsize::new(0),
            fail_on_tool: None,
        }
    }

    fn with_failure(tool_name: &str) -> Self {
        Self {
            call_count: AtomicUsize::new(0),
            fail_on_tool: Some(tool_name.to_string()),
        }
    }
}

#[async_trait]
impl ToolPort for MockToolHandler {
    async fn execute_tool(&self, name: &str, args: Value) -> anyhow::Result<Value> {
        self.call_count.fetch_add(1, Ordering::SeqCst);

        if let Some(fail_tool) = &self.fail_on_tool {
            if name == fail_tool {
                return Err(anyhow::anyhow!("simulated failure for {}", name));
            }
        }

        // Return mock response based on tool name
        Ok(json!({
            "tool": name,
            "args": args,
            "success": true
        }))
    }

    async fn list_tools(&self) -> anyhow::Result<Vec<crate::domain::Tool>> {
        Ok(vec![])
    }
}

#[tokio::test]
async fn test_simple_workflow() {
    let handler = Arc::new(MockToolHandler::new());
    let engine = WorkflowEngine::new(handler.clone());

    let workflow = WorkflowConfig {
        name: "test_workflow".to_string(),
        description: "Test workflow".to_string(),
        input_schema: json!({}),
        output_schema: None,
        steps: vec![
            WorkflowStep {
                id: "step1".to_string(),
                tool: "tool_a".to_string(),
                args: Some(json!({"value": "{{ input.x }}"})),
                depends_on: vec![],
                condition: None,
                loop_over: None,
                loop_var: "item".to_string(),
                loop_concurrency: 1,
                on_error: ErrorStrategy::Fail,
            },
            WorkflowStep {
                id: "step2".to_string(),
                tool: "tool_b".to_string(),
                args: Some(json!({"prev": "{{ steps.step1.success }}"})),
                depends_on: vec![],
                condition: None,
                loop_over: None,
                loop_var: "item".to_string(),
                loop_concurrency: 1,
                on_error: ErrorStrategy::Fail,
            },
        ],
        on_error: ErrorStrategy::Fail,
        tags: vec![],
    };

    let input = json!({"x": 42});
    let result = engine.execute(&workflow, input).await.unwrap();

    assert!(result["success"].as_bool().unwrap());
    assert!(result["steps"]["step1"].is_object());
    assert!(result["steps"]["step2"].is_object());
}

#[tokio::test]
async fn test_workflow_with_condition() {
    let handler = Arc::new(MockToolHandler::new());
    let engine = WorkflowEngine::new(handler.clone());

    let workflow = WorkflowConfig {
        name: "conditional_workflow".to_string(),
        description: "Test conditional".to_string(),
        input_schema: json!({}),
        output_schema: None,
        steps: vec![
            WorkflowStep {
                id: "always_run".to_string(),
                tool: "tool_a".to_string(),
                args: None,
                depends_on: vec![],
                condition: None,
                loop_over: None,
                loop_var: "item".to_string(),
                loop_concurrency: 1,
                on_error: ErrorStrategy::Fail,
            },
            WorkflowStep {
                id: "skip_me".to_string(),
                tool: "tool_b".to_string(),
                args: None,
                depends_on: vec![],
                condition: Some("input.run_optional == true".to_string()),
                loop_over: None,
                loop_var: "item".to_string(),
                loop_concurrency: 1,
                on_error: ErrorStrategy::Fail,
            },
        ],
        on_error: ErrorStrategy::Fail,
        tags: vec![],
    };

    // With condition false
    let input = json!({"run_optional": false});
    let result = engine.execute(&workflow, input).await.unwrap();

    assert!(result["success"].as_bool().unwrap());
    assert!(result["steps"]["skip_me"]["skipped"].as_bool().unwrap_or(false));
}

#[tokio::test]
async fn test_workflow_with_loop() {
    let handler = Arc::new(MockToolHandler::new());
    let engine = WorkflowEngine::new(handler.clone());

    let workflow = WorkflowConfig {
        name: "loop_workflow".to_string(),
        description: "Test looping".to_string(),
        input_schema: json!({}),
        output_schema: None,
        steps: vec![WorkflowStep {
            id: "process_items".to_string(),
            tool: "process".to_string(),
            args: Some(json!({"item_id": "{{ item.id }}"})),
            depends_on: vec![],
            condition: None,
            loop_over: Some("input.items".to_string()),
            loop_var: "item".to_string(),
            loop_concurrency: 1,
            on_error: ErrorStrategy::Fail,
        }],
        on_error: ErrorStrategy::Fail,
        tags: vec![],
    };

    let input = json!({
        "items": [
            {"id": 1},
            {"id": 2},
            {"id": 3}
        ]
    });

    let result = engine.execute(&workflow, input).await.unwrap();

    assert!(result["success"].as_bool().unwrap());
    let loop_results = result["steps"]["process_items"].as_array().unwrap();
    assert_eq!(loop_results.len(), 3);
}

#[tokio::test]
async fn test_workflow_parallel_loop() {
    let handler = Arc::new(MockToolHandler::new());
    let engine = WorkflowEngine::new(handler.clone());

    let workflow = WorkflowConfig {
        name: "parallel_loop".to_string(),
        description: "Test parallel looping".to_string(),
        input_schema: json!({}),
        output_schema: None,
        steps: vec![WorkflowStep {
            id: "parallel_process".to_string(),
            tool: "process".to_string(),
            args: Some(json!({"item_id": "{{ item }}"})),
            depends_on: vec![],
            condition: None,
            loop_over: Some("input.ids".to_string()),
            loop_var: "item".to_string(),
            loop_concurrency: 3, // Process 3 at a time
            on_error: ErrorStrategy::Fail,
        }],
        on_error: ErrorStrategy::Fail,
        tags: vec![],
    };

    let input = json!({
        "ids": [1, 2, 3, 4, 5, 6]
    });

    let result = engine.execute(&workflow, input).await.unwrap();

    assert!(result["success"].as_bool().unwrap());
    let loop_results = result["steps"]["parallel_process"].as_array().unwrap();
    assert_eq!(loop_results.len(), 6);
}

#[tokio::test]
async fn test_workflow_error_continue() {
    let handler = Arc::new(MockToolHandler::with_failure("failing_tool"));
    let engine = WorkflowEngine::new(handler);

    let workflow = WorkflowConfig {
        name: "error_continue".to_string(),
        description: "Test error continue".to_string(),
        input_schema: json!({}),
        output_schema: None,
        steps: vec![
            WorkflowStep {
                id: "will_fail".to_string(),
                tool: "failing_tool".to_string(),
                args: None,
                depends_on: vec![],
                condition: None,
                loop_over: None,
                loop_var: "item".to_string(),
                loop_concurrency: 1,
                on_error: ErrorStrategy::Continue,
            },
            WorkflowStep {
                id: "should_run".to_string(),
                tool: "good_tool".to_string(),
                args: None,
                depends_on: vec![],
                condition: None,
                loop_over: None,
                loop_var: "item".to_string(),
                loop_concurrency: 1,
                on_error: ErrorStrategy::Fail,
            },
        ],
        on_error: ErrorStrategy::Continue,
        tags: vec![],
    };

    let result = engine.execute(&workflow, json!({})).await.unwrap();

    // Workflow completes but reports partial failure
    assert!(!result["success"].as_bool().unwrap());
    assert!(result["steps"]["should_run"].is_object());
}

#[tokio::test]
async fn test_workflow_error_fallback() {
    let handler = Arc::new(MockToolHandler::with_failure("failing_tool"));
    let engine = WorkflowEngine::new(handler);

    let workflow = WorkflowConfig {
        name: "error_fallback".to_string(),
        description: "Test error fallback".to_string(),
        input_schema: json!({}),
        output_schema: None,
        steps: vec![WorkflowStep {
            id: "with_fallback".to_string(),
            tool: "failing_tool".to_string(),
            args: None,
            depends_on: vec![],
            condition: None,
            loop_over: None,
            loop_var: "item".to_string(),
            loop_concurrency: 1,
            on_error: ErrorStrategy::Fallback {
                value: json!({"default": true}),
            },
        }],
        on_error: ErrorStrategy::Fallback {
            value: json!({"default": true}),
        },
        tags: vec![],
    };

    let result = engine.execute(&workflow, json!({})).await.unwrap();

    // Should use fallback value
    assert_eq!(result["steps"]["with_fallback"]["default"], true);
}

#[tokio::test]
async fn test_workflow_data_passing() {
    let handler = Arc::new(MockToolHandler::new());
    let engine = WorkflowEngine::new(handler);

    let workflow = WorkflowConfig {
        name: "data_passing".to_string(),
        description: "Test data passing between steps".to_string(),
        input_schema: json!({}),
        output_schema: None,
        steps: vec![
            WorkflowStep {
                id: "producer".to_string(),
                tool: "produce".to_string(),
                args: Some(json!({"input_value": "{{ input.value }}"})),
                depends_on: vec![],
                condition: None,
                loop_over: None,
                loop_var: "item".to_string(),
                loop_concurrency: 1,
                on_error: ErrorStrategy::Fail,
            },
            WorkflowStep {
                id: "consumer".to_string(),
                tool: "consume".to_string(),
                args: Some(json!({"from_producer": "{{ steps.producer.tool }}"})),
                depends_on: vec![],
                condition: None,
                loop_over: None,
                loop_var: "item".to_string(),
                loop_concurrency: 1,
                on_error: ErrorStrategy::Fail,
            },
        ],
        on_error: ErrorStrategy::Fail,
        tags: vec![],
    };

    let result = engine.execute(&workflow, json!({"value": "test"})).await.unwrap();

    assert!(result["success"].as_bool().unwrap());
    // Consumer should have received data from producer
    assert!(result["steps"]["consumer"]["args"]["from_producer"].is_string());
}

#[tokio::test]
async fn test_empty_loop() {
    let handler = Arc::new(MockToolHandler::new());
    let engine = WorkflowEngine::new(handler);

    let workflow = WorkflowConfig {
        name: "empty_loop".to_string(),
        description: "Test empty loop".to_string(),
        input_schema: json!({}),
        output_schema: None,
        steps: vec![WorkflowStep {
            id: "empty".to_string(),
            tool: "process".to_string(),
            args: None,
            depends_on: vec![],
            condition: None,
            loop_over: Some("input.items".to_string()),
            loop_var: "item".to_string(),
            loop_concurrency: 1,
            on_error: ErrorStrategy::Fail,
        }],
        on_error: ErrorStrategy::Fail,
        tags: vec![],
    };

    let result = engine.execute(&workflow, json!({"items": []})).await.unwrap();

    assert!(result["success"].as_bool().unwrap());
    let loop_results = result["steps"]["empty"].as_array().unwrap();
    assert_eq!(loop_results.len(), 0);
}
