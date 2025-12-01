//! Secrets management UI component
//!
//! Provides a form for managing in-memory API keys and credentials.
//! Keys are stored server-side in memory only and lost on restart.

use crate::api;
use leptos::prelude::*;

/// Secrets Editor component
#[component]
pub fn SecretsEditor() -> impl IntoView {
    let (secrets_status, set_secrets_status) =
        signal(None::<api::SecretsStatusResponse>);
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal(None::<String>);
    let (success_message, set_success_message) = signal(None::<String>);

    // Input values for each secret (keyed by secret key)
    let (input_values, set_input_values) =
        signal(std::collections::HashMap::<String, String>::new());

    // Load secrets status
    let load_secrets = move || {
        set_loading.set(true);
        set_error.set(None);
        wasm_bindgen_futures::spawn_local(async move {
            match api::list_secrets().await {
                Ok(status) => {
                    set_secrets_status.set(Some(status));
                    set_loading.set(false);
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_loading.set(false);
                }
            }
        });
    };

    // Load on mount
    Effect::new(move || {
        load_secrets();
    });

    // Show success message temporarily
    let show_success = move |msg: String| {
        set_success_message.set(Some(msg));
        // Clear after 3 seconds
        let handle = gloo_timers::callback::Timeout::new(3000, move || {
            set_success_message.set(None);
        });
        handle.forget();
    };

    // Set a secret
    let set_secret = move |key: String| {
        let value = input_values.get().get(&key).cloned().unwrap_or_default();
        if value.is_empty() {
            set_error.set(Some("Please enter a value".to_string()));
            return;
        }

        set_loading.set(true);
        set_error.set(None);
        wasm_bindgen_futures::spawn_local(async move {
            match api::set_secret(&key, &value).await {
                Ok(()) => {
                    // Clear the input
                    set_input_values.update(|vals| {
                        vals.remove(&key);
                    });
                    show_success(format!("{} has been set", key));
                    load_secrets();
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_loading.set(false);
                }
            }
        });
    };

    // Delete a secret
    let delete_secret = move |key: String| {
        set_loading.set(true);
        set_error.set(None);
        wasm_bindgen_futures::spawn_local(async move {
            match api::delete_secret(&key).await {
                Ok(()) => {
                    show_success(format!("{} has been cleared", key));
                    load_secrets();
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_loading.set(false);
                }
            }
        });
    };

    // Clear all secrets
    let clear_all = move || {
        set_loading.set(true);
        set_error.set(None);
        wasm_bindgen_futures::spawn_local(async move {
            match api::clear_secrets().await {
                Ok(()) => {
                    show_success("All secrets have been cleared".to_string());
                    load_secrets();
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_loading.set(false);
                }
            }
        });
    };

    view! {
        <div class="">
            <div class="flex justify-between items-center mb-4">
                <div>
                    <h2 class="text-xl font-semibold text-gray-900">"API Keys & Credentials"</h2>
                    <p class="text-sm text-gray-500 mt-1">
                        "Keys are stored in memory only and will be lost when the server restarts. "
                        "Environment variables can still be used if no key is set here."
                    </p>
                </div>
                <button
                    class="px-3 py-1.5 text-sm bg-red-100 text-red-700 rounded hover:bg-red-200 disabled:opacity-50"
                    disabled=move || loading.get()
                    on:click=move |_| clear_all()
                >
                    "Clear All"
                </button>
            </div>

            // Success message
            {move || success_message.get().map(|msg| view! {
                <div class="mb-4 p-3 bg-green-50 border border-green-200 rounded text-green-800 text-sm">
                    {msg}
                </div>
            })}

            // Error message
            {move || error.get().map(|e| view! {
                <div class="mb-4 p-3 bg-red-50 border border-red-200 rounded text-red-800 text-sm">
                    {e}
                </div>
            })}

            // Loading state
            {move || loading.get().then(|| view! {
                <div class="text-center py-4 text-gray-500">
                    "Loading..."
                </div>
            })}

            // Secrets list grouped by category
            {move || secrets_status.get().map(|status| {
                // Group secrets by category
                let mut categories: std::collections::HashMap<String, Vec<api::SecretKeyStatus>> =
                    std::collections::HashMap::new();
                for key_status in status.keys {
                    categories
                        .entry(key_status.category.clone())
                        .or_default()
                        .push(key_status);
                }

                // Sort categories
                let mut sorted_categories: Vec<_> = categories.into_iter().collect();
                sorted_categories.sort_by(|a, b| a.0.cmp(&b.0));

                view! {
                    <div class="space-y-6">
                        {sorted_categories.into_iter().map(|(category, keys)| {
                            view! {
                                <div>
                                    <h3 class="text-lg font-medium text-gray-800 mb-3 pb-2 border-b">
                                        {category}
                                    </h3>
                                    <div class="space-y-4">
                                        {keys.into_iter().map(|key_status| {
                                            let key = key_status.key.clone();
                                            let key_for_input = key.clone();
                                            let key_for_set = key.clone();
                                            let key_for_delete = key.clone();

                                            view! {
                                                <div class="flex items-start gap-4 p-4 bg-gray-50 rounded-lg">
                                                    <div class="flex-1">
                                                        <div class="flex items-center gap-2 mb-1">
                                                            <label class="font-medium text-gray-700">
                                                                {key_status.label}
                                                            </label>
                                                            {if key_status.is_set {
                                                                view! {
                                                                    <span class="px-2 py-0.5 text-xs bg-green-100 text-green-700 rounded">
                                                                        "Set"
                                                                    </span>
                                                                }.into_any()
                                                            } else {
                                                                view! {
                                                                    <span class="px-2 py-0.5 text-xs bg-gray-200 text-gray-600 rounded">
                                                                        "Not Set"
                                                                    </span>
                                                                }.into_any()
                                                            }}
                                                        </div>
                                                        <p class="text-sm text-gray-500 mb-2">
                                                            {key_status.description}
                                                        </p>
                                                        <div class="flex gap-2">
                                                            <input
                                                                type="password"
                                                                class="flex-1 px-3 py-2 border rounded text-sm focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                                                                placeholder=move || {
                                                                    if key_status.is_set {
                                                                        "Enter new value to replace..."
                                                                    } else {
                                                                        "Enter value..."
                                                                    }
                                                                }
                                                                prop:value=move || {
                                                                    input_values.get().get(&key_for_input).cloned().unwrap_or_default()
                                                                }
                                                                on:input=move |ev| {
                                                                    let value = event_target_value(&ev);
                                                                    let key_clone = key.clone();
                                                                    set_input_values.update(|vals| {
                                                                        vals.insert(key_clone, value);
                                                                    });
                                                                }
                                                            />
                                                            <button
                                                                class="px-4 py-2 text-sm bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50"
                                                                disabled=move || loading.get()
                                                                on:click=move |_| {
                                                                    set_secret(key_for_set.clone());
                                                                }
                                                            >
                                                                {if key_status.is_set { "Update" } else { "Set" }}
                                                            </button>
                                                            {if key_status.is_set {
                                                                Some(view! {
                                                                    <button
                                                                        class="px-4 py-2 text-sm bg-gray-200 text-gray-700 rounded hover:bg-gray-300 disabled:opacity-50"
                                                                        disabled=move || loading.get()
                                                                        on:click=move |_| {
                                                                            delete_secret(key_for_delete.clone());
                                                                        }
                                                                    >
                                                                        "Clear"
                                                                    </button>
                                                                })
                                                            } else {
                                                                None
                                                            }}
                                                        </div>
                                                    </div>
                                                </div>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </div>
                                </div>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                }
            })}
        </div>
    }
}
