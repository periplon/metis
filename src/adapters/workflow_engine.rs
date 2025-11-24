//! Workflow Engine - Executes multi-step workflows as MCP tools
//!
//! The workflow engine enables complex orchestration patterns including:
//! - Sequential step execution with data passing
//! - Conditional branching using Rhai expressions
//! - Loop iteration over arrays (sequential or parallel)
//! - Error handling strategies (fail, continue, retry, fallback)

use crate::config::{ErrorStrategy, WorkflowConfig, WorkflowStep};
use crate::domain::ToolPort;
use anyhow::{anyhow, Result};
use futures::stream::{self, StreamExt};
use rhai::{Dynamic, Engine as RhaiEngine, Scope};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tera::{Context, Tera};
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

/// Result of a single workflow step execution
#[derive(Debug, Clone)]
pub struct StepResult {
    pub step_id: String,
    pub success: bool,
    pub result: Value,
    pub error: Option<String>,
}

/// Context passed through workflow execution
#[derive(Debug, Clone)]
pub struct WorkflowContext {
    /// Original input to the workflow
    pub input: Value,
    /// Results from completed steps (keyed by step_id)
    pub steps: HashMap<String, Value>,
}

impl WorkflowContext {
    pub fn new(input: Value) -> Self {
        Self {
            input,
            steps: HashMap::new(),
        }
    }

    /// Convert context to a JSON value for template rendering
    pub fn to_value(&self) -> Value {
        json!({
            "input": self.input,
            "steps": self.steps
        })
    }

    /// Convert context to Rhai scope for condition evaluation
    pub fn to_rhai_scope(&self) -> Scope<'static> {
        let mut scope = Scope::new();
        scope.push("input", json_to_dynamic(&self.input));
        scope.push("steps", json_to_dynamic(&json!(self.steps)));
        scope
    }
}

/// Workflow execution engine
pub struct WorkflowEngine {
    tool_handler: Arc<dyn ToolPort>,
    rhai_engine: RhaiEngine,
}

impl WorkflowEngine {
    pub fn new(tool_handler: Arc<dyn ToolPort>) -> Self {
        let mut rhai_engine = RhaiEngine::new();
        // Register JSON-like access functions
        rhai_engine.set_max_expr_depths(64, 64);
        Self {
            tool_handler,
            rhai_engine,
        }
    }

    /// Execute a workflow with the given input
    pub async fn execute(&self, workflow: &WorkflowConfig, input: Value) -> Result<Value> {
        let context = Arc::new(RwLock::new(WorkflowContext::new(input)));
        let mut results: Vec<StepResult> = Vec::new();

        for step in &workflow.steps {
            let step_result = self
                .execute_step(step, context.clone(), &workflow.on_error)
                .await;

            match step_result {
                Ok(result) => {
                    // Store result in context for subsequent steps
                    let mut ctx = context.write().await;
                    ctx.steps.insert(step.id.clone(), result.result.clone());
                    results.push(result);
                }
                Err(e) => {
                    // Handle error based on workflow-level strategy
                    match &workflow.on_error {
                        ErrorStrategy::Fail => return Err(e),
                        ErrorStrategy::Continue => {
                            let mut ctx = context.write().await;
                            ctx.steps.insert(
                                step.id.clone(),
                                json!({
                                    "error": e.to_string(),
                                    "success": false
                                }),
                            );
                            results.push(StepResult {
                                step_id: step.id.clone(),
                                success: false,
                                result: Value::Null,
                                error: Some(e.to_string()),
                            });
                        }
                        ErrorStrategy::Fallback { value } => {
                            let mut ctx = context.write().await;
                            ctx.steps.insert(step.id.clone(), value.clone());
                            results.push(StepResult {
                                step_id: step.id.clone(),
                                success: false,
                                result: value.clone(),
                                error: Some(e.to_string()),
                            });
                        }
                        ErrorStrategy::Retry { .. } => {
                            // Retry is handled at step level
                            return Err(e);
                        }
                    }
                }
            }
        }

        // Return final context with all step results
        let ctx = context.read().await;
        Ok(json!({
            "success": results.iter().all(|r| r.success),
            "steps": ctx.steps,
            "results": results.iter().map(|r| json!({
                "step_id": r.step_id,
                "success": r.success,
                "error": r.error
            })).collect::<Vec<_>>()
        }))
    }

    /// Execute a single workflow step
    async fn execute_step(
        &self,
        step: &WorkflowStep,
        context: Arc<RwLock<WorkflowContext>>,
        workflow_error_strategy: &ErrorStrategy,
    ) -> Result<StepResult> {
        // Check condition if present
        if let Some(condition) = &step.condition {
            let ctx = context.read().await;
            if !self.evaluate_condition(condition, &ctx)? {
                return Ok(StepResult {
                    step_id: step.id.clone(),
                    success: true,
                    result: json!({"skipped": true, "reason": "condition not met"}),
                    error: None,
                });
            }
        }

        // Handle loop if present
        if let Some(loop_expr) = &step.loop_over {
            return self
                .execute_loop(step, loop_expr, context, workflow_error_strategy)
                .await;
        }

        // Execute single step with error handling
        self.execute_single_step(step, context, workflow_error_strategy)
            .await
    }

    /// Execute a step that loops over an array
    async fn execute_loop(
        &self,
        step: &WorkflowStep,
        loop_expr: &str,
        context: Arc<RwLock<WorkflowContext>>,
        _workflow_error_strategy: &ErrorStrategy,
    ) -> Result<StepResult> {
        // Evaluate loop expression to get array
        let items = {
            let ctx = context.read().await;
            self.evaluate_loop_expression(loop_expr, &ctx)?
        };

        let items_array = items.as_array().ok_or_else(|| {
            anyhow!(
                "loop_over expression must evaluate to an array, got: {:?}",
                items
            )
        })?;

        if items_array.is_empty() {
            return Ok(StepResult {
                step_id: step.id.clone(),
                success: true,
                result: json!([]),
                error: None,
            });
        }

        let concurrency = step.loop_concurrency.max(1) as usize;
        let loop_var = &step.loop_var;

        // Execute loop iterations
        let results: Vec<Result<Value>> = if concurrency == 1 {
            // Sequential execution
            let mut results = Vec::new();
            for (index, item) in items_array.iter().enumerate() {
                let result = self
                    .execute_loop_iteration(step, item, index, loop_var, context.clone())
                    .await;
                results.push(result);
            }
            results
        } else {
            // Parallel execution with concurrency limit
            // Clone items to avoid lifetime issues with async
            let items_with_index: Vec<_> = items_array
                .iter()
                .enumerate()
                .map(|(i, v)| (i, v.clone()))
                .collect();
            stream::iter(items_with_index)
                .map(|(index, item)| {
                    let step = step.clone();
                    let loop_var = loop_var.clone();
                    let context = context.clone();
                    async move {
                        self.execute_loop_iteration(&step, &item, index, &loop_var, context)
                            .await
                    }
                })
                .buffer_unordered(concurrency)
                .collect()
                .await
        };

        // Aggregate results
        let mut loop_results = Vec::new();
        let mut all_success = true;
        let mut errors = Vec::new();

        for (i, result) in results.into_iter().enumerate() {
            match result {
                Ok(value) => loop_results.push(value),
                Err(e) => {
                    all_success = false;
                    let error_msg = format!("iteration {}: {}", i, e);
                    errors.push(error_msg.clone());

                    match &step.on_error {
                        ErrorStrategy::Fail => return Err(anyhow!(error_msg)),
                        ErrorStrategy::Continue => {
                            loop_results.push(json!({"error": e.to_string(), "success": false}));
                        }
                        ErrorStrategy::Fallback { value } => {
                            loop_results.push(value.clone());
                        }
                        ErrorStrategy::Retry { .. } => {
                            // For loops, retry is not supported at iteration level
                            loop_results.push(json!({"error": e.to_string(), "success": false}));
                        }
                    }
                }
            }
        }

        Ok(StepResult {
            step_id: step.id.clone(),
            success: all_success,
            result: json!(loop_results),
            error: if errors.is_empty() {
                None
            } else {
                Some(errors.join("; "))
            },
        })
    }

    /// Execute a single loop iteration
    async fn execute_loop_iteration(
        &self,
        step: &WorkflowStep,
        item: &Value,
        index: usize,
        loop_var: &str,
        context: Arc<RwLock<WorkflowContext>>,
    ) -> Result<Value> {
        // Render args with loop variable in context
        let args = if let Some(args_template) = &step.args {
            let ctx = context.read().await;
            let mut context_value = ctx.to_value();
            if let Some(obj) = context_value.as_object_mut() {
                obj.insert(loop_var.to_string(), item.clone());
                obj.insert("index".to_string(), json!(index));
            }
            self.render_args(args_template, &context_value)?
        } else {
            item.clone()
        };

        // Execute tool
        self.tool_handler.execute_tool(&step.tool, args).await
    }

    /// Execute a single step (non-loop)
    async fn execute_single_step(
        &self,
        step: &WorkflowStep,
        context: Arc<RwLock<WorkflowContext>>,
        _workflow_error_strategy: &ErrorStrategy,
    ) -> Result<StepResult> {
        let error_strategy = &step.on_error;

        // Render arguments
        let args = {
            let ctx = context.read().await;
            if let Some(args_template) = &step.args {
                self.render_args(args_template, &ctx.to_value())?
            } else {
                Value::Null
            }
        };

        // Execute with retry if configured
        match error_strategy {
            ErrorStrategy::Retry {
                max_attempts,
                delay_ms,
            } => {
                let mut last_error = None;
                for attempt in 0..*max_attempts {
                    match self.tool_handler.execute_tool(&step.tool, args.clone()).await {
                        Ok(result) => {
                            return Ok(StepResult {
                                step_id: step.id.clone(),
                                success: true,
                                result,
                                error: None,
                            });
                        }
                        Err(e) => {
                            last_error = Some(e);
                            if attempt < max_attempts - 1 {
                                // Exponential backoff
                                let delay = *delay_ms * 2u64.pow(attempt);
                                sleep(Duration::from_millis(delay)).await;
                            }
                        }
                    }
                }
                Err(last_error.unwrap_or_else(|| anyhow!("retry failed")))
            }
            _ => {
                // Single execution
                let result = self
                    .tool_handler
                    .execute_tool(&step.tool, args.clone())
                    .await?;
                Ok(StepResult {
                    step_id: step.id.clone(),
                    success: true,
                    result,
                    error: None,
                })
            }
        }
    }

    /// Evaluate a Rhai condition expression
    fn evaluate_condition(&self, condition: &str, context: &WorkflowContext) -> Result<bool> {
        let mut scope = context.to_rhai_scope();
        let result: Dynamic = self
            .rhai_engine
            .eval_with_scope(&mut scope, condition)
            .map_err(|e| anyhow!("condition evaluation failed: {}", e))?;

        result
            .as_bool()
            .map_err(|_| anyhow!("condition must evaluate to boolean"))
    }

    /// Evaluate a Rhai expression that returns an array for looping
    fn evaluate_loop_expression(&self, expr: &str, context: &WorkflowContext) -> Result<Value> {
        let mut scope = context.to_rhai_scope();
        let result: Dynamic = self
            .rhai_engine
            .eval_with_scope(&mut scope, expr)
            .map_err(|e| anyhow!("loop expression evaluation failed: {}", e))?;

        dynamic_to_json(&result)
    }

    /// Render step arguments using Tera templates
    fn render_args(&self, args: &Value, context: &Value) -> Result<Value> {
        match args {
            Value::String(s) => {
                // Render string template
                let rendered = self.render_template(s, context)?;
                // Try to parse as JSON, otherwise return as string
                match serde_json::from_str(&rendered) {
                    Ok(v) => Ok(v),
                    Err(_) => Ok(Value::String(rendered)),
                }
            }
            Value::Object(obj) => {
                // Recursively render object values
                let mut result = Map::new();
                for (k, v) in obj {
                    result.insert(k.clone(), self.render_args(v, context)?);
                }
                Ok(Value::Object(result))
            }
            Value::Array(arr) => {
                // Recursively render array items
                let result: Result<Vec<Value>> =
                    arr.iter().map(|v| self.render_args(v, context)).collect();
                Ok(Value::Array(result?))
            }
            _ => Ok(args.clone()),
        }
    }

    /// Render a single template string
    fn render_template(&self, template: &str, context: &Value) -> Result<String> {
        let mut tera = Tera::default();
        tera.add_raw_template("template", template)
            .map_err(|e| anyhow!("template parse error: {}", e))?;

        let mut tera_context = Context::new();
        if let Some(obj) = context.as_object() {
            for (k, v) in obj {
                tera_context.insert(k, v);
            }
        }

        tera.render("template", &tera_context)
            .map_err(|e| anyhow!("template render error: {}", e))
    }
}

/// Convert JSON Value to Rhai Dynamic
fn json_to_dynamic(value: &Value) -> Dynamic {
    match value {
        Value::Null => Dynamic::UNIT,
        Value::Bool(b) => Dynamic::from(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Dynamic::from(i)
            } else if let Some(f) = n.as_f64() {
                Dynamic::from(f)
            } else {
                Dynamic::UNIT
            }
        }
        Value::String(s) => Dynamic::from(s.clone()),
        Value::Array(arr) => {
            let vec: Vec<Dynamic> = arr.iter().map(json_to_dynamic).collect();
            Dynamic::from(vec)
        }
        Value::Object(obj) => {
            let map: rhai::Map = obj
                .iter()
                .map(|(k, v)| (k.clone().into(), json_to_dynamic(v)))
                .collect();
            Dynamic::from(map)
        }
    }
}

/// Convert Rhai Dynamic to JSON Value
fn dynamic_to_json(value: &Dynamic) -> Result<Value> {
    if value.is_unit() {
        Ok(Value::Null)
    } else if let Some(b) = value.as_bool().ok() {
        Ok(Value::Bool(b))
    } else if let Some(i) = value.as_int().ok() {
        Ok(json!(i))
    } else if let Some(f) = value.as_float().ok() {
        Ok(json!(f))
    } else if let Some(s) = value.clone().into_string().ok() {
        Ok(Value::String(s))
    } else if value.is_array() {
        let arr = value.clone().into_array().unwrap();
        let result: Result<Vec<Value>> = arr.iter().map(dynamic_to_json).collect();
        Ok(Value::Array(result?))
    } else if value.is_map() {
        let map = value.clone().into_typed_array::<(String, Dynamic)>();
        if let Ok(pairs) = map {
            let mut obj = Map::new();
            for (k, v) in pairs {
                obj.insert(k, dynamic_to_json(&v)?);
            }
            Ok(Value::Object(obj))
        } else {
            // Try as Rhai Map
            let map = value.clone().cast::<rhai::Map>();
            let mut obj = Map::new();
            for (k, v) in map {
                obj.insert(k.to_string(), dynamic_to_json(&v)?);
            }
            Ok(Value::Object(obj))
        }
    } else {
        Ok(Value::String(value.to_string()))
    }
}

#[cfg(test)]
#[path = "workflow_engine_test.rs"]
mod tests;
