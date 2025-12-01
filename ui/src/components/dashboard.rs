use leptos::prelude::*;
use crate::api;
use crate::types::{ConfigOverview, DatabaseStatus};

#[component]
pub fn Dashboard() -> impl IntoView {
    let config = LocalResource::new(|| async move {
        api::get_config().await.ok()
    });

    let db_status = LocalResource::new(|| async move {
        api::get_database_status().await.ok()
    });

    view! {
        <div class="p-6">
            <h2 class="text-2xl font-bold mb-6">"Dashboard"</h2>

            <Suspense fallback=move || view! { <div class="text-gray-500">"Loading..."</div> }>
                {move || {
                    match config.get() {
                        Some(Some(overview)) => {
                            let db = db_status.get().flatten();
                            if !overview.config_file_loaded {
                                // No config file - show dashboard with info banner
                                view! {
                                    <div>
                                        <div class="bg-blue-50 border border-blue-200 rounded-lg p-4 mb-6">
                                            <p class="text-blue-800">
                                                <strong>"No configuration file found."</strong>
                                                " The server is running with default settings. "
                                                <a href="/config" class="underline font-semibold">"Configure server settings"</a>
                                                " to create a configuration file."
                                            </p>
                                        </div>
                                        <ServerInfoCard overview=overview.clone() />
                                        <DatabaseStatusCard status=db />
                                        <StatsGrid overview=overview />
                                    </div>
                                }.into_any()
                            } else {
                                // Config file exists - show normal dashboard
                                view! {
                                    <div>
                                        <ServerInfoCard overview=overview.clone() />
                                        <DatabaseStatusCard status=db />
                                        <StatsGrid overview=overview />
                                    </div>
                                }.into_any()
                            }
                        },
                        Some(None) => view! {
                            <div class="bg-red-50 border border-red-200 rounded-lg p-4">
                                <p class="text-red-800">
                                    <strong>"Failed to load configuration."</strong>
                                    " Please check if the server is running properly."
                                </p>
                            </div>
                        }.into_any(),
                        None => view! {
                            <div class="text-gray-500">"Loading configuration..."</div>
                        }.into_any(),
                    }
                }}
            </Suspense>
        </div>
    }
}

#[component]
fn ServerInfoCard(overview: ConfigOverview) -> impl IntoView {
    view! {
        <div class="bg-white p-4 rounded-lg shadow mb-6">
            <h3 class="text-lg font-semibold text-gray-700 mb-2">"Server Information"</h3>
            <div class="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
                <div>
                    <span class="text-gray-500">"Host: "</span>
                    <span class="font-mono">{overview.server.host}</span>
                </div>
                <div>
                    <span class="text-gray-500">"Port: "</span>
                    <span class="font-mono">{overview.server.port}</span>
                </div>
                <div>
                    <span class="text-gray-500">"Version: "</span>
                    <span class="font-mono">{overview.server.version}</span>
                </div>
                <div>
                    <span class="text-gray-500">"Auth: "</span>
                    <span class={if overview.auth_enabled { "text-green-600" } else { "text-gray-400" }}>
                        {if overview.auth_enabled { "Enabled" } else { "Disabled" }}
                    </span>
                </div>
            </div>
        </div>
    }
}

#[component]
fn DatabaseStatusCard(status: Option<DatabaseStatus>) -> impl IntoView {
    match status {
        Some(db) if db.enabled => {
            let (health_bg, health_text, health_label) = if db.healthy {
                ("bg-green-100", "text-green-700", "Healthy")
            } else {
                ("bg-red-100", "text-red-700", "Unhealthy")
            };

            let backend = db.backend.clone().unwrap_or_else(|| "Unknown".to_string());
            let head_info = db.head.as_ref().map(|h| {
                let short_hash = if h.commit_hash.len() >= 8 {
                    &h.commit_hash[..8]
                } else {
                    &h.commit_hash
                };
                format!("{} - {}", short_hash, h.message)
            }).unwrap_or_else(|| "No commits yet".to_string());

            view! {
                <div class="bg-white p-4 rounded-lg shadow mb-6">
                    <div class="flex items-center justify-between mb-2">
                        <h3 class="text-lg font-semibold text-gray-700">"Database Status"</h3>
                        <span class=format!("px-2 py-1 rounded text-sm font-medium {} {}", health_bg, health_text)>
                            {health_label}
                        </span>
                    </div>
                    <div class="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
                        <div>
                            <span class="text-gray-500">"Backend: "</span>
                            <span class="font-mono">{backend}</span>
                        </div>
                        <div>
                            <span class="text-gray-500">"Commits: "</span>
                            <span class="font-mono">{db.total_commits}</span>
                        </div>
                        <div>
                            <span class="text-gray-500">"Tags: "</span>
                            <span class="font-mono">{db.total_tags}</span>
                        </div>
                        <div class="md:col-span-1">
                            <span class="text-gray-500">"HEAD: "</span>
                            <span class="font-mono text-xs">{head_info}</span>
                        </div>
                    </div>
                </div>
            }.into_any()
        }
        Some(_) => {
            // Database not enabled
            view! {
                <div class="bg-gray-50 p-4 rounded-lg border border-gray-200 mb-6">
                    <div class="flex items-center">
                        <span class="text-gray-500 text-sm">
                            "Database persistence is not configured. Add a "
                            <code class="bg-gray-200 px-1 rounded">"[database]"</code>
                            " section to your config to enable version history."
                        </span>
                    </div>
                </div>
            }.into_any()
        }
        None => {
            // Loading or error
            view! {
                <div class="bg-gray-50 p-4 rounded-lg border border-gray-200 mb-6 animate-pulse">
                    <span class="text-gray-400 text-sm">"Loading database status..."</span>
                </div>
            }.into_any()
        }
    }
}

#[component]
fn StatsGrid(overview: ConfigOverview) -> impl IntoView {
    view! {
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-5 gap-4">
            <StatCard
                title="Resources"
                count=overview.resources_count
                color="blue"
                href="/resources"
                secondary_count=overview.resource_templates_count
                secondary_label="templates"
                secondary_href="/resource-templates"
            />
            <StatCard
                title="Tools"
                count=overview.tools_count
                color="green"
                href="/tools"
            />
            <StatCard
                title="Prompts"
                count=overview.prompts_count
                color="purple"
                href="/prompts"
            />
            <StatCard
                title="Workflows"
                count=overview.workflows_count
                color="orange"
                href="/workflows"
            />
            <StatCard
                title="Agents"
                count=overview.agents_count
                color="indigo"
                href="/agents"
            />
            <StatCard
                title="Schemas"
                count=overview.schemas_count
                color="teal"
                href="/schemas"
            />
            <StatCard
                title="Data Lakes"
                count=overview.data_lakes_count
                color="cyan"
                href="/data-lakes"
            />
        </div>

        <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mt-4">
            <FeatureCard
                title="Rate Limiting"
                enabled=overview.rate_limit_enabled
            />
            <FeatureCard
                title="S3 Config"
                enabled=overview.s3_enabled
            />
            <FeatureCard
                title="Authentication"
                enabled=overview.auth_enabled
            />
        </div>
    }
}

#[component]
fn StatCard(
    title: &'static str,
    count: usize,
    color: &'static str,
    href: &'static str,
    #[prop(optional)] secondary_count: Option<usize>,
    #[prop(optional)] secondary_label: Option<&'static str>,
    #[prop(optional)] secondary_href: Option<&'static str>,
) -> impl IntoView {
    let bg_class = match color {
        "blue" => "bg-blue-50 border-blue-200",
        "green" => "bg-green-50 border-green-200",
        "purple" => "bg-purple-50 border-purple-200",
        "orange" => "bg-orange-50 border-orange-200",
        "indigo" => "bg-indigo-50 border-indigo-200",
        "teal" => "bg-teal-50 border-teal-200",
        "cyan" => "bg-cyan-50 border-cyan-200",
        _ => "bg-gray-50 border-gray-200",
    };

    let text_class = match color {
        "blue" => "text-blue-600",
        "green" => "text-green-600",
        "purple" => "text-purple-600",
        "orange" => "text-orange-600",
        "indigo" => "text-indigo-600",
        "teal" => "text-teal-600",
        "cyan" => "text-cyan-600",
        _ => "text-gray-600",
    };

    view! {
        <div class=format!("p-4 rounded-lg border-2 {} hover:shadow-md transition-shadow", bg_class)>
            <a href=href class="block">
                <h3 class="font-bold text-gray-500 text-sm uppercase tracking-wide">{title}</h3>
                <p class=format!("text-3xl font-bold {}", text_class)>{count}</p>
            </a>
            {secondary_count.map(|sec_count| {
                let label = secondary_label.unwrap_or("templates");
                let link = secondary_href.unwrap_or(href);
                view! {
                    <a href=link class="text-sm text-gray-500 hover:text-gray-700 hover:underline">
                        {format!("{} {}", sec_count, label)}
                    </a>
                }
            })}
        </div>
    }
}

#[component]
fn FeatureCard(title: &'static str, enabled: bool) -> impl IntoView {
    let (bg, text, status) = if enabled {
        ("bg-green-50 border-green-200", "text-green-600", "Enabled")
    } else {
        ("bg-gray-50 border-gray-200", "text-gray-400", "Disabled")
    };

    view! {
        <div class=format!("p-4 rounded-lg border-2 {}", bg)>
            <h3 class="font-bold text-gray-500 text-sm uppercase tracking-wide">{title}</h3>
            <p class=format!("text-lg font-semibold {}", text)>{status}</p>
        </div>
    }
}
