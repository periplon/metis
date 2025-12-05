//! Database Strategy Editor Component
//!
//! Provides a rich UI for configuring database mock strategies:
//! - Database type selection (SQLite, PostgreSQL, MySQL, DataFusion)
//! - URL builder for traditional databases
//! - DataFusion configuration for querying datalakes
//! - SQL editor with syntax highlighting and autocompletion

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use crate::types::{DatabaseType, DataFusionConfig, DataLake};
use crate::components::sql_editor::{SqlEditor, TableRef, FieldRef};

/// Database URL builder component for traditional databases
#[component]
pub fn DatabaseUrlBuilder(
    /// Database type being configured
    db_type: ReadSignal<DatabaseType>,
    /// The complete URL (output)
    url: RwSignal<String>,
) -> impl IntoView {
    // Individual URL components
    let (host, set_host) = signal(String::new());
    let (port, set_port) = signal(String::new());
    let (database, set_database) = signal(String::new());
    let (username, set_username) = signal(String::new());
    let (password, set_password) = signal(String::new());
    let (file_path, set_file_path) = signal(String::new());
    let (use_manual_url, set_use_manual_url) = signal(false);

    // Parse existing URL into components when URL changes externally
    Effect::new(move |_| {
        let current_url = url.get();
        if current_url.is_empty() || use_manual_url.get() {
            return;
        }

        // Parse URL based on type
        let db = db_type.get();
        match db {
            DatabaseType::Sqlite => {
                if let Some(path) = current_url.strip_prefix("sqlite://") {
                    set_file_path.set(path.to_string());
                }
            }
            DatabaseType::Postgres | DatabaseType::Mysql => {
                // Parse: postgres://user:pass@host:port/database
                let prefix = if db == DatabaseType::Postgres { "postgres://" } else { "mysql://" };
                if let Some(rest) = current_url.strip_prefix(prefix) {
                    if let Some((auth_host, db_name)) = rest.rsplit_once('/') {
                        set_database.set(db_name.to_string());
                        if let Some((auth, host_port)) = auth_host.rsplit_once('@') {
                            if let Some((user, pass)) = auth.split_once(':') {
                                set_username.set(user.to_string());
                                set_password.set(pass.to_string());
                            }
                            if let Some((h, p)) = host_port.rsplit_once(':') {
                                set_host.set(h.to_string());
                                set_port.set(p.to_string());
                            } else {
                                set_host.set(host_port.to_string());
                            }
                        }
                    }
                }
            }
            DatabaseType::DataFusion => {
                // DataFusion doesn't use URL
            }
        }
    });

    // Build URL from components
    let build_url = move || {
        if use_manual_url.get() {
            return;
        }

        let db = db_type.get();
        let new_url = match db {
            DatabaseType::Sqlite => {
                let path = file_path.get();
                if path.is_empty() {
                    String::new()
                } else {
                    format!("sqlite://{}", path)
                }
            }
            DatabaseType::Postgres => {
                let h = host.get();
                let d = database.get();
                if h.is_empty() || d.is_empty() {
                    String::new()
                } else {
                    let auth = if !username.get().is_empty() {
                        let pass = password.get();
                        if !pass.is_empty() {
                            format!("{}:{}@", username.get(), pass)
                        } else {
                            format!("{}@", username.get())
                        }
                    } else {
                        String::new()
                    };
                    let port_str = if !port.get().is_empty() {
                        format!(":{}", port.get())
                    } else {
                        String::new()
                    };
                    format!("postgres://{}{}{}/{}", auth, h, port_str, d)
                }
            }
            DatabaseType::Mysql => {
                let h = host.get();
                let d = database.get();
                if h.is_empty() || d.is_empty() {
                    String::new()
                } else {
                    let auth = if !username.get().is_empty() {
                        let pass = password.get();
                        if !pass.is_empty() {
                            format!("{}:{}@", username.get(), pass)
                        } else {
                            format!("{}@", username.get())
                        }
                    } else {
                        String::new()
                    };
                    let port_str = if !port.get().is_empty() {
                        format!(":{}", port.get())
                    } else {
                        String::new()
                    };
                    format!("mysql://{}{}{}/{}", auth, h, port_str, d)
                }
            }
            DatabaseType::DataFusion => {
                // DataFusion doesn't use URL
                String::new()
            }
        };
        url.set(new_url);
    };

    // Watch for component changes to rebuild URL
    Effect::new(move |_| {
        let _ = (host.get(), port.get(), database.get(), username.get(), password.get(), file_path.get(), db_type.get());
        build_url();
    });

    let is_sqlite = Memo::new(move |_| db_type.get() == DatabaseType::Sqlite);
    let is_postgres_or_mysql = Memo::new(move |_| {
        let dt = db_type.get();
        dt == DatabaseType::Postgres || dt == DatabaseType::Mysql
    });

    view! {
        <div class="space-y-4">
            // Toggle between builder and manual URL
            <div class="flex items-center gap-3">
                <label class="flex items-center gap-2 cursor-pointer">
                    <input
                        type="checkbox"
                        class="rounded border-gray-300 text-cyan-600 focus:ring-cyan-500"
                        prop:checked=move || use_manual_url.get()
                        on:change=move |ev| {
                            let target = ev.target().unwrap();
                            let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                            set_use_manual_url.set(input.checked());
                        }
                    />
                    <span class="text-sm text-gray-600">"Enter URL manually"</span>
                </label>
            </div>

            <Show when=move || use_manual_url.get()>
                <div>
                    <label class="block text-sm font-medium text-gray-700 mb-1">"Database URL"</label>
                    <input
                        type="text"
                        class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-cyan-500 focus:border-cyan-500 font-mono text-sm"
                        placeholder=move || match db_type.get() {
                            DatabaseType::Sqlite => "sqlite://path/to/database.db",
                            DatabaseType::Postgres => "postgres://user:pass@host:5432/database",
                            DatabaseType::Mysql => "mysql://user:pass@host:3306/database",
                            DatabaseType::DataFusion => "",
                        }
                        prop:value=move || url.get()
                        on:input=move |ev| {
                            let target = ev.target().unwrap();
                            let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                            url.set(input.value());
                        }
                    />
                </div>
            </Show>

            <Show when=move || !use_manual_url.get()>
                // SQLite file path
                <Show when=move || is_sqlite.get()>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">"Database File Path"</label>
                        <input
                            type="text"
                            class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-cyan-500 focus:border-cyan-500 font-mono text-sm"
                            placeholder="path/to/database.db or :memory:"
                            prop:value=move || file_path.get()
                            on:input=move |ev| {
                                let target = ev.target().unwrap();
                                let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                set_file_path.set(input.value());
                            }
                        />
                        <p class="mt-1 text-xs text-gray-500">"Use :memory: for in-memory database"</p>
                    </div>
                </Show>

                // PostgreSQL/MySQL connection details
                <Show when=move || is_postgres_or_mysql.get()>
                    <div class="grid grid-cols-2 gap-4">
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"Host *"</label>
                            <input
                                type="text"
                                class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-cyan-500 focus:border-cyan-500"
                                placeholder="localhost"
                                prop:value=move || host.get()
                                on:input=move |ev| {
                                    let target = ev.target().unwrap();
                                    let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                    set_host.set(input.value());
                                }
                            />
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"Port"</label>
                            <input
                                type="text"
                                class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-cyan-500 focus:border-cyan-500"
                                placeholder=move || if db_type.get() == DatabaseType::Postgres { "5432" } else { "3306" }
                                prop:value=move || port.get()
                                on:input=move |ev| {
                                    let target = ev.target().unwrap();
                                    let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                    set_port.set(input.value());
                                }
                            />
                        </div>
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">"Database Name *"</label>
                        <input
                            type="text"
                            class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-cyan-500 focus:border-cyan-500"
                            placeholder="mydb"
                            prop:value=move || database.get()
                            on:input=move |ev| {
                                let target = ev.target().unwrap();
                                let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                set_database.set(input.value());
                            }
                        />
                    </div>
                    <div class="grid grid-cols-2 gap-4">
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"Username"</label>
                            <input
                                type="text"
                                class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-cyan-500 focus:border-cyan-500"
                                placeholder="postgres"
                                prop:value=move || username.get()
                                on:input=move |ev| {
                                    let target = ev.target().unwrap();
                                    let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                    set_username.set(input.value());
                                }
                            />
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"Password"</label>
                            <input
                                type="password"
                                class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-cyan-500 focus:border-cyan-500"
                                placeholder="****"
                                prop:value=move || password.get()
                                on:input=move |ev| {
                                    let target = ev.target().unwrap();
                                    let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                    set_password.set(input.value());
                                }
                            />
                        </div>
                    </div>
                </Show>
            </Show>

            // Show generated URL preview
            <Show when=move || !use_manual_url.get() && !url.get().is_empty()>
                <div class="p-3 bg-gray-50 rounded-lg">
                    <label class="block text-xs font-medium text-gray-500 mb-1">"Generated URL"</label>
                    <code class="text-sm font-mono text-gray-700 break-all">{move || url.get()}</code>
                </div>
            </Show>
        </div>
    }
}

/// DataFusion configuration component for querying datalakes
/// Storage settings are inherited from the data lake configuration
#[component]
pub fn DataFusionEditor(
    /// DataFusion configuration
    config: RwSignal<DataFusionConfig>,
    /// Available data lakes for selection
    #[prop(into)]
    data_lakes: Signal<Vec<DataLake>>,
) -> impl IntoView {
    // Local signals bound to config fields
    let data_lake = Memo::new(move |_| config.get().data_lake.clone());
    let schema_name = Memo::new(move |_| config.get().schema_name.clone());

    // Get available schemas for selected data lake
    let available_schemas = Memo::new(move |_| {
        let selected_lake = data_lake.get();
        data_lakes.get()
            .into_iter()
            .find(|dl| dl.name == selected_lake)
            .map(|dl| dl.schemas.iter().map(|s| s.schema_name.clone()).collect::<Vec<_>>())
            .unwrap_or_default()
    });

    // Get selected data lake info for display
    let selected_lake_info = Memo::new(move |_| {
        let selected_lake = data_lake.get();
        data_lakes.get()
            .into_iter()
            .find(|dl| dl.name == selected_lake)
    });

    view! {
        <div class="space-y-4">
            <div class="p-3 bg-cyan-50 border border-cyan-200 rounded-lg">
                <p class="text-sm text-cyan-800">
                    <strong>"DataFusion"</strong>" queries data from your Data Lakes using SQL. Select a data lake and schema to query. Storage settings are inherited from the data lake configuration."
                </p>
            </div>

            // Data Lake Selection
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">"Data Lake *"</label>
                <select
                    class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-cyan-500 focus:border-cyan-500"
                    prop:value=move || data_lake.get()
                    on:change=move |ev| {
                        let val = event_target_value(&ev);
                        config.update(|c| {
                            c.data_lake = val;
                            c.schema_name = String::new(); // Reset schema when lake changes
                        });
                    }
                >
                    <option value="">"Select a data lake..."</option>
                    {move || data_lakes.get().into_iter().map(|dl| {
                        let name = dl.name.clone();
                        let name2 = name.clone();
                        view! {
                            <option value=name>{name2}</option>
                        }
                    }).collect::<Vec<_>>()}
                </select>
            </div>

            // Schema Selection (filtered by selected data lake)
            <Show when=move || !data_lake.get().is_empty()>
                <div>
                    <label class="block text-sm font-medium text-gray-700 mb-1">"Schema *"</label>
                    <select
                        class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-cyan-500 focus:border-cyan-500"
                        prop:value=move || schema_name.get()
                        on:change=move |ev| {
                            let val = event_target_value(&ev);
                            config.update(|c| c.schema_name = val);
                        }
                    >
                        <option value="">"Select a schema..."</option>
                        {move || available_schemas.get().into_iter().map(|s| {
                            let s2 = s.clone();
                            view! {
                                <option value=s>{s2}</option>
                            }
                        }).collect::<Vec<_>>()}
                    </select>
                </div>

                // Show data lake storage info
                {move || selected_lake_info.get().map(|dl| {
                    let storage_mode = format!("{:?}", dl.storage_mode);
                    let file_format = format!("{:?}", dl.file_format);
                    view! {
                        <div class="p-3 bg-gray-50 rounded-lg text-sm">
                            <div class="text-gray-600 font-medium mb-1">"Data Lake Storage"</div>
                            <div class="text-gray-500 text-xs space-y-1">
                                <div>"Storage mode: "<span class="font-mono">{storage_mode}</span></div>
                                <div>"File format: "<span class="font-mono">{file_format}</span></div>
                                <div>"Path: "<span class="font-mono">"data_files/"{dl.name.clone()}"/"</span></div>
                            </div>
                        </div>
                    }
                })}
            </Show>
        </div>
    }
}

/// Complete Database Strategy Editor with type selection and SQL editor
#[component]
pub fn DatabaseStrategyEditor(
    /// Database type
    db_type: RwSignal<DatabaseType>,
    /// Database URL (for traditional databases)
    db_url: RwSignal<String>,
    /// SQL query
    db_query: RwSignal<String>,
    /// DataFusion configuration (for DataFusion type)
    datafusion_config: RwSignal<DataFusionConfig>,
    /// Available data lakes for DataFusion selection
    #[prop(into)]
    data_lakes: Signal<Vec<DataLake>>,
) -> impl IntoView {
    // Build table refs for SQL editor autocompletion
    let table_refs = Memo::new(move |_| {
        let lakes = data_lakes.get();
        let mut refs = Vec::new();
        for lake in lakes {
            for schema in &lake.schemas {
                refs.push(TableRef {
                    data_lake: lake.name.clone(),
                    schema: schema.schema_name.clone(),
                    display: format!("{}.{}", lake.name, schema.schema_name),
                });
            }
        }
        refs
    });

    // Build field refs for SQL editor autocompletion
    let field_refs = Memo::new(move |_| {
        let lakes = data_lakes.get();
        let mut refs = Vec::new();
        // Standard data lake columns
        for col in &["id", "data", "data_lake", "schema_name", "created_at", "updated_at", "created_by", "metadata"] {
            refs.push(FieldRef {
                name: col.to_string(),
                field_type: "column".to_string(),
                schema_name: "all".to_string(),
            });
        }
        // Add schema fields from data lakes
        for lake in lakes {
            for schema_ref in &lake.schemas {
                // Would need to load actual schema definitions for detailed fields
                refs.push(FieldRef {
                    name: schema_ref.schema_name.clone(),
                    field_type: "schema".to_string(),
                    schema_name: lake.name.clone(),
                });
            }
        }
        refs
    });

    let is_datafusion = Memo::new(move |_| db_type.get() == DatabaseType::DataFusion);
    let is_traditional = Memo::new(move |_| db_type.get() != DatabaseType::DataFusion);

    view! {
        <div class="space-y-4">
            // Database Type Selection
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">"Database Type"</label>
                <div class="grid grid-cols-4 gap-2">
                    <button
                        type="button"
                        class=move || format!(
                            "px-3 py-2 text-sm font-medium rounded-lg border transition-colors {}",
                            if db_type.get() == DatabaseType::Sqlite {
                                "bg-cyan-100 border-cyan-500 text-cyan-700"
                            } else {
                                "bg-white border-gray-300 text-gray-700 hover:bg-gray-50"
                            }
                        )
                        on:click=move |_| db_type.set(DatabaseType::Sqlite)
                    >
                        "SQLite"
                    </button>
                    <button
                        type="button"
                        class=move || format!(
                            "px-3 py-2 text-sm font-medium rounded-lg border transition-colors {}",
                            if db_type.get() == DatabaseType::Postgres {
                                "bg-cyan-100 border-cyan-500 text-cyan-700"
                            } else {
                                "bg-white border-gray-300 text-gray-700 hover:bg-gray-50"
                            }
                        )
                        on:click=move |_| db_type.set(DatabaseType::Postgres)
                    >
                        "PostgreSQL"
                    </button>
                    <button
                        type="button"
                        class=move || format!(
                            "px-3 py-2 text-sm font-medium rounded-lg border transition-colors {}",
                            if db_type.get() == DatabaseType::Mysql {
                                "bg-cyan-100 border-cyan-500 text-cyan-700"
                            } else {
                                "bg-white border-gray-300 text-gray-700 hover:bg-gray-50"
                            }
                        )
                        on:click=move |_| db_type.set(DatabaseType::Mysql)
                    >
                        "MySQL"
                    </button>
                    <button
                        type="button"
                        class=move || format!(
                            "px-3 py-2 text-sm font-medium rounded-lg border transition-colors {}",
                            if db_type.get() == DatabaseType::DataFusion {
                                "bg-cyan-100 border-cyan-500 text-cyan-700"
                            } else {
                                "bg-white border-gray-300 text-gray-700 hover:bg-gray-50"
                            }
                        )
                        on:click=move |_| db_type.set(DatabaseType::DataFusion)
                    >
                        "DataFusion"
                    </button>
                </div>
            </div>

            // Traditional Database URL Builder
            <Show when=move || is_traditional.get()>
                <div class="border-t border-gray-200 pt-4">
                    <DatabaseUrlBuilder
                        db_type=db_type.read_only()
                        url=db_url
                    />
                </div>
            </Show>

            // DataFusion Configuration
            <Show when=move || is_datafusion.get()>
                <div class="border-t border-gray-200 pt-4">
                    <DataFusionEditor
                        config=datafusion_config
                        data_lakes=data_lakes
                    />
                </div>
            </Show>

            // SQL Query Editor
            <div class="border-t border-gray-200 pt-4">
                <label class="block text-sm font-medium text-gray-700 mb-1">"SQL Query *"</label>
                <SqlEditor
                    value=db_query
                    tables=Signal::derive(move || table_refs.get())
                    fields=Signal::derive(move || field_refs.get())
                    placeholder="SELECT * FROM $table WHERE ..."
                    rows=6
                />
                <div class="mt-2 text-xs text-gray-500 space-y-1">
                    <Show when=move || is_datafusion.get()>
                        <p>"Use "<code class="bg-gray-100 px-1 rounded">"$table"</code>" as placeholder for the selected data lake table."</p>
                        <p>"JSON functions: "<code class="bg-gray-100 px-1 rounded">"json_get_str(data, 'field')"</code>", "<code class="bg-gray-100 px-1 rounded">"json_get_int(data, 'field')"</code>", etc."</p>
                    </Show>
                    <Show when=move || !is_datafusion.get()>
                        <p>"Use "<code class="bg-gray-100 px-1 rounded">"$1"</code>", "<code class="bg-gray-100 px-1 rounded">"$2"</code>", etc. for parameter binding."</p>
                    </Show>
                </div>
            </div>
        </div>
    }
}
