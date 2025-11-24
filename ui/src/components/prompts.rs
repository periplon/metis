use leptos::prelude::*;
use crate::api;
use crate::types::Prompt;

#[component]
pub fn Prompts() -> impl IntoView {
    let prompts = LocalResource::new(|| async move {
        api::list_prompts().await.ok()
    });

    view! {
        <div class="p-6">
            <div class="flex justify-between items-center mb-6">
                <h2 class="text-2xl font-bold">"Prompts"</h2>
                <a href="/prompts/new" class="bg-purple-500 hover:bg-purple-600 text-white px-4 py-2 rounded">
                    "+ New Prompt"
                </a>
            </div>

            <Suspense fallback=move || view! { <div class="text-gray-500">"Loading..."</div> }>
                {move || {
                    prompts.get().map(|data| {
                        match data {
                            Some(list) if !list.is_empty() => view! {
                                <div class="space-y-4">
                                    {list.into_iter().map(|prompt| {
                                        view! { <PromptCard prompt=prompt /> }
                                    }).collect::<Vec<_>>()}
                                </div>
                            }.into_any(),
                            Some(_) => view! {
                                <div class="text-center py-12 bg-white rounded-lg shadow">
                                    <p class="text-gray-500 mb-4">"No prompts configured"</p>
                                    <a href="/prompts/new" class="text-purple-500 hover:underline">"Create your first prompt"</a>
                                </div>
                            }.into_any(),
                            None => view! {
                                <div class="text-red-500">"Failed to load prompts"</div>
                            }.into_any(),
                        }
                    })
                }}
            </Suspense>
        </div>
    }
}

#[component]
fn PromptCard(prompt: Prompt) -> impl IntoView {
    let args_count = prompt.arguments.as_ref().map(|a| a.len()).unwrap_or(0);
    let msgs_count = prompt.messages.as_ref().map(|m| m.len()).unwrap_or(0);

    view! {
        <div class="bg-white rounded-lg shadow p-4 hover:shadow-md transition-shadow">
            <div class="flex justify-between items-start mb-2">
                <div>
                    <h3 class="font-bold text-lg text-gray-900">{prompt.name.clone()}</h3>
                    <p class="text-gray-600 text-sm">{prompt.description.clone()}</p>
                </div>
                <div class="flex space-x-2">
                    <span class="px-2 py-1 text-xs rounded bg-purple-100 text-purple-800">
                        {format!("{} args", args_count)}
                    </span>
                    <span class="px-2 py-1 text-xs rounded bg-gray-100 text-gray-800">
                        {format!("{} messages", msgs_count)}
                    </span>
                </div>
            </div>
            <div class="flex justify-end space-x-2 mt-4">
                <button class="text-sm text-blue-600 hover:text-blue-900">"Edit"</button>
                <button class="text-sm text-red-600 hover:text-red-900">"Delete"</button>
            </div>
        </div>
    }
}
