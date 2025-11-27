//! Workflow Step Editor Component
//!
//! A visual editor for creating and editing workflow steps with guided UI.

use leptos::prelude::*;
use leptos::web_sys;
use wasm_bindgen::JsCast;
use crate::types::{WorkflowStep, ErrorStrategy};

/// UI representation of a workflow step for editing
#[derive(Clone, Debug, Default)]
pub struct StepData {
    /// Internal unique key for stable iteration (not user-editable)
    pub key: u32,
    pub id: String,
    pub tool: String,
    pub args_json: String,
    /// Step IDs that must complete before this step can execute (DAG dependencies)
    pub depends_on: Vec<String>,
    pub condition: String,
    pub loop_over: String,
    pub loop_var: String,
    pub loop_concurrency: u32,
    pub error_strategy: String,
    pub retry_max_attempts: u32,
    pub retry_delay_ms: u64,
    pub fallback_value: String,
}

static NEXT_STEP_KEY: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

fn next_step_key() -> u32 {
    NEXT_STEP_KEY.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

impl StepData {
    pub fn new() -> Self {
        Self {
            key: next_step_key(),
            id: String::new(),
            tool: String::new(),
            args_json: String::new(),
            depends_on: Vec::new(),
            condition: String::new(),
            loop_over: String::new(),
            loop_var: "item".to_string(),
            loop_concurrency: 1,
            error_strategy: "fail".to_string(),
            retry_max_attempts: 3,
            retry_delay_ms: 1000,
            fallback_value: "null".to_string(),
        }
    }
}

/// Convert StepData to WorkflowStep
pub fn step_data_to_workflow_step(data: &StepData) -> Result<WorkflowStep, String> {
    if data.id.is_empty() {
        return Err("Step ID is required".to_string());
    }
    if data.tool.is_empty() {
        return Err(format!("Tool is required for step '{}'", data.id));
    }

    let args = if data.args_json.trim().is_empty() {
        None
    } else {
        Some(serde_json::from_str(&data.args_json)
            .map_err(|e| format!("Invalid args JSON for step '{}': {}", data.id, e))?)
    };

    let condition = if data.condition.trim().is_empty() {
        None
    } else {
        Some(data.condition.clone())
    };

    let loop_over = if data.loop_over.trim().is_empty() {
        None
    } else {
        Some(data.loop_over.clone())
    };

    let on_error = match data.error_strategy.as_str() {
        "continue" => ErrorStrategy::Continue,
        "retry" => ErrorStrategy::Retry {
            max_attempts: data.retry_max_attempts,
            delay_ms: data.retry_delay_ms,
        },
        "fallback" => {
            let value = serde_json::from_str(&data.fallback_value)
                .map_err(|e| format!("Invalid fallback JSON for step '{}': {}", data.id, e))?;
            ErrorStrategy::Fallback { value }
        }
        _ => ErrorStrategy::Fail,
    };

    Ok(WorkflowStep {
        id: data.id.clone(),
        tool: data.tool.clone(),
        args,
        depends_on: data.depends_on.clone(),
        condition,
        loop_over,
        loop_var: data.loop_var.clone(),
        loop_concurrency: data.loop_concurrency,
        on_error,
    })
}

/// Convert WorkflowStep to StepData for editing
pub fn workflow_step_to_step_data(step: &WorkflowStep) -> StepData {
    let (error_strategy, retry_max_attempts, retry_delay_ms, fallback_value) = match &step.on_error {
        ErrorStrategy::Fail => ("fail".to_string(), 3, 1000, "null".to_string()),
        ErrorStrategy::Continue => ("continue".to_string(), 3, 1000, "null".to_string()),
        ErrorStrategy::Retry { max_attempts, delay_ms } => {
            ("retry".to_string(), *max_attempts, *delay_ms, "null".to_string())
        }
        ErrorStrategy::Fallback { value } => {
            ("fallback".to_string(), 3, 1000, serde_json::to_string_pretty(value).unwrap_or_default())
        }
    };

    StepData {
        key: next_step_key(),
        id: step.id.clone(),
        tool: step.tool.clone(),
        args_json: step.args.as_ref()
            .map(|a| serde_json::to_string_pretty(a).unwrap_or_default())
            .unwrap_or_default(),
        depends_on: step.depends_on.clone(),
        condition: step.condition.clone().unwrap_or_default(),
        loop_over: step.loop_over.clone().unwrap_or_default(),
        loop_var: step.loop_var.clone(),
        loop_concurrency: step.loop_concurrency,
        error_strategy,
        retry_max_attempts,
        retry_delay_ms,
        fallback_value,
    }
}

/// Convert Vec<StepData> to Vec<WorkflowStep>
pub fn steps_data_to_workflow_steps(steps: &[StepData]) -> Result<Vec<WorkflowStep>, String> {
    steps.iter().map(step_data_to_workflow_step).collect()
}

/// Convert Vec<WorkflowStep> to Vec<StepData>
pub fn workflow_steps_to_steps_data(steps: &[WorkflowStep]) -> Vec<StepData> {
    steps.iter().map(workflow_step_to_step_data).collect()
}

/// Main workflow steps editor component
#[component]
pub fn WorkflowStepsEditor(
    steps: ReadSignal<Vec<StepData>>,
    set_steps: WriteSignal<Vec<StepData>>,
    #[prop(default = Vec::new())] available_tools: Vec<String>,
) -> impl IntoView {
    let tools = StoredValue::new(available_tools);

    let add_step = move |_| {
        set_steps.update(|s| {
            let mut new_step = StepData::new();
            new_step.id = format!("step{}", s.len() + 1);
            s.push(new_step);
        });
    };

    view! {
        <div class="space-y-4">
            <div class="flex justify-between items-center">
                <label class="block text-sm font-medium text-gray-700">"Workflow Steps"</label>
                <button
                    type="button"
                    class="px-3 py-1 text-sm bg-orange-500 text-white rounded hover:bg-orange-600 flex items-center gap-1"
                    on:click=add_step
                >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/>
                    </svg>
                    "Add Step"
                </button>
            </div>

            // Visual step flow
            <Show
                when=move || !steps.get().is_empty()
                fallback=|| view! {
                    <div class="text-center py-8 bg-gray-50 rounded-lg border-2 border-dashed border-gray-300">
                        <svg class="mx-auto h-12 w-12 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"/>
                        </svg>
                        <p class="mt-2 text-sm text-gray-500">"No steps yet. Click \"Add Step\" to create your first step."</p>
                    </div>
                }
            >
                <div class="space-y-3">
                    <For
                        each=move || {
                            steps.get().into_iter().enumerate().map(|(idx, step)| (idx, step.key)).collect::<Vec<_>>()
                        }
                        key=|(_, key)| *key
                        children=move |(idx, _)| {
                            view! {
                                <StepCard
                                    index=idx
                                    steps=steps
                                    set_steps=set_steps
                                    available_tools=tools.get_value()
                                />
                            }
                        }
                    />
                </div>
            </Show>

            // Step flow preview (DAG visualization)
            {move || {
                let current_steps = steps.get();
                if current_steps.len() > 1 {
                    // Build a map of step_id -> index for lookup
                    let step_id_to_idx: std::collections::HashMap<String, usize> = current_steps
                        .iter()
                        .enumerate()
                        .filter(|(_, s)| !s.id.is_empty())
                        .map(|(i, s)| (s.id.clone(), i))
                        .collect();

                    // Group steps by their "depth" (max distance from root)
                    let mut depths: Vec<usize> = vec![0; current_steps.len()];
                    for (i, step) in current_steps.iter().enumerate() {
                        if step.depends_on.is_empty() {
                            depths[i] = 0;
                        } else {
                            let max_dep_depth = step.depends_on.iter()
                                .filter_map(|dep| step_id_to_idx.get(dep))
                                .map(|&idx| depths[idx])
                                .max()
                                .unwrap_or(0);
                            depths[i] = max_dep_depth + 1;
                        }
                    }

                    // Determine max depth
                    let max_depth = *depths.iter().max().unwrap_or(&0);

                    view! {
                        <div class="mt-4 p-3 bg-gray-50 rounded-lg">
                            <div class="text-xs font-medium text-gray-500 uppercase mb-2">"DAG Flow Preview"</div>
                            <div class="space-y-2">
                                {(0..=max_depth).map(|depth| {
                                    let steps_at_depth: Vec<(usize, &StepData)> = current_steps
                                        .iter()
                                        .enumerate()
                                        .filter(|(i, _)| depths[*i] == depth)
                                        .collect();

                                    view! {
                                        <div class="flex items-center gap-2">
                                            <span class="text-xs text-gray-400 w-8">{format!("L{}", depth)}</span>
                                            <div class="flex flex-wrap gap-2">
                                                {steps_at_depth.iter().map(|(_, step)| {
                                                    let has_condition = !step.condition.is_empty();
                                                    let has_loop = !step.loop_over.is_empty();
                                                    let has_deps = !step.depends_on.is_empty();
                                                    let deps_display = step.depends_on.join(", ");

                                                    view! {
                                                        <div class="flex flex-col items-center">
                                                            {if has_deps {
                                                                view! {
                                                                    <div class="text-xs text-green-600 mb-0.5 font-mono">
                                                                        {format!("← {}", deps_display)}
                                                                    </div>
                                                                }.into_any()
                                                            } else {
                                                                view! { <span></span> }.into_any()
                                                            }}
                                                            <div class=format!(
                                                                "px-2 py-1 rounded text-xs font-mono {}",
                                                                if has_deps { "bg-green-100 border border-green-300" }
                                                                else if has_condition { "bg-yellow-100 border border-yellow-300" }
                                                                else if has_loop { "bg-purple-100 border border-purple-300" }
                                                                else { "bg-orange-100 border border-orange-300" }
                                                            )>
                                                                {if has_loop { "⟳ " } else { "" }}
                                                                {if has_condition { "? " } else { "" }}
                                                                <span class="font-semibold">{step.id.clone()}</span>
                                                                {if !step.tool.is_empty() {
                                                                    format!(": {}", step.tool)
                                                                } else {
                                                                    String::new()
                                                                }}
                                                            </div>
                                                        </div>
                                                    }
                                                }).collect::<Vec<_>>()}
                                            </div>
                                        </div>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                            <div class="mt-2 text-xs text-gray-500">
                                "Steps at the same level (L0, L1, ...) can run in parallel"
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }
            }}
        </div>
    }
}

/// Individual step card component - reads from signal to avoid re-renders
#[component]
fn StepCard(
    index: usize,
    steps: ReadSignal<Vec<StepData>>,
    set_steps: WriteSignal<Vec<StepData>>,
    available_tools: Vec<String>,
) -> impl IntoView {
    let (expanded, set_expanded) = signal(true);

    // Derived signals for step fields
    let step_id = move || steps.get().get(index).map(|s| s.id.clone()).unwrap_or_default();
    let step_tool = move || steps.get().get(index).map(|s| s.tool.clone()).unwrap_or_default();
    let step_depends_on = move || steps.get().get(index).map(|s| s.depends_on.clone()).unwrap_or_default();
    let step_condition = move || steps.get().get(index).map(|s| s.condition.clone()).unwrap_or_default();
    let step_loop_over = move || steps.get().get(index).map(|s| s.loop_over.clone()).unwrap_or_default();
    let step_loop_var = move || steps.get().get(index).map(|s| s.loop_var.clone()).unwrap_or_default();
    let step_loop_concurrency = move || steps.get().get(index).map(|s| s.loop_concurrency).unwrap_or(1);
    let step_error_strategy = move || steps.get().get(index).map(|s| s.error_strategy.clone()).unwrap_or_else(|| "fail".to_string());
    let step_args_json = move || steps.get().get(index).map(|s| s.args_json.clone()).unwrap_or_default();
    let step_retry_max = move || steps.get().get(index).map(|s| s.retry_max_attempts).unwrap_or(3);
    let step_retry_delay = move || steps.get().get(index).map(|s| s.retry_delay_ms).unwrap_or(1000);
    let step_fallback = move || steps.get().get(index).map(|s| s.fallback_value.clone()).unwrap_or_else(|| "null".to_string());
    let total_steps = move || steps.get().len();

    // Get available steps for dependency selection (all steps except current one)
    let available_deps = move || {
        steps.get()
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != index)
            .map(|(_, s)| s.id.clone())
            .filter(|id| !id.is_empty())
            .collect::<Vec<_>>()
    };

    let has_advanced = move || {
        !step_depends_on().is_empty() || !step_condition().is_empty() || !step_loop_over().is_empty() || step_error_strategy() != "fail"
    };

    view! {
        <div class="border border-gray-200 rounded-lg bg-white shadow-sm">
            // Step header
            <div class="flex items-center justify-between p-3 bg-gray-50 rounded-t-lg border-b border-gray-200">
                <div class="flex items-center gap-3">
                    // Step number badge
                    <span class="flex items-center justify-center w-7 h-7 rounded-full bg-orange-500 text-white text-sm font-medium">
                        {index + 1}
                    </span>
                    // Step ID and tool preview
                    <div>
                        <span class="font-medium text-gray-900">{move || step_id()}</span>
                        {move || {
                            let tool = step_tool();
                            if !tool.is_empty() {
                                view! {
                                    <span class="ml-2 text-sm text-gray-500">
                                        "→ "
                                        <span class="font-mono text-orange-600">{tool.clone()}</span>
                                    </span>
                                }.into_any()
                            } else {
                                view! { <span class="ml-2 text-sm text-red-500 italic">"(no tool selected)"</span> }.into_any()
                            }
                        }}
                    </div>
                    // Feature badges
                    {move || if has_advanced() {
                        view! {
                            <div class="flex gap-1">
                                {move || {
                                    let deps = step_depends_on();
                                    if !deps.is_empty() {
                                        view! { <span class="px-1.5 py-0.5 text-xs bg-green-100 text-green-700 rounded">{format!("{} dep{}", deps.len(), if deps.len() > 1 { "s" } else { "" })}</span> }.into_any()
                                    } else {
                                        view! { <span></span> }.into_any()
                                    }
                                }}
                                {move || if !step_condition().is_empty() {
                                    view! { <span class="px-1.5 py-0.5 text-xs bg-yellow-100 text-yellow-700 rounded">"conditional"</span> }.into_any()
                                } else {
                                    view! { <span></span> }.into_any()
                                }}
                                {move || if !step_loop_over().is_empty() {
                                    view! { <span class="px-1.5 py-0.5 text-xs bg-purple-100 text-purple-700 rounded">"loop"</span> }.into_any()
                                } else {
                                    view! { <span></span> }.into_any()
                                }}
                                {move || if step_error_strategy() != "fail" {
                                    view! { <span class="px-1.5 py-0.5 text-xs bg-blue-100 text-blue-700 rounded">{step_error_strategy()}</span> }.into_any()
                                } else {
                                    view! { <span></span> }.into_any()
                                }}
                            </div>
                        }.into_any()
                    } else {
                        view! { <span></span> }.into_any()
                    }}
                </div>
                <div class="flex items-center gap-1">
                    // Reorder buttons
                    {
                        let is_first = move || index == 0;
                        let is_last = move || index + 1 >= total_steps();
                        view! {
                            <button
                                type="button"
                                class="p-1 text-gray-400 hover:text-gray-600 disabled:opacity-30 disabled:cursor-not-allowed"
                                disabled=is_first
                                on:click=move |_| {
                                    if index > 0 {
                                        set_steps.update(|s| s.swap(index, index - 1));
                                    }
                                }
                            >
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 15l7-7 7 7"/>
                                </svg>
                            </button>
                            <button
                                type="button"
                                class="p-1 text-gray-400 hover:text-gray-600 disabled:opacity-30 disabled:cursor-not-allowed"
                                disabled=is_last
                                on:click=move |_| {
                                    let ts = total_steps();
                                    if index < ts - 1 {
                                        set_steps.update(|s| s.swap(index, index + 1));
                                    }
                                }
                            >
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"/>
                                </svg>
                            </button>
                        }
                    }
                    // Expand/collapse
                    <button
                        type="button"
                        class="p-1 text-gray-400 hover:text-gray-600 ml-2"
                        on:click=move |_| set_expanded.update(|e| *e = !*e)
                    >
                        {move || if expanded.get() {
                            view! {
                                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"/>
                                </svg>
                            }
                        } else {
                            view! {
                                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"/>
                                </svg>
                            }
                        }}
                    </button>
                    // Delete button
                    <button
                        type="button"
                        class="p-1 text-red-400 hover:text-red-600 ml-2"
                        on:click=move |_| {
                            set_steps.update(|s| { s.remove(index); });
                        }
                    >
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/>
                        </svg>
                    </button>
                </div>
            </div>

            // Step content (expandable)
            <Show when=move || expanded.get()>
                <div class="p-4 space-y-4">
                    // Basic fields row
                    <div class="grid grid-cols-2 gap-4">
                        // Step ID
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"Step ID *"</label>
                            <input
                                type="text"
                                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-orange-500 text-sm"
                                placeholder="step1"
                                prop:value=step_id
                                on:input=move |ev| {
                                    let target = ev.target().unwrap();
                                    let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                    set_steps.update(|s| {
                                        if let Some(st) = s.get_mut(index) {
                                            st.id = input.value();
                                        }
                                    });
                                }
                            />
                            <p class="mt-1 text-xs text-gray-500">"Unique identifier for this step"</p>
                        </div>

                        // Tool selection
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"Tool *"</label>
                            {if available_tools.is_empty() {
                                view! {
                                    <input
                                        type="text"
                                        class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-orange-500 text-sm font-mono"
                                        placeholder="tool-name"
                                        prop:value=step_tool
                                        on:input=move |ev| {
                                            let target = ev.target().unwrap();
                                            let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                            set_steps.update(|s| {
                                                if let Some(st) = s.get_mut(index) {
                                                    st.tool = input.value();
                                                }
                                            });
                                        }
                                    />
                                }.into_any()
                            } else {
                                let tools_list = available_tools.clone();
                                view! {
                                    <select
                                        class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-orange-500 text-sm"
                                        prop:value=step_tool
                                        on:change=move |ev| {
                                            let target = ev.target().unwrap();
                                            let select: web_sys::HtmlSelectElement = target.dyn_into().unwrap();
                                            set_steps.update(|s| {
                                                if let Some(st) = s.get_mut(index) {
                                                    st.tool = select.value();
                                                }
                                            });
                                        }
                                    >
                                        <option value="">"-- Select Tool --"</option>
                                        {tools_list.iter().map(|t| {
                                            view! {
                                                <option value=t.clone()>{t.clone()}</option>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </select>
                                }.into_any()
                            }}
                            <p class="mt-1 text-xs text-gray-500">"Tool to execute in this step"</p>
                        </div>
                    </div>

                    // Arguments
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">"Arguments (JSON)"</label>
                        <textarea
                            rows=3
                            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-orange-500 text-sm font-mono"
                            placeholder=r#"{"key": "value"}"#
                            prop:value=step_args_json
                            on:input=move |ev| {
                                let target = ev.target().unwrap();
                                let textarea: web_sys::HtmlTextAreaElement = target.dyn_into().unwrap();
                                set_steps.update(|s| {
                                    if let Some(st) = s.get_mut(index) {
                                        st.args_json = textarea.value();
                                    }
                                });
                            }
                        />
                        <p class="mt-1 text-xs text-gray-500">
                            "Use "
                            <code class="bg-gray-100 px-1 rounded">"{{$.input.field}}"</code>
                            " for workflow inputs, "
                            <code class="bg-gray-100 px-1 rounded">"{{$.steps.stepId.output}}"</code>
                            " for previous step outputs"
                        </p>
                    </div>

                    // Advanced options section
                    <details class="border border-gray-200 rounded-lg">
                        <summary class="px-4 py-2 cursor-pointer text-sm font-medium text-gray-700 bg-gray-50 rounded-t-lg hover:bg-gray-100">
                            "Advanced Options"
                        </summary>
                        <div class="p-4 space-y-4">
                            // Dependencies (DAG)
                            <div class="p-3 bg-green-50 rounded-lg border border-green-200">
                                <label class="block text-sm font-medium text-green-700 mb-2">"Dependencies (DAG)"</label>
                                {move || {
                                    let deps = available_deps();
                                    let current_deps = step_depends_on();
                                    if deps.is_empty() {
                                        view! {
                                            <p class="text-sm text-gray-500 italic">"No other steps available. Add more steps to create dependencies."</p>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <div class="space-y-2">
                                                <p class="text-xs text-gray-500 mb-2">"Select steps that must complete before this step runs:"</p>
                                                <div class="flex flex-wrap gap-2">
                                                    {deps.iter().map(|dep_id| {
                                                        let dep_id_clone = dep_id.clone();
                                                        let dep_id_display = dep_id.clone();
                                                        let is_checked = current_deps.contains(dep_id);
                                                        view! {
                                                            <label class=format!(
                                                                "inline-flex items-center px-2 py-1 rounded border cursor-pointer transition-colors {}",
                                                                if is_checked {
                                                                    "bg-green-100 border-green-400 text-green-800"
                                                                } else {
                                                                    "bg-white border-gray-300 text-gray-700 hover:bg-gray-50"
                                                                }
                                                            )>
                                                                <input
                                                                    type="checkbox"
                                                                    class="mr-1.5 rounded border-gray-300 text-green-600 focus:ring-green-500"
                                                                    prop:checked=is_checked
                                                                    on:change=move |ev| {
                                                                        let target = ev.target().unwrap();
                                                                        let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                                                        let checked = input.checked();
                                                                        let dep = dep_id_clone.clone();
                                                                        set_steps.update(|s| {
                                                                            if let Some(st) = s.get_mut(index) {
                                                                                if checked {
                                                                                    if !st.depends_on.contains(&dep) {
                                                                                        st.depends_on.push(dep);
                                                                                    }
                                                                                } else {
                                                                                    st.depends_on.retain(|d| d != &dep);
                                                                                }
                                                                            }
                                                                        });
                                                                    }
                                                                />
                                                                <span class="text-sm font-mono">{dep_id_display}</span>
                                                            </label>
                                                        }
                                                    }).collect::<Vec<_>>()}
                                                </div>
                                                {move || {
                                                    let deps = step_depends_on();
                                                    if !deps.is_empty() {
                                                        view! {
                                                            <div class="mt-2 text-xs text-green-600">
                                                                "This step will run after: "
                                                                <span class="font-mono">{deps.join(", ")}</span>
                                                            </div>
                                                        }.into_any()
                                                    } else {
                                                        view! {
                                                            <div class="mt-2 text-xs text-gray-500">
                                                                "No dependencies - step can run immediately when workflow starts"
                                                            </div>
                                                        }.into_any()
                                                    }
                                                }}
                                            </div>
                                        }.into_any()
                                    }
                                }}
                            </div>

                            // Conditional execution
                            <div>
                                <label class="block text-sm font-medium text-gray-700 mb-1">"Condition (optional)"</label>
                                <input
                                    type="text"
                                    class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-orange-500 text-sm font-mono"
                                    placeholder="$.input.enabled == true"
                                    prop:value=step_condition
                                    on:input=move |ev| {
                                        let target = ev.target().unwrap();
                                        let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                        set_steps.update(|s| {
                                            if let Some(st) = s.get_mut(index) {
                                                st.condition = input.value();
                                            }
                                        });
                                    }
                                />
                                <p class="mt-1 text-xs text-gray-500">"Step will only execute if condition evaluates to true"</p>
                            </div>

                            // Loop configuration
                            <div class="p-3 bg-purple-50 rounded-lg border border-purple-200">
                                <label class="block text-sm font-medium text-purple-700 mb-2">"Loop Configuration"</label>
                                <div class="grid grid-cols-3 gap-3">
                                    <div>
                                        <label class="block text-xs text-gray-600 mb-1">"Loop Over"</label>
                                        <input
                                            type="text"
                                            class="w-full px-2 py-1 border border-gray-300 rounded text-sm font-mono"
                                            placeholder="$.input.items"
                                            prop:value=step_loop_over
                                            on:input=move |ev| {
                                                let target = ev.target().unwrap();
                                                let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                                set_steps.update(|s| {
                                                    if let Some(st) = s.get_mut(index) {
                                                        st.loop_over = input.value();
                                                    }
                                                });
                                            }
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-xs text-gray-600 mb-1">"Loop Variable"</label>
                                        <input
                                            type="text"
                                            class="w-full px-2 py-1 border border-gray-300 rounded text-sm font-mono"
                                            placeholder="item"
                                            prop:value=step_loop_var
                                            on:input=move |ev| {
                                                let target = ev.target().unwrap();
                                                let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                                set_steps.update(|s| {
                                                    if let Some(st) = s.get_mut(index) {
                                                        st.loop_var = input.value();
                                                    }
                                                });
                                            }
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-xs text-gray-600 mb-1">"Concurrency"</label>
                                        <input
                                            type="number"
                                            min=1
                                            class="w-full px-2 py-1 border border-gray-300 rounded text-sm"
                                            prop:value=move || step_loop_concurrency().to_string()
                                            on:input=move |ev| {
                                                let target = ev.target().unwrap();
                                                let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                                set_steps.update(|s| {
                                                    if let Some(st) = s.get_mut(index) {
                                                        st.loop_concurrency = input.value().parse().unwrap_or(1);
                                                    }
                                                });
                                            }
                                        />
                                    </div>
                                </div>
                                <p class="mt-2 text-xs text-gray-500">"Execute this step for each item in an array. Access current item via loop variable."</p>
                            </div>

                            // Error handling
                            <div class="p-3 bg-blue-50 rounded-lg border border-blue-200">
                                <label class="block text-sm font-medium text-blue-700 mb-2">"Error Handling"</label>
                                <div class="space-y-3">
                                    <div>
                                        <label class="block text-xs text-gray-600 mb-1">"On Error"</label>
                                        <select
                                            class="w-full px-2 py-1 border border-gray-300 rounded text-sm"
                                            prop:value=step_error_strategy
                                            on:change=move |ev| {
                                                let target = ev.target().unwrap();
                                                let select: web_sys::HtmlSelectElement = target.dyn_into().unwrap();
                                                set_steps.update(|s| {
                                                    if let Some(st) = s.get_mut(index) {
                                                        st.error_strategy = select.value();
                                                    }
                                                });
                                            }
                                        >
                                            <option value="fail">"Fail - Stop workflow"</option>
                                            <option value="continue">"Continue - Skip to next step"</option>
                                            <option value="retry">"Retry - Retry with backoff"</option>
                                            <option value="fallback">"Fallback - Use default value"</option>
                                        </select>
                                    </div>

                                    // Retry options
                                    <Show when=move || step_error_strategy() == "retry">
                                        <div class="grid grid-cols-2 gap-3">
                                            <div>
                                                <label class="block text-xs text-gray-600 mb-1">"Max Attempts"</label>
                                                <input
                                                    type="number"
                                                    min=1
                                                    class="w-full px-2 py-1 border border-gray-300 rounded text-sm"
                                                    prop:value=move || step_retry_max().to_string()
                                                    on:input=move |ev| {
                                                        let target = ev.target().unwrap();
                                                        let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                                        set_steps.update(|s| {
                                                            if let Some(st) = s.get_mut(index) {
                                                                st.retry_max_attempts = input.value().parse().unwrap_or(3);
                                                            }
                                                        });
                                                    }
                                                />
                                            </div>
                                            <div>
                                                <label class="block text-xs text-gray-600 mb-1">"Delay (ms)"</label>
                                                <input
                                                    type="number"
                                                    min=0
                                                    class="w-full px-2 py-1 border border-gray-300 rounded text-sm"
                                                    prop:value=move || step_retry_delay().to_string()
                                                    on:input=move |ev| {
                                                        let target = ev.target().unwrap();
                                                        let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                                        set_steps.update(|s| {
                                                            if let Some(st) = s.get_mut(index) {
                                                                st.retry_delay_ms = input.value().parse().unwrap_or(1000);
                                                            }
                                                        });
                                                    }
                                                />
                                            </div>
                                        </div>
                                    </Show>

                                    // Fallback value
                                    <Show when=move || step_error_strategy() == "fallback">
                                        <div>
                                            <label class="block text-xs text-gray-600 mb-1">"Fallback Value (JSON)"</label>
                                            <textarea
                                                rows=2
                                                class="w-full px-2 py-1 border border-gray-300 rounded text-sm font-mono"
                                                placeholder="null"
                                                prop:value=step_fallback
                                                on:input=move |ev| {
                                                    let target = ev.target().unwrap();
                                                    let textarea: web_sys::HtmlTextAreaElement = target.dyn_into().unwrap();
                                                    set_steps.update(|s| {
                                                        if let Some(st) = s.get_mut(index) {
                                                            st.fallback_value = textarea.value();
                                                        }
                                                    });
                                                }
                                            />
                                        </div>
                                    </Show>
                                </div>
                            </div>
                        </div>
                    </details>
                </div>
            </Show>
        </div>
    }
}
