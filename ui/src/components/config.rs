use leptos::prelude::*;
use leptos::web_sys;
use std::collections::HashMap;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use crate::api;
use crate::components::secrets::SecretsEditor;
use crate::types::{ConfigOverview, ServerSettings, AuthConfig, RateLimitConfig, S3Config};

// Re-export web_sys types with full features for file operations
use web_sys::{Blob, BlobPropertyBag, FileReader, Url};

#[component]
pub fn Config() -> impl IntoView {
    let config = LocalResource::new(|| async move {
        log::info!("Fetching config...");
        let result = api::get_config().await;
        match &result {
            Ok(_) => log::info!("Config result: OK"),
            Err(e) => log::error!("Config result: ERROR - {}", e),
        }
        result.ok()
    });

    let settings = LocalResource::new(|| async move {
        log::info!("Fetching settings...");
        let result = api::get_server_settings().await;
        match &result {
            Ok(_) => log::info!("Settings result: OK"),
            Err(e) => log::error!("Settings result: ERROR - {}", e),
        }
        result.ok()
    });

    view! {
        <div class="p-6">
            <h2 class="text-2xl font-bold mb-6">"Configuration"</h2>

            <Suspense fallback=move || view! { <div class="text-gray-500">"Loading..."</div> }>
                {move || {
                    let config_data = config.get();
                    let settings_data = settings.get();

                    log::info!("Config data: {:?}", config_data.is_some());
                    log::info!("Settings data: {:?}", settings_data.is_some());

                    match (config_data, settings_data) {
                        (Some(Some(overview)), Some(Some(server_settings))) => {
                            // Check if config file was loaded
                            if !overview.config_file_loaded {
                                // No config file - show banner and editor with defaults
                                view! {
                                    <div class="space-y-6">
                                        <NewConfigBanner />
                                        <SecretsEditor />
                                        <SettingsEditorCard initial_settings=server_settings />
                                    </div>
                                }.into_any()
                            } else {
                                // Config file exists - show normal view
                                view! {
                                    <div class="space-y-6">
                                        <ServerConfigCard overview=overview.clone() />
                                        <SecretsEditor />
                                        <SettingsEditorCard initial_settings=server_settings />
                                        <QuickLinksCard overview=overview />
                                    </div>
                                }.into_any()
                            }
                        },
                        (Some(Some(overview)), _) => {
                            // Handle case where settings API failed but we have overview
                            if !overview.config_file_loaded {
                                // No config file - show default editor even if settings API failed
                                let default_settings = ServerSettings {
                                    auth: AuthConfig {
                                        enabled: false,
                                        mode: "None".to_string(),
                                        api_keys: None,
                                        jwt_secret: None,
                                        jwt_algorithm: None,
                                        basic_users: None,
                                        jwks_url: None,
                                    },
                                    rate_limit: Some(RateLimitConfig {
                                        enabled: false,
                                        requests_per_second: 100,
                                        burst_size: 10,
                                    }),
                                    s3: Some(S3Config {
                                        enabled: false,
                                        bucket: None,
                                        prefix: None,
                                        region: None,
                                        endpoint: None,
                                        poll_interval_secs: 30,
                                    }),
                                };
                                view! {
                                    <div class="space-y-6">
                                        <NewConfigBanner />
                                        <SecretsEditor />
                                        <SettingsEditorCard initial_settings=default_settings />
                                    </div>
                                }.into_any()
                            } else {
                                // Config file exists - show normal view without settings editor
                                view! {
                                    <div class="space-y-6">
                                        <ServerConfigCard overview=overview.clone() />
                                        <SecretsEditor />
                                        <FeaturesCard overview=overview.clone() />
                                        <QuickLinksCard overview=overview />
                                    </div>
                                }.into_any()
                            }
                        },
                        // Still loading or error - show loading/error state
                        _ => view! {
                            <div class="text-gray-500">"Loading configuration..."</div>
                        }.into_any(),
                    }
                }}
            </Suspense>
        </div>
    }
}

#[component]
fn NewConfigBanner() -> impl IntoView {
    view! {
        <div class="bg-blue-50 border border-blue-200 rounded-lg p-6">
            <div class="flex items-start">
                <div class="flex-shrink-0">
                    <svg class="h-6 w-6 text-blue-600" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                    </svg>
                </div>
                <div class="ml-4">
                    <h3 class="text-lg font-semibold text-blue-800">"No Configuration File"</h3>
                    <p class="mt-1 text-blue-700">
                        "No "
                        <code class="bg-blue-100 px-1 rounded">"metis.toml"</code>
                        " found. Settings are currently running with defaults."
                    </p>
                    <div class="mt-3 p-3 bg-yellow-50 border border-yellow-200 rounded">
                        <p class="text-sm text-yellow-800">
                            <strong>"⚠️ Important:"</strong>
                            " Changes made here are only saved "
                            <strong>"in memory"</strong>
                            " and will be lost when the server restarts. To persist settings, create a "
                            <code class="bg-yellow-100 px-1 rounded">"metis.toml"</code>
                            " file in the server's working directory."
                        </p>
                    </div>
                </div>
            </div>
        </div>
    }
}

#[component]
fn ServerConfigCard(overview: ConfigOverview) -> impl IntoView {
    view! {
        <div class="bg-white rounded-lg shadow overflow-hidden">
            <div class="px-6 py-4 border-b border-gray-200 bg-gray-50">
                <h3 class="text-lg font-semibold text-gray-800">"Server Configuration"</h3>
            </div>
            <div class="p-6">
                <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                    <ConfigItem label="Host" value=overview.server.host.clone() />
                    <ConfigItem label="Port" value=overview.server.port.to_string() />
                    <ConfigItem label="Version" value=overview.server.version.clone() />
                    <ConfigItem
                        label="Status"
                        value="Running".to_string()
                        badge_color="green"
                    />
                </div>
            </div>
        </div>
    }
}

#[component]
fn SettingsEditorCard(initial_settings: ServerSettings) -> impl IntoView {
    let (auth_enabled, set_auth_enabled) = signal(initial_settings.auth.enabled);
    let (auth_mode, set_auth_mode) = signal(initial_settings.auth.mode.clone());

    // Auth mode-specific fields
    let (api_keys, set_api_keys) = signal(
        initial_settings.auth.api_keys.clone()
            .map(|keys| keys.join(", "))
            .unwrap_or_default()
    );
    let (jwt_secret, set_jwt_secret) = signal(
        initial_settings.auth.jwt_secret.clone().unwrap_or_default()
    );
    let (jwt_algorithm, set_jwt_algorithm) = signal(
        initial_settings.auth.jwt_algorithm.clone().unwrap_or_else(|| "HS256".to_string())
    );
    let (jwks_url, set_jwks_url) = signal(
        initial_settings.auth.jwks_url.clone().unwrap_or_default()
    );

    // Basic Auth users - stored as Vec<(username, password)> for easy UI management
    let (basic_users, set_basic_users) = signal(
        initial_settings.auth.basic_users.clone()
            .map(|users| users.into_iter().collect::<Vec<_>>())
            .unwrap_or_default()
    );
    let (new_username, set_new_username) = signal(String::new());
    let (new_password, set_new_password) = signal(String::new());

    let initial_rate_limit = initial_settings.rate_limit.clone().unwrap_or(RateLimitConfig {
        enabled: false,
        requests_per_second: 100,
        burst_size: 10,
    });

    let (rate_limit_enabled, set_rate_limit_enabled) = signal(initial_rate_limit.enabled);
    let (requests_per_second, set_requests_per_second) = signal(initial_rate_limit.requests_per_second);
    let (burst_size, set_burst_size) = signal(initial_rate_limit.burst_size);

    let initial_s3 = initial_settings.s3.clone().unwrap_or(S3Config {
        enabled: false,
        bucket: None,
        prefix: None,
        region: None,
        endpoint: None,
        poll_interval_secs: 30,
    });

    let (s3_enabled, set_s3_enabled) = signal(initial_s3.enabled);
    let (s3_bucket, set_s3_bucket) = signal(initial_s3.bucket.unwrap_or_default());
    let (s3_prefix, set_s3_prefix) = signal(initial_s3.prefix.unwrap_or_default());
    let (s3_region, set_s3_region) = signal(initial_s3.region.unwrap_or_default());
    let (s3_endpoint, set_s3_endpoint) = signal(initial_s3.endpoint.unwrap_or_default());
    let (s3_poll_interval, set_s3_poll_interval) = signal(initial_s3.poll_interval_secs);

    let (saving, set_saving) = signal(false);
    let (message, set_message) = signal(Option::<(String, bool)>::None);

    let on_save = move |_| {
        set_saving.set(true);
        set_message.set(None);

        let bucket_val = s3_bucket.get();
        let prefix_val = s3_prefix.get();
        let region_val = s3_region.get();
        let endpoint_val = s3_endpoint.get();

        // Parse API keys from comma-separated string
        let api_keys_val = api_keys.get();
        let api_keys_vec: Option<Vec<String>> = if api_keys_val.is_empty() {
            None
        } else {
            Some(api_keys_val.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
        };

        let jwt_secret_val = jwt_secret.get();
        let jwt_algorithm_val = jwt_algorithm.get();
        let jwks_url_val = jwks_url.get();
        let current_mode = auth_mode.get();

        // Convert basic_users Vec back to HashMap
        let basic_users_map: Option<HashMap<String, String>> = if current_mode == "BasicAuth" {
            let users = basic_users.get();
            if users.is_empty() {
                None
            } else {
                Some(users.into_iter().collect())
            }
        } else {
            None
        };

        let settings = ServerSettings {
            auth: AuthConfig {
                enabled: auth_enabled.get(),
                mode: current_mode.clone(),
                api_keys: if current_mode == "ApiKey" { api_keys_vec } else { None },
                jwt_secret: if current_mode == "BearerToken" && !jwt_secret_val.is_empty() { Some(jwt_secret_val) } else { None },
                jwt_algorithm: if current_mode == "BearerToken" { Some(jwt_algorithm_val) } else { None },
                basic_users: basic_users_map,
                jwks_url: if current_mode == "OAuth2" && !jwks_url_val.is_empty() { Some(jwks_url_val) } else { None },
            },
            rate_limit: Some(RateLimitConfig {
                enabled: rate_limit_enabled.get(),
                requests_per_second: requests_per_second.get(),
                burst_size: burst_size.get(),
            }),
            s3: Some(S3Config {
                enabled: s3_enabled.get(),
                bucket: if bucket_val.is_empty() { None } else { Some(bucket_val) },
                prefix: if prefix_val.is_empty() { None } else { Some(prefix_val) },
                region: if region_val.is_empty() { None } else { Some(region_val) },
                endpoint: if endpoint_val.is_empty() { None } else { Some(endpoint_val) },
                poll_interval_secs: s3_poll_interval.get(),
            }),
        };

        wasm_bindgen_futures::spawn_local(async move {
            match api::update_server_settings(&settings).await {
                Ok(_) => {
                    set_message.set(Some(("Settings saved to memory. Restart server to revert changes unless saved to metis.toml.".to_string(), true)));
                }
                Err(e) => {
                    set_message.set(Some((format!("Failed to save: {}", e), false)));
                }
            }
            set_saving.set(false);
        });
    };

    let (saving_disk, set_saving_disk) = signal(false);
    let on_save_disk = move |_| {
        set_saving_disk.set(true);
        wasm_bindgen_futures::spawn_local(async move {
            match api::save_config_to_disk().await {
                Ok(_) => {
                    set_message.set(Some(("Configuration saved to metis.toml successfully!".to_string(), true)));
                }
                Err(e) => {
                    set_message.set(Some((format!("Failed to save to disk: {}", e), false)));
                }
            }
            set_saving_disk.set(false);
        });
    };

    let (saving_s3, set_saving_s3) = signal(false);
    let on_save_s3 = move |_| {
        set_saving_s3.set(true);
        wasm_bindgen_futures::spawn_local(async move {
            match api::save_config_to_s3().await {
                Ok(_) => {
                    set_message.set(Some(("Configuration saved to S3 successfully!".to_string(), true)));
                }
                Err(e) => {
                    set_message.set(Some((format!("Failed to save to S3: {}", e), false)));
                }
            }
            set_saving_s3.set(false);
        });
    };

    // Export config to local file
    let (exporting, set_exporting) = signal(false);
    let on_export = move |_| {
        set_exporting.set(true);
        set_message.set(None);
        wasm_bindgen_futures::spawn_local(async move {
            match api::export_config().await {
                Ok(config_json) => {
                    // Create blob and download
                    let json_str = serde_json::to_string_pretty(&config_json)
                        .unwrap_or_else(|_| config_json.to_string());

                    if let Some(window) = web_sys::window() {
                        if let Some(document) = window.document() {
                            // Create blob
                            let blob_parts = js_sys::Array::new();
                            blob_parts.push(&wasm_bindgen::JsValue::from_str(&json_str));
                            let options = BlobPropertyBag::new();
                            options.set_type("application/json");
                            if let Ok(blob) = Blob::new_with_str_sequence_and_options(&blob_parts, &options) {
                                // Create URL
                                if let Ok(url) = Url::create_object_url_with_blob(&blob) {
                                    // Create download link
                                    if let Ok(link) = document.create_element("a") {
                                        let link: web_sys::HtmlAnchorElement = link.dyn_into().unwrap();
                                        link.set_href(&url);
                                        link.set_download("metis-config.json");
                                        link.click();
                                        let _ = Url::revoke_object_url(&url);
                                        set_message.set(Some(("Configuration exported successfully!".to_string(), true)));
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    set_message.set(Some((format!("Failed to export config: {}", e), false)));
                }
            }
            set_exporting.set(false);
        });
    };

    // Import config from local file (replaces all)
    let (importing, set_importing) = signal(false);
    let file_input_ref = NodeRef::<leptos::html::Input>::new();

    let on_import_click = move |_| {
        if let Some(input) = file_input_ref.get() {
            input.click();
        }
    };

    let on_file_selected = move |_| {
        if let Some(input) = file_input_ref.get() {
            // Get the native DOM element and cast to HtmlInputElement with files support
            let input_el: web_sys::HtmlInputElement = input.clone().into();
            if let Some(files) = input_el.files() {
                if let Some(file) = files.get(0) {
                    set_importing.set(true);
                    set_message.set(None);

                    let reader = FileReader::new().unwrap();
                    let reader_clone = reader.clone();

                    let onload = Closure::once(Box::new(move || {
                        if let Ok(result) = reader_clone.result() {
                            if let Some(text) = result.as_string() {
                                // Parse JSON and import
                                wasm_bindgen_futures::spawn_local(async move {
                                    match serde_json::from_str::<serde_json::Value>(&text) {
                                        Ok(config) => {
                                            match api::import_config(&config).await {
                                                Ok(_) => {
                                                    // Reload the page to reflect imported config
                                                    if let Some(window) = web_sys::window() {
                                                        let _ = window.location().reload();
                                                    }
                                                }
                                                Err(e) => {
                                                    log::error!("Failed to import config: {}", e);
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            log::error!("Failed to parse JSON: {}", e);
                                        }
                                    }
                                });
                            }
                        }
                    }) as Box<dyn FnOnce()>);

                    reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                    onload.forget(); // Prevent closure from being dropped
                    let _ = reader.read_as_text(&file);
                }
            }
        }
    };

    // Merge config from local file (only adds new elements)
    let (merging, set_merging) = signal(false);
    let merge_file_input_ref = NodeRef::<leptos::html::Input>::new();

    let on_merge_click = move |_| {
        if let Some(input) = merge_file_input_ref.get() {
            input.click();
        }
    };

    let on_merge_file_selected = move |_| {
        if let Some(input) = merge_file_input_ref.get() {
            let input_el: web_sys::HtmlInputElement = input.clone().into();
            if let Some(files) = input_el.files() {
                if let Some(file) = files.get(0) {
                    set_merging.set(true);
                    set_message.set(None);

                    let reader = FileReader::new().unwrap();
                    let reader_clone = reader.clone();

                    let onload = Closure::once(Box::new(move || {
                        if let Ok(result) = reader_clone.result() {
                            if let Some(text) = result.as_string() {
                                wasm_bindgen_futures::spawn_local(async move {
                                    match serde_json::from_str::<serde_json::Value>(&text) {
                                        Ok(config) => {
                                            match api::merge_config(&config).await {
                                                Ok(merge_result) => {
                                                    // Show success message with summary, then reload
                                                    log::info!("Merge successful: {}", merge_result.summary());
                                                    if let Some(window) = web_sys::window() {
                                                        let _ = window.location().reload();
                                                    }
                                                }
                                                Err(e) => {
                                                    log::error!("Failed to merge config: {}", e);
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            log::error!("Failed to parse JSON: {}", e);
                                        }
                                    }
                                });
                            }
                        }
                    }) as Box<dyn FnOnce()>);

                    reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                    onload.forget();
                    let _ = reader.read_as_text(&file);
                }
            }
        }
    };

    view! {
        <div class="bg-white dark:bg-gray-800 rounded-lg shadow overflow-hidden">
            <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900">
                <div class="flex justify-between items-center mb-2">
                    <h3 class="text-lg font-semibold text-gray-800 dark:text-white">"Settings Editor"</h3>
                </div>
                <div class="flex flex-wrap gap-2">
                    <button
                        on:click=on_save
                        disabled=move || saving.get()
                        class="px-4 py-2 bg-blue-500 text-white text-sm rounded hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                        {move || if saving.get() { "Saving..." } else { "Save to Memory" }}
                    </button>
                    <button
                        on:click=on_save_disk
                        disabled=move || saving_disk.get()
                        class="px-4 py-2 bg-green-500 text-white text-sm rounded hover:bg-green-600 disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                        {move || if saving_disk.get() { "Saving..." } else { "Save to Disk" }}
                    </button>
                    <button
                        on:click=on_save_s3
                        disabled=move || saving_s3.get()
                        class="px-4 py-2 bg-purple-500 text-white text-sm rounded hover:bg-purple-600 disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                        {move || if saving_s3.get() { "Saving..." } else { "Save to S3" }}
                    </button>
                    <div class="border-l border-gray-300 dark:border-gray-600 h-6 mx-1"></div>
                    <button
                        on:click=on_export
                        disabled=move || exporting.get()
                        class="px-4 py-2 bg-amber-500 text-white text-sm rounded hover:bg-amber-600 disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-1"
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"/>
                        </svg>
                        {move || if exporting.get() { "Exporting..." } else { "Export" }}
                    </button>
                    <button
                        on:click=on_import_click
                        disabled=move || importing.get()
                        class="px-4 py-2 bg-teal-500 text-white text-sm rounded hover:bg-teal-600 disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-1"
                        title="Replace all configuration with imported file"
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"/>
                        </svg>
                        {move || if importing.get() { "Importing..." } else { "Import" }}
                    </button>
                    <button
                        on:click=on_merge_click
                        disabled=move || merging.get()
                        class="px-4 py-2 bg-indigo-500 text-white text-sm rounded hover:bg-indigo-600 disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-1"
                        title="Merge new items from file (keeps existing, adds new)"
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4"/>
                        </svg>
                        {move || if merging.get() { "Merging..." } else { "Merge" }}
                    </button>
                    <input
                        type="file"
                        accept=".json"
                        class="hidden"
                        node_ref=file_input_ref
                        on:change=on_file_selected
                    />
                    <input
                        type="file"
                        accept=".json"
                        class="hidden"
                        node_ref=merge_file_input_ref
                        on:change=on_merge_file_selected
                    />
                </div>
            </div>

            {move || message.get().map(|(msg, success)| {
                let color = if success { "bg-green-100 border-green-400 text-green-700" } else { "bg-red-100 border-red-400 text-red-700" };
                view! {
                    <div class=format!("mx-6 mt-4 p-3 border rounded {}", color)>
                        {msg}
                    </div>
                }
            })}

            <div class="p-6 space-y-8">
                // Authentication Section
                <div>
                    <h4 class="text-md font-semibold text-gray-700 mb-4 flex items-center">
                        <span class="w-2 h-2 bg-blue-500 rounded-full mr-2"></span>
                        "Authentication"
                    </h4>
                    <div class="space-y-4 pl-4">
                        <div class="flex items-center justify-between">
                            <div>
                                <label class="font-medium text-gray-700">"Enable Authentication"</label>
                                <p class="text-sm text-gray-500">"Require authentication for API requests"</p>
                            </div>
                            <ToggleSwitch
                                enabled=auth_enabled
                                on_toggle=move |v| set_auth_enabled.set(v)
                            />
                        </div>

                        <div class=move || if auth_enabled.get() { "" } else { "opacity-50 pointer-events-none" }>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"Auth Mode"</label>
                            <select
                                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                                prop:value=move || auth_mode.get()
                                on:change=move |ev| {
                                    let target = ev.target().unwrap();
                                    let select: web_sys::HtmlSelectElement = target.dyn_into().unwrap();
                                    set_auth_mode.set(select.value());
                                }
                            >
                                <option value="None">"None"</option>
                                <option value="ApiKey">"API Key"</option>
                                <option value="BearerToken">"Bearer Token"</option>
                                <option value="BasicAuth">"Basic Auth"</option>
                                <option value="OAuth2">"OAuth2"</option>
                            </select>
                        </div>

                        // API Key mode fields
                        {move || {
                            let mode = auth_mode.get();
                            let enabled = auth_enabled.get();
                            if mode == "ApiKey" && enabled {
                                view! {
                                    <div class="mt-4 p-4 bg-blue-50 rounded-lg border border-blue-200">
                                        <label class="block text-sm font-medium text-gray-700 mb-1">"API Keys"</label>
                                        <textarea
                                            rows=3
                                            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 font-mono text-sm"
                                            placeholder="key1, key2, key3"
                                            prop:value=move || api_keys.get()
                                            on:input=move |ev| {
                                                let target = ev.target().unwrap();
                                                let textarea: web_sys::HtmlTextAreaElement = target.dyn_into().unwrap();
                                                set_api_keys.set(textarea.value());
                                            }
                                        />
                                        <p class="mt-1 text-xs text-gray-500">"Comma-separated list of valid API keys"</p>
                                    </div>
                                }.into_any()
                            } else {
                                view! { <div></div> }.into_any()
                            }
                        }}

                        // Bearer Token (JWT) mode fields
                        {move || {
                            let mode = auth_mode.get();
                            let enabled = auth_enabled.get();
                            if mode == "BearerToken" && enabled {
                                view! {
                                    <div class="mt-4 p-4 bg-blue-50 rounded-lg border border-blue-200 space-y-4">
                                        <div>
                                            <label class="block text-sm font-medium text-gray-700 mb-1">"JWT Secret"</label>
                                            <input
                                                type="password"
                                                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 font-mono"
                                                placeholder="your-secret-key"
                                                prop:value=move || jwt_secret.get()
                                                on:input=move |ev| {
                                                    let target = ev.target().unwrap();
                                                    let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                                    set_jwt_secret.set(input.value());
                                                }
                                            />
                                            <p class="mt-1 text-xs text-gray-500">"Secret key for JWT validation"</p>
                                        </div>
                                        <div>
                                            <label class="block text-sm font-medium text-gray-700 mb-1">"JWT Algorithm"</label>
                                            <select
                                                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                                                prop:value=move || jwt_algorithm.get()
                                                on:change=move |ev| {
                                                    let target = ev.target().unwrap();
                                                    let select: web_sys::HtmlSelectElement = target.dyn_into().unwrap();
                                                    set_jwt_algorithm.set(select.value());
                                                }
                                            >
                                                <option value="HS256">"HS256"</option>
                                                <option value="HS384">"HS384"</option>
                                                <option value="HS512">"HS512"</option>
                                                <option value="RS256">"RS256"</option>
                                                <option value="RS384">"RS384"</option>
                                                <option value="RS512">"RS512"</option>
                                            </select>
                                        </div>
                                    </div>
                                }.into_any()
                            } else {
                                view! { <div></div> }.into_any()
                            }
                        }}

                        // OAuth2 mode fields
                        {move || {
                            let mode = auth_mode.get();
                            let enabled = auth_enabled.get();
                            if mode == "OAuth2" && enabled {
                                view! {
                                    <div class="mt-4 p-4 bg-blue-50 rounded-lg border border-blue-200">
                                        <label class="block text-sm font-medium text-gray-700 mb-1">"JWKS URL"</label>
                                        <input
                                            type="url"
                                            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                                            placeholder="https://your-auth-server/.well-known/jwks.json"
                                            prop:value=move || jwks_url.get()
                                            on:input=move |ev| {
                                                let target = ev.target().unwrap();
                                                let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                                set_jwks_url.set(input.value());
                                            }
                                        />
                                        <p class="mt-1 text-xs text-gray-500">"URL to fetch JSON Web Key Set for token validation"</p>
                                    </div>
                                }.into_any()
                            } else {
                                view! { <div></div> }.into_any()
                            }
                        }}

                        // BasicAuth user editor
                        {move || {
                            let mode = auth_mode.get();
                            let enabled = auth_enabled.get();
                            if mode == "BasicAuth" && enabled {
                                let add_user = move |_| {
                                    let username = new_username.get();
                                    let password = new_password.get();
                                    if !username.is_empty() && !password.is_empty() {
                                        set_basic_users.update(|users| {
                                            // Check if username already exists
                                            if !users.iter().any(|(u, _)| u == &username) {
                                                users.push((username.clone(), password.clone()));
                                            }
                                        });
                                        set_new_username.set(String::new());
                                        set_new_password.set(String::new());
                                    }
                                };

                                view! {
                                    <div class="mt-4 p-4 bg-blue-50 rounded-lg border border-blue-200 space-y-4">
                                        <div>
                                            <label class="block text-sm font-medium text-gray-700 mb-2">"Basic Auth Users"</label>

                                            // Existing users list
                                            <div class="space-y-2 mb-4">
                                                {move || {
                                                    let users = basic_users.get();
                                                    if users.is_empty() {
                                                        view! {
                                                            <p class="text-sm text-gray-500 italic">"No users configured"</p>
                                                        }.into_any()
                                                    } else {
                                                        view! {
                                                            <div class="bg-white rounded border border-gray-200 divide-y">
                                                                {users.into_iter().enumerate().map(|(idx, (username, _password))| {
                                                                    let username_display = username.clone();
                                                                    view! {
                                                                        <div class="flex items-center justify-between px-3 py-2">
                                                                            <div class="flex items-center gap-2">
                                                                                <svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z"/>
                                                                                </svg>
                                                                                <span class="font-mono text-sm">{username_display}</span>
                                                                                <span class="text-gray-400 text-xs">"(password hidden)"</span>
                                                                            </div>
                                                                            <button
                                                                                type="button"
                                                                                class="text-red-500 hover:text-red-700 text-sm"
                                                                                on:click=move |_| {
                                                                                    set_basic_users.update(|users| {
                                                                                        users.remove(idx);
                                                                                    });
                                                                                }
                                                                            >
                                                                                "Remove"
                                                                            </button>
                                                                        </div>
                                                                    }
                                                                }).collect::<Vec<_>>()}
                                                            </div>
                                                        }.into_any()
                                                    }
                                                }}
                                            </div>

                                            // Add new user form
                                            <div class="flex gap-2">
                                                <input
                                                    type="text"
                                                    placeholder="Username"
                                                    class="flex-1 px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 text-sm"
                                                    prop:value=move || new_username.get()
                                                    on:input=move |ev| {
                                                        let target = ev.target().unwrap();
                                                        let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                                        set_new_username.set(input.value());
                                                    }
                                                />
                                                <input
                                                    type="password"
                                                    placeholder="Password"
                                                    class="flex-1 px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 text-sm"
                                                    prop:value=move || new_password.get()
                                                    on:input=move |ev| {
                                                        let target = ev.target().unwrap();
                                                        let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                                        set_new_password.set(input.value());
                                                    }
                                                    on:keypress=move |ev: web_sys::KeyboardEvent| {
                                                        if ev.key() == "Enter" {
                                                            ev.prevent_default();
                                                            let username = new_username.get();
                                                            let password = new_password.get();
                                                            if !username.is_empty() && !password.is_empty() {
                                                                set_basic_users.update(|users| {
                                                                    if !users.iter().any(|(u, _)| u == &username) {
                                                                        users.push((username.clone(), password.clone()));
                                                                    }
                                                                });
                                                                set_new_username.set(String::new());
                                                                set_new_password.set(String::new());
                                                            }
                                                        }
                                                    }
                                                />
                                                <button
                                                    type="button"
                                                    class="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600 text-sm whitespace-nowrap"
                                                    on:click=add_user
                                                >
                                                    "Add User"
                                                </button>
                                            </div>
                                            <p class="mt-2 text-xs text-gray-500">"Add username/password pairs for Basic Auth authentication"</p>
                                        </div>
                                    </div>
                                }.into_any()
                            } else {
                                view! { <div></div> }.into_any()
                            }
                        }}
                    </div>
                </div>

                // Rate Limiting Section
                <div>
                    <h4 class="text-md font-semibold text-gray-700 mb-4 flex items-center">
                        <span class="w-2 h-2 bg-green-500 rounded-full mr-2"></span>
                        "Rate Limiting"
                    </h4>
                    <div class="space-y-4 pl-4">
                        <div class="flex items-center justify-between">
                            <div>
                                <label class="font-medium text-gray-700">"Enable Rate Limiting"</label>
                                <p class="text-sm text-gray-500">"Limit request rate per client"</p>
                            </div>
                            <ToggleSwitch
                                enabled=rate_limit_enabled
                                on_toggle=move |v| set_rate_limit_enabled.set(v)
                            />
                        </div>

                        <div class=move || format!("grid grid-cols-2 gap-4 {}", if rate_limit_enabled.get() { "" } else { "opacity-50 pointer-events-none" })>
                            <div>
                                <label class="block text-sm font-medium text-gray-700 mb-1">"Requests per Second"</label>
                                <input
                                    type="number"
                                    min="1"
                                    max="10000"
                                    class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                                    prop:value=move || requests_per_second.get().to_string()
                                    on:input=move |ev| {
                                        let target = ev.target().unwrap();
                                        let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                        if let Ok(v) = input.value().parse::<u32>() {
                                            set_requests_per_second.set(v);
                                        }
                                    }
                                />
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-gray-700 mb-1">"Burst Size"</label>
                                <input
                                    type="number"
                                    min="1"
                                    max="1000"
                                    class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                                    prop:value=move || burst_size.get().to_string()
                                    on:input=move |ev| {
                                        let target = ev.target().unwrap();
                                        let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                        if let Ok(v) = input.value().parse::<u32>() {
                                            set_burst_size.set(v);
                                        }
                                    }
                                />
                            </div>
                        </div>
                    </div>
                </div>

                // S3 Configuration Section
                <div>
                    <h4 class="text-md font-semibold text-gray-700 mb-4 flex items-center">
                        <span class="w-2 h-2 bg-purple-500 rounded-full mr-2"></span>
                        "S3 Storage"
                    </h4>
                    <div class="space-y-4 pl-4">
                        <div class="flex items-center justify-between">
                            <div>
                                <label class="font-medium text-gray-700">"Enable S3 Configuration Storage"</label>
                                <p class="text-sm text-gray-500">"Store configuration in AWS S3 or compatible storage"</p>
                            </div>
                            <ToggleSwitch
                                enabled=s3_enabled
                                on_toggle=move |v| set_s3_enabled.set(v)
                            />
                        </div>

                        <div class=move || format!("space-y-4 {}", if s3_enabled.get() { "" } else { "opacity-50 pointer-events-none" })>
                            <div>
                                <label class="block text-sm font-medium text-gray-700 mb-1">"Bucket Name"</label>
                                <input
                                    type="text"
                                    placeholder="my-config-bucket"
                                    class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                                    prop:value=move || s3_bucket.get()
                                    on:input=move |ev| {
                                        let target = ev.target().unwrap();
                                        let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                        set_s3_bucket.set(input.value());
                                    }
                                />
                            </div>

                            <div class="grid grid-cols-2 gap-4">
                                <div>
                                    <label class="block text-sm font-medium text-gray-700 mb-1">"Prefix (optional)"</label>
                                    <input
                                        type="text"
                                        placeholder="config/"
                                        class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                                        prop:value=move || s3_prefix.get()
                                        on:input=move |ev| {
                                            let target = ev.target().unwrap();
                                            let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                            set_s3_prefix.set(input.value());
                                        }
                                    />
                                </div>
                                <div>
                                    <label class="block text-sm font-medium text-gray-700 mb-1">"Region (optional)"</label>
                                    <input
                                        type="text"
                                        placeholder="us-east-1"
                                        class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                                        prop:value=move || s3_region.get()
                                        on:input=move |ev| {
                                            let target = ev.target().unwrap();
                                            let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                            set_s3_region.set(input.value());
                                        }
                                    />
                                </div>
                            </div>

                            <div>
                                <label class="block text-sm font-medium text-gray-700 mb-1">"Custom Endpoint (optional)"</label>
                                <input
                                    type="text"
                                    placeholder="https://s3.example.com"
                                    class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                                    prop:value=move || s3_endpoint.get()
                                    on:input=move |ev| {
                                        let target = ev.target().unwrap();
                                        let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                        set_s3_endpoint.set(input.value());
                                    }
                                />
                                <p class="text-xs text-gray-500 mt-1">"For MinIO, LocalStack, or S3-compatible services"</p>
                            </div>

                            <div>
                                <label class="block text-sm font-medium text-gray-700 mb-1">"Poll Interval (seconds)"</label>
                                <input
                                    type="number"
                                    min="1"
                                    max="3600"
                                    class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                                    prop:value=move || s3_poll_interval.get().to_string()
                                    on:input=move |ev| {
                                        let target = ev.target().unwrap();
                                        let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                        if let Ok(v) = input.value().parse::<u64>() {
                                            set_s3_poll_interval.set(v);
                                        }
                                    }
                                />
                                <p class="text-xs text-gray-500 mt-1">"How often to check for configuration changes"</p>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}

#[component]
fn ToggleSwitch(
    enabled: ReadSignal<bool>,
    on_toggle: impl Fn(bool) + 'static,
) -> impl IntoView {
    view! {
        <button
            type="button"
            class=move || format!(
                "relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 {}",
                if enabled.get() { "bg-blue-600" } else { "bg-gray-200" }
            )
            on:click=move |_| on_toggle(!enabled.get())
        >
            <span
                class=move || format!(
                    "inline-block h-4 w-4 transform rounded-full bg-white transition-transform {}",
                    if enabled.get() { "translate-x-6" } else { "translate-x-1" }
                )
            />
        </button>
    }
}

#[component]
fn FeaturesCard(overview: ConfigOverview) -> impl IntoView {
    view! {
        <div class="bg-white rounded-lg shadow overflow-hidden">
            <div class="px-6 py-4 border-b border-gray-200 bg-gray-50">
                <h3 class="text-lg font-semibold text-gray-800">"Features"</h3>
            </div>
            <div class="p-6">
                <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
                    <FeatureToggle
                        name="Authentication"
                        enabled=overview.auth_enabled
                        description="API key or token-based authentication"
                    />
                    <FeatureToggle
                        name="Rate Limiting"
                        enabled=overview.rate_limit_enabled
                        description="Request rate limiting per client"
                    />
                    <FeatureToggle
                        name="S3 Storage"
                        enabled=overview.s3_enabled
                        description="AWS S3 configuration storage"
                    />
                </div>
            </div>
        </div>
    }
}

#[component]
fn QuickLinksCard(overview: ConfigOverview) -> impl IntoView {
    view! {
        <div class="bg-white rounded-lg shadow overflow-hidden">
            <div class="px-6 py-4 border-b border-gray-200 bg-gray-50">
                <h3 class="text-lg font-semibold text-gray-800">"Configured Items"</h3>
            </div>
            <div class="p-6">
                <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
                    <QuickLinkItem
                        href="/resources"
                        label="Resources"
                        count=overview.resources_count
                        color="blue"
                    />
                    <QuickLinkItem
                        href="/tools"
                        label="Tools"
                        count=overview.tools_count
                        color="green"
                    />
                    <QuickLinkItem
                        href="/prompts"
                        label="Prompts"
                        count=overview.prompts_count
                        color="purple"
                    />
                    <QuickLinkItem
                        href="/workflows"
                        label="Workflows"
                        count=overview.workflows_count
                        color="orange"
                    />
                </div>
            </div>
        </div>
    }
}

#[component]
fn ConfigItem(
    label: &'static str,
    value: String,
    #[prop(optional)] badge_color: Option<&'static str>,
) -> impl IntoView {
    view! {
        <div class="flex items-center justify-between py-2 border-b border-gray-100 last:border-0">
            <span class="text-gray-600 font-medium">{label}</span>
            {match badge_color {
                Some(color) => {
                    let badge_class = match color {
                        "green" => "bg-green-100 text-green-800",
                        "red" => "bg-red-100 text-red-800",
                        "yellow" => "bg-yellow-100 text-yellow-800",
                        _ => "bg-gray-100 text-gray-800",
                    };
                    view! {
                        <span class=format!("px-2 py-1 text-sm font-semibold rounded-full {}", badge_class)>
                            {value}
                        </span>
                    }.into_any()
                }
                None => view! {
                    <span class="font-mono text-gray-900">{value}</span>
                }.into_any(),
            }}
        </div>
    }
}

#[component]
fn FeatureToggle(
    name: &'static str,
    enabled: bool,
    description: &'static str,
) -> impl IntoView {
    let (bg, border, dot_color, status_text, status_color) = if enabled {
        ("bg-green-50", "border-green-200", "bg-green-500", "Enabled", "text-green-700")
    } else {
        ("bg-gray-50", "border-gray-200", "bg-gray-400", "Disabled", "text-gray-500")
    };

    view! {
        <div class=format!("p-4 rounded-lg border {} {}", border, bg)>
            <div class="flex items-center justify-between mb-2">
                <span class="font-semibold text-gray-800">{name}</span>
                <span class=format!("flex items-center text-sm {}", status_color)>
                    <span class=format!("w-2 h-2 rounded-full mr-2 {}", dot_color)></span>
                    {status_text}
                </span>
            </div>
            <p class="text-sm text-gray-500">{description}</p>
        </div>
    }
}

#[component]
fn QuickLinkItem(
    href: &'static str,
    label: &'static str,
    count: usize,
    color: &'static str,
) -> impl IntoView {
    let (bg, text) = match color {
        "blue" => ("bg-blue-50 hover:bg-blue-100 border-blue-200", "text-blue-600"),
        "green" => ("bg-green-50 hover:bg-green-100 border-green-200", "text-green-600"),
        "purple" => ("bg-purple-50 hover:bg-purple-100 border-purple-200", "text-purple-600"),
        "orange" => ("bg-orange-50 hover:bg-orange-100 border-orange-200", "text-orange-600"),
        _ => ("bg-gray-50 hover:bg-gray-100 border-gray-200", "text-gray-600"),
    };

    view! {
        <a
            href=href
            class=format!("block p-4 rounded-lg border {} transition-colors", bg)
        >
            <div class="text-sm text-gray-500 mb-1">{label}</div>
            <div class=format!("text-2xl font-bold {}", text)>{count}</div>
        </a>
    }
}
