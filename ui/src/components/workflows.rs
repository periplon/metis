use leptos::prelude::*;
use leptos::web_sys;
use wasm_bindgen::JsCast;
use crate::api;
use crate::types::{Workflow, WorkflowStep, ErrorStrategy};

#[component]
pub fn Workflows() -> impl IntoView {
    let workflows = LocalResource::new(|| async move {
        api::list_workflows().await.ok()
    });

    view! {
        <div class="p-6">
            <div class="flex justify-between items-center mb-6">
                <h2 class="text-2xl font-bold">"Workflows"</h2>
                <a href="/workflows/new" class="bg-orange-500 hover:bg-orange-600 text-white px-4 py-2 rounded">
                    "+ New Workflow"
                </a>
            </div>

            <Suspense fallback=move || view! { <div class="text-gray-500">"Loading..."</div> }>
                {move || {
                    workflows.get().map(|data| {
                        match data {
                            Some(list) if !list.is_empty() => view! {
                                <div class="space-y-4">
                                    {list.into_iter().map(|workflow| {
                                        view! { <WorkflowCard workflow=workflow /> }
                                    }).collect::<Vec<_>>()}
                                </div>
                            }.into_any(),
                            Some(_) => view! {
                                <div class="text-center py-12 bg-white rounded-lg shadow">
                                    <p class="text-gray-500 mb-4">"No workflows configured"</p>
                                    <a href="/workflows/new" class="text-orange-500 hover:underline">"Create your first workflow"</a>
                                </div>
                            }.into_any(),
                            None => view! {
                                <div class="text-red-500">"Failed to load workflows"</div>
                            }.into_any(),
                        }
                    })
                }}
            </Suspense>
        </div>
    }
}

#[component]
fn WorkflowCard(workflow: Workflow) -> impl IntoView {
    let steps_count = workflow.steps.len();

    view! {
        <div class="bg-white rounded-lg shadow p-4 hover:shadow-md transition-shadow">
            <div class="flex justify-between items-start mb-4">
                <div>
                    <h3 class="font-bold text-lg text-gray-900">{workflow.name.clone()}</h3>
                    <p class="text-gray-600 text-sm">{workflow.description.clone()}</p>
                </div>
                <span class="px-2 py-1 text-xs font-semibold rounded-full bg-orange-100 text-orange-800">
                    {format!("{} steps", steps_count)}
                </span>
            </div>

            // Visual step flow
            <div class="flex items-center space-x-2 overflow-x-auto pb-2">
                {workflow.steps.iter().enumerate().map(|(i, step)| {
                    let is_last = i == steps_count - 1;
                    view! {
                        <div class="flex items-center">
                            <div class="px-3 py-1 bg-gray-100 rounded text-sm font-mono">
                                {step.tool.clone()}
                            </div>
                            {if !is_last {
                                view! { <span class="mx-1 text-gray-400">"→"</span> }.into_any()
                            } else {
                                view! { <span></span> }.into_any()
                            }}
                        </div>
                    }
                }).collect::<Vec<_>>()}
            </div>

            <div class="flex justify-end space-x-2 mt-4 pt-4 border-t">
                <button class="text-sm text-blue-600 hover:text-blue-900">"Edit"</button>
                <button class="text-sm text-green-600 hover:text-green-900">"Test"</button>
                <button class="text-sm text-red-600 hover:text-red-900">"Delete"</button>
            </div>
        </div>
    }
}

#[component]
pub fn WorkflowForm() -> impl IntoView {
    let (name, set_name) = signal(String::new());
    let (description, set_description) = signal(String::new());
    let (input_schema, set_input_schema) = signal(String::from("{}"));
    let (steps_json, set_steps_json) = signal(String::new());
    let (error, set_error) = signal(Option::<String>::None);
    let (saving, set_saving) = signal(false);

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        set_saving.set(true);
        set_error.set(None);

        let schema: serde_json::Value = serde_json::from_str(&input_schema.get())
            .unwrap_or(serde_json::json!({}));

        let steps: Vec<WorkflowStep> = if steps_json.get().is_empty() {
            Vec::new()
        } else {
            match serde_json::from_str(&steps_json.get()) {
                Ok(s) => s,
                Err(e) => {
                    set_error.set(Some(format!("Invalid steps JSON: {}", e)));
                    set_saving.set(false);
                    return;
                }
            }
        };

        if steps.is_empty() {
            set_error.set(Some("At least one step is required".to_string()));
            set_saving.set(false);
            return;
        }

        let workflow = Workflow {
            name: name.get(),
            description: description.get(),
            input_schema: schema,
            steps,
            on_error: ErrorStrategy::Fail,
        };

        wasm_bindgen_futures::spawn_local(async move {
            match api::create_workflow(&workflow).await {
                Ok(_) => {
                    if let Some(window) = web_sys::window() {
                        let _ = window.location().set_href("/workflows");
                    }
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_saving.set(false);
                }
            }
        });
    };

    view! {
        <div class="p-6">
            <div class="mb-6">
                <a href="/workflows" class="text-orange-500 hover:underline">"← Back to Workflows"</a>
            </div>

            <h2 class="text-2xl font-bold mb-6">"New Workflow"</h2>

            <form on:submit=on_submit class="bg-white rounded-lg shadow p-6 max-w-2xl">
                {move || error.get().map(|e| view! {
                    <div class="mb-4 p-3 bg-red-100 border border-red-400 text-red-700 rounded">
                        {e}
                    </div>
                })}

                <div class="space-y-4">
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">"Name *"</label>
                        <input
                            type="text"
                            required=true
                            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-orange-500"
                            placeholder="my-workflow"
                            prop:value=move || name.get()
                            on:input=move |ev| {
                                let target = ev.target().unwrap();
                                let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                set_name.set(input.value());
                            }
                        />
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">"Description *"</label>
                        <input
                            type="text"
                            required=true
                            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-orange-500"
                            placeholder="What this workflow does"
                            prop:value=move || description.get()
                            on:input=move |ev| {
                                let target = ev.target().unwrap();
                                let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                set_description.set(input.value());
                            }
                        />
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">"Input Schema (JSON)"</label>
                        <textarea
                            rows=4
                            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-orange-500 font-mono text-sm"
                            placeholder="{}"
                            prop:value=move || input_schema.get()
                            on:input=move |ev| {
                                let target = ev.target().unwrap();
                                let textarea: web_sys::HtmlTextAreaElement = target.dyn_into().unwrap();
                                set_input_schema.set(textarea.value());
                            }
                        />
                        <p class="mt-1 text-xs text-gray-500">"JSON Schema for workflow input parameters"</p>
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">"Steps (JSON Array) *"</label>
                        <textarea
                            rows=8
                            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-orange-500 font-mono text-sm"
                            placeholder=r#"[{"id": "step1", "tool": "my-tool"}]"#
                            prop:value=move || steps_json.get()
                            on:input=move |ev| {
                                let target = ev.target().unwrap();
                                let textarea: web_sys::HtmlTextAreaElement = target.dyn_into().unwrap();
                                set_steps_json.set(textarea.value());
                            }
                        />
                        <p class="mt-1 text-xs text-gray-500">"Array of workflow steps. Each step needs: id, tool, and optionally args"</p>
                    </div>
                </div>

                <div class="mt-6 flex gap-3">
                    <button
                        type="submit"
                        disabled=move || saving.get()
                        class="px-4 py-2 bg-orange-500 text-white rounded hover:bg-orange-600 disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                        {move || if saving.get() { "Creating..." } else { "Create Workflow" }}
                    </button>
                    <a
                        href="/workflows"
                        class="px-4 py-2 border border-gray-300 text-gray-700 rounded hover:bg-gray-50"
                    >
                        "Cancel"
                    </a>
                </div>
            </form>
        </div>
    }
}
