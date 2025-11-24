use leptos::prelude::*;
use leptos::web_sys;
use wasm_bindgen::JsCast;
use crate::api;
use crate::types::{Prompt, PromptArgument, PromptMessage};

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

#[component]
pub fn PromptForm() -> impl IntoView {
    let (name, set_name) = signal(String::new());
    let (description, set_description) = signal(String::new());
    let (args_json, set_args_json) = signal(String::new());
    let (messages_json, set_messages_json) = signal(String::new());
    let (error, set_error) = signal(Option::<String>::None);
    let (saving, set_saving) = signal(false);

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        set_saving.set(true);
        set_error.set(None);

        let arguments: Option<Vec<PromptArgument>> = if args_json.get().is_empty() {
            None
        } else {
            match serde_json::from_str(&args_json.get()) {
                Ok(args) => Some(args),
                Err(e) => {
                    set_error.set(Some(format!("Invalid arguments JSON: {}", e)));
                    set_saving.set(false);
                    return;
                }
            }
        };

        let messages: Option<Vec<PromptMessage>> = if messages_json.get().is_empty() {
            None
        } else {
            match serde_json::from_str(&messages_json.get()) {
                Ok(msgs) => Some(msgs),
                Err(e) => {
                    set_error.set(Some(format!("Invalid messages JSON: {}", e)));
                    set_saving.set(false);
                    return;
                }
            }
        };

        let prompt = Prompt {
            name: name.get(),
            description: description.get(),
            arguments,
            messages,
        };

        wasm_bindgen_futures::spawn_local(async move {
            match api::create_prompt(&prompt).await {
                Ok(_) => {
                    if let Some(window) = web_sys::window() {
                        let _ = window.location().set_href("/prompts");
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
                <a href="/prompts" class="text-purple-500 hover:underline">"← Back to Prompts"</a>
            </div>

            <h2 class="text-2xl font-bold mb-6">"New Prompt"</h2>

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
                            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
                            placeholder="my-prompt"
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
                            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
                            placeholder="What this prompt does"
                            prop:value=move || description.get()
                            on:input=move |ev| {
                                let target = ev.target().unwrap();
                                let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                set_description.set(input.value());
                            }
                        />
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">"Arguments (JSON Array)"</label>
                        <textarea
                            rows=4
                            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500 font-mono text-sm"
                            placeholder=r#"[{"name": "topic", "required": true}]"#
                            prop:value=move || args_json.get()
                            on:input=move |ev| {
                                let target = ev.target().unwrap();
                                let textarea: web_sys::HtmlTextAreaElement = target.dyn_into().unwrap();
                                set_args_json.set(textarea.value());
                            }
                        />
                        <p class="mt-1 text-xs text-gray-500">"Optional array of argument definitions"</p>
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">"Messages (JSON Array)"</label>
                        <textarea
                            rows=6
                            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500 font-mono text-sm"
                            placeholder=r#"[{"role": "user", "content": "Hello"}]"#
                            prop:value=move || messages_json.get()
                            on:input=move |ev| {
                                let target = ev.target().unwrap();
                                let textarea: web_sys::HtmlTextAreaElement = target.dyn_into().unwrap();
                                set_messages_json.set(textarea.value());
                            }
                        />
                        <p class="mt-1 text-xs text-gray-500">"Optional array of prompt messages"</p>
                    </div>
                </div>

                <div class="mt-6 flex gap-3">
                    <button
                        type="submit"
                        disabled=move || saving.get()
                        class="px-4 py-2 bg-purple-500 text-white rounded hover:bg-purple-600 disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                        {move || if saving.get() { "Creating..." } else { "Create Prompt" }}
                    </button>
                    <a
                        href="/prompts"
                        class="px-4 py-2 border border-gray-300 text-gray-700 rounded hover:bg-gray-50"
                    >
                        "Cancel"
                    </a>
                </div>
            </form>
        </div>
    }
}
