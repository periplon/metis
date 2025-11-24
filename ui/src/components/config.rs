use leptos::prelude::*;
use leptos::web_sys;
use wasm_bindgen::JsCast;
use crate::api;
use crate::types::{ConfigOverview, ServerSettings, AuthConfig, RateLimitConfig};

#[component]
pub fn Config() -> impl IntoView {
    let config = LocalResource::new(|| async move {
        api::get_config().await.ok()
    });

    let settings = LocalResource::new(|| async move {
        api::get_server_settings().await.ok()
    });

    view! {
        <div class="p-6">
            <h2 class="text-2xl font-bold mb-6">"Configuration"</h2>

            <Suspense fallback=move || view! { <div class="text-gray-500">"Loading..."</div> }>
                {move || {
                    let config_data = config.get();
                    let settings_data = settings.get();

                    match (config_data, settings_data) {
                        (Some(Some(overview)), Some(Some(server_settings))) => view! {
                            <div class="space-y-6">
                                <ServerConfigCard overview=overview.clone() />
                                <SettingsEditorCard initial_settings=server_settings />
                                <QuickLinksCard overview=overview />
                            </div>
                        }.into_any(),
                        (Some(Some(overview)), _) => view! {
                            <div class="space-y-6">
                                <ServerConfigCard overview=overview.clone() />
                                <FeaturesCard overview=overview.clone() />
                                <QuickLinksCard overview=overview />
                            </div>
                        }.into_any(),
                        // No config exists - show editor with defaults to create new config
                        _ => {
                            let default_settings = ServerSettings {
                                auth: AuthConfig {
                                    enabled: false,
                                    mode: "None".to_string(),
                                    api_keys: None,
                                    jwt_secret: None,
                                    jwt_algorithm: None,
                                    jwks_url: None,
                                },
                                rate_limit: Some(RateLimitConfig {
                                    enabled: false,
                                    requests_per_second: 100,
                                    burst_size: 10,
                                }),
                            };
                            view! {
                                <div class="space-y-6">
                                    <NewConfigBanner />
                                    <SettingsEditorCard initial_settings=default_settings />
                                </div>
                            }.into_any()
                        },
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
                    <h3 class="text-lg font-semibold text-blue-800">"Create New Configuration"</h3>
                    <p class="mt-1 text-blue-700">
                        "No configuration file found. Configure your settings below and click "
                        <strong>"Save Changes"</strong>
                        " to create a new configuration."
                    </p>
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

    let initial_rate_limit = initial_settings.rate_limit.clone().unwrap_or(RateLimitConfig {
        enabled: false,
        requests_per_second: 100,
        burst_size: 10,
    });

    let (rate_limit_enabled, set_rate_limit_enabled) = signal(initial_rate_limit.enabled);
    let (requests_per_second, set_requests_per_second) = signal(initial_rate_limit.requests_per_second);
    let (burst_size, set_burst_size) = signal(initial_rate_limit.burst_size);

    let (saving, set_saving) = signal(false);
    let (message, set_message) = signal(Option::<(String, bool)>::None);

    let on_save = move |_| {
        set_saving.set(true);
        set_message.set(None);

        let settings = ServerSettings {
            auth: AuthConfig {
                enabled: auth_enabled.get(),
                mode: auth_mode.get(),
                api_keys: None,
                jwt_secret: None,
                jwt_algorithm: None,
                jwks_url: None,
            },
            rate_limit: Some(RateLimitConfig {
                enabled: rate_limit_enabled.get(),
                requests_per_second: requests_per_second.get(),
                burst_size: burst_size.get(),
            }),
        };

        wasm_bindgen_futures::spawn_local(async move {
            match api::update_server_settings(&settings).await {
                Ok(_) => {
                    set_message.set(Some(("Settings saved successfully".to_string(), true)));
                }
                Err(e) => {
                    set_message.set(Some((format!("Failed to save: {}", e), false)));
                }
            }
            set_saving.set(false);
        });
    };

    view! {
        <div class="bg-white rounded-lg shadow overflow-hidden">
            <div class="px-6 py-4 border-b border-gray-200 bg-gray-50 flex justify-between items-center">
                <h3 class="text-lg font-semibold text-gray-800">"Settings Editor"</h3>
                <button
                    on:click=on_save
                    disabled=move || saving.get()
                    class="px-4 py-2 bg-blue-500 text-white text-sm rounded hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed"
                >
                    {move || if saving.get() { "Saving..." } else { "Save Changes" }}
                </button>
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
