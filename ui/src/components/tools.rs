use leptos::prelude::*;
use crate::api;
use crate::types::Tool;

#[component]
pub fn Tools() -> impl IntoView {
    let tools = LocalResource::new(|| async move {
        api::list_tools().await.ok()
    });

    view! {
        <div class="p-6">
            <div class="flex justify-between items-center mb-6">
                <h2 class="text-2xl font-bold">"Tools"</h2>
                <a href="/tools/new" class="bg-green-500 hover:bg-green-600 text-white px-4 py-2 rounded">
                    "+ New Tool"
                </a>
            </div>

            <Suspense fallback=move || view! { <div class="text-gray-500">"Loading..."</div> }>
                {move || {
                    tools.get().map(|data| {
                        match data {
                            Some(list) if !list.is_empty() => view! {
                                <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                                    {list.into_iter().map(|tool| {
                                        view! { <ToolCard tool=tool /> }
                                    }).collect::<Vec<_>>()}
                                </div>
                            }.into_any(),
                            Some(_) => view! {
                                <div class="text-center py-12 bg-white rounded-lg shadow">
                                    <p class="text-gray-500 mb-4">"No tools configured"</p>
                                    <a href="/tools/new" class="text-green-500 hover:underline">"Create your first tool"</a>
                                </div>
                            }.into_any(),
                            None => view! {
                                <div class="text-red-500">"Failed to load tools"</div>
                            }.into_any(),
                        }
                    })
                }}
            </Suspense>
        </div>
    }
}

#[component]
fn ToolCard(tool: Tool) -> impl IntoView {
    let strategy = tool.mock.as_ref()
        .map(|m| format!("{:?}", m.strategy))
        .unwrap_or_else(|| if tool.static_response.is_some() { "Static".to_string() } else { "-".to_string() });

    view! {
        <div class="bg-white rounded-lg shadow p-4 hover:shadow-md transition-shadow">
            <div class="flex justify-between items-start mb-2">
                <h3 class="font-bold text-lg text-gray-900">{tool.name.clone()}</h3>
                <span class="px-2 py-1 text-xs font-semibold rounded-full bg-green-100 text-green-800">
                    {strategy}
                </span>
            </div>
            <p class="text-gray-600 text-sm mb-4">{tool.description.clone()}</p>
            <div class="flex justify-end space-x-2">
                <button class="text-sm text-blue-600 hover:text-blue-900">"Edit"</button>
                <button class="text-sm text-red-600 hover:text-red-900">"Delete"</button>
            </div>
        </div>
    }
}
