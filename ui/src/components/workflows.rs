use leptos::prelude::*;
use crate::api;
use crate::types::Workflow;

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
