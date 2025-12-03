//! Data Lakes management page
//!
//! Provides CRUD operations for Data Lakes (collections of schema references)
//! and their associated Data Records. Supports manual record entry with
//! auto-generated forms and bulk data generation via mock strategies.

use leptos::prelude::*;
use leptos::web_sys;
use leptos_router::hooks::use_params_map;
use leptos_router::components::A;
use std::collections::HashSet;
use js_sys;
use crate::api;
use crate::types::{
    DataLake, DataLakeSchemaRef, DataRecord, CreateRecordRequest, UpdateRecordRequest,
    GenerateRecordsRequest, DataLakeStorageMode, DataLakeFileFormat,
    SqlQueryRequest, SqlQueryResponse, SyncRequest, FileInfo,
};
use crate::components::list_filter::{
    ListFilterBar, Pagination, TagBadges, TagInput,
    extract_tags, filter_items, paginate_items, total_pages,
    SortField, SortOrder, sort_items,
};
use crate::components::schema_form::{SchemaFormGenerator, SchemaFormMode};
use crate::components::sql_editor::{SqlEditor, TableSelector, TableRef, FieldRef, extract_fields_from_schema};

#[derive(Clone, Copy, PartialEq)]
enum ViewMode {
    Table,
    Card,
}

/// Tab for Data Lake create/edit forms
#[derive(Clone, Copy, PartialEq)]
enum DataLakeFormTab {
    Basic,
    Storage,
}

/// Tab for Data Lake records page
#[derive(Clone, Copy, PartialEq)]
enum DataLakeRecordsTab {
    Records,
    Query,
    Files,
}

/// Main Data Lakes list page
#[component]
pub fn DataLakes() -> impl IntoView {
    let (view_mode, set_view_mode) = signal(ViewMode::Table);
    let (refresh_trigger, set_refresh_trigger) = signal(0u32);
    let (delete_target, set_delete_target) = signal(Option::<String>::None);
    let (deleting, set_deleting) = signal(false);

    // Search, filter, and sort state
    let search_query = RwSignal::new(String::new());
    let selected_tags = RwSignal::new(HashSet::<String>::new());
    let sort_field = RwSignal::new(SortField::Name);
    let sort_order = RwSignal::new(SortOrder::Ascending);
    let current_page = RwSignal::new(0usize);
    let items_per_page = 10usize;
    // Available tags - updated when data lakes load
    let available_tags = RwSignal::new(Vec::<String>::new());

    let data_lakes = LocalResource::new(move || {
        let _ = refresh_trigger.get();
        async move { api::list_data_lakes().await.ok() }
    });

    // Reset page when filters or sort change
    Effect::new(move || {
        let _ = search_query.get();
        let _ = selected_tags.get();
        let _ = sort_field.get();
        let _ = sort_order.get();
        current_page.set(0);
    });

    let on_delete_confirm = move |_| {
        if let Some(name) = delete_target.get() {
            set_deleting.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                match api::delete_data_lake(&name).await {
                    Ok(_) => {
                        set_delete_target.set(None);
                        set_refresh_trigger.update(|n| *n += 1);
                    }
                    Err(e) => {
                        web_sys::window()
                            .and_then(|w| w.alert_with_message(&format!("Failed to delete: {}", e)).ok());
                    }
                }
                set_deleting.set(false);
            });
        }
    };

    view! {
        <div class="p-6">
            // Header with title, view toggle, and new button
            <div class="flex justify-between items-center mb-6">
                <h2 class="text-2xl font-bold">"Data Lakes"</h2>
                <div class="flex items-center gap-4">
                    // View mode toggle
                    <div class="flex bg-gray-100 rounded-lg p-1">
                        <button
                            class=move || format!(
                                "px-3 py-1 rounded text-sm font-medium transition-colors {}",
                                if view_mode.get() == ViewMode::Table { "bg-white shadow text-gray-900" } else { "text-gray-600 hover:text-gray-900" }
                            )
                            on:click=move |_| set_view_mode.set(ViewMode::Table)
                        >
                            <span class="flex items-center gap-1">
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 10h16M4 14h16M4 18h16"/>
                                </svg>
                                "Table"
                            </span>
                        </button>
                        <button
                            class=move || format!(
                                "px-3 py-1 rounded text-sm font-medium transition-colors {}",
                                if view_mode.get() == ViewMode::Card { "bg-white shadow text-gray-900" } else { "text-gray-600 hover:text-gray-900" }
                            )
                            on:click=move |_| set_view_mode.set(ViewMode::Card)
                        >
                            <span class="flex items-center gap-1">
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z"/>
                                </svg>
                                "Cards"
                            </span>
                        </button>
                    </div>
                    <A href="/data-lakes/new" attr:class="px-4 py-2 bg-cyan-600 text-white rounded-lg hover:bg-cyan-700 flex items-center gap-2">
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/>
                        </svg>
                        "New Data Lake"
                    </A>
                </div>
            </div>

            // Filter bar - rendered once outside reactive block to prevent focus loss
            <ListFilterBar
                search_query=search_query
                selected_tags=selected_tags
                available_tags=Signal::derive(move || available_tags.get())
                sort_field=sort_field
                sort_order=sort_order
                placeholder="Search data lakes by name or description..."
            />

            // Content area
            <Suspense fallback=move || view! { <div class="text-gray-500">"Loading data lakes..."</div> }>
                {move || {
                    data_lakes.get().map(|maybe_lakes| {
                        match maybe_lakes {
                            Some(list) if !list.is_empty() => {
                                // Update available tags signal
                                let tags = extract_tags(&list, |d: &DataLake| d.tags.as_slice());
                                available_tags.set(tags);

                                // Filter items based on search and tags
                                let mut filtered = filter_items(
                                    &list,
                                    &search_query.get(),
                                    &selected_tags.get(),
                                    |d: &DataLake| format!("{} {}", d.name, d.description.as_deref().unwrap_or("")),
                                    |d: &DataLake| d.tags.as_slice(),
                                );

                                // Sort the filtered items
                                sort_items(&mut filtered, sort_field.get(), sort_order.get(), |d: &DataLake| &d.name);

                                let total_filtered = filtered.len();
                                let total_count = list.len();
                                let pages = total_pages(total_filtered, items_per_page);
                                let paginated = paginate_items(&filtered, current_page.get(), items_per_page);

                                view! {
                                    <div>
                                        // Results info
                                        {(total_filtered != total_count).then(|| view! {
                                            <p class="text-sm text-gray-500 mb-4">
                                                "Showing " {total_filtered} " of " {total_count} " data lakes"
                                            </p>
                                        })}

                                        // Content
                                        {if paginated.is_empty() {
                                            view! { <NoResultsState /> }.into_any()
                                        } else if view_mode.get() == ViewMode::Table {
                                    view! {
                                        <div class="bg-white rounded-lg shadow overflow-hidden">
                                            <table class="min-w-full divide-y divide-gray-200">
                                                <thead class="bg-gray-50">
                                                    <tr>
                                                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Name"</th>
                                                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Description"</th>
                                                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Tags"</th>
                                                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Schemas"</th>
                                                        <th class="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">"Actions"</th>
                                                    </tr>
                                                </thead>
                                                <tbody class="bg-white divide-y divide-gray-200">
                                                    {paginated.into_iter().map(|lake| {
                                                        let name = lake.name.clone();
                                                        let name_for_delete = lake.name.clone();
                                                        let name_for_edit = lake.name.clone();
                                                        let name_for_records = lake.name.clone();
                                                        let tags = lake.tags.clone();
                                                        let schema_count = lake.schemas.len();
                                                        view! {
                                                            <tr class="hover:bg-gray-50">
                                                                <td class="px-6 py-4 whitespace-nowrap">
                                                                    <div class="text-sm font-medium text-gray-900">{name.clone()}</div>
                                                                </td>
                                                                <td class="px-6 py-4">
                                                                    <div class="text-sm text-gray-500 max-w-md truncate">
                                                                        {lake.description.clone().unwrap_or_else(|| "—".to_string())}
                                                                    </div>
                                                                </td>
                                                                <td class="px-6 py-4">
                                                                    <TagBadges tags=tags />
                                                                </td>
                                                                <td class="px-6 py-4 whitespace-nowrap">
                                                                    <span class="px-2 inline-flex text-xs leading-5 font-semibold rounded-full bg-cyan-100 text-cyan-800">
                                                                        {schema_count} " schemas"
                                                                    </span>
                                                                </td>
                                                                <td class="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
                                                                    <A href=format!("/data-lakes/{}/records", name_for_records) attr:class="text-cyan-600 hover:text-cyan-900 mr-4">"Records"</A>
                                                                    <A href=format!("/data-lakes/edit/{}", name_for_edit) attr:class="text-cyan-600 hover:text-cyan-900 mr-4">"Edit"</A>
                                                                    <button
                                                                        class="text-red-600 hover:text-red-900"
                                                                        on:click=move |_| set_delete_target.set(Some(name_for_delete.clone()))
                                                                    >
                                                                        "Delete"
                                                                    </button>
                                                                </td>
                                                            </tr>
                                                        }
                                                    }).collect::<Vec<_>>()}
                                                </tbody>
                                            </table>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                                            {paginated.into_iter().map(|lake| {
                                                let name = lake.name.clone();
                                                let name_for_delete = lake.name.clone();
                                                let name_for_edit = lake.name.clone();
                                                let name_for_records = lake.name.clone();
                                                let tags = lake.tags.clone();
                                                let schema_count = lake.schemas.len();
                                                view! {
                                                    <div class="bg-white rounded-lg shadow p-6 hover:shadow-lg transition-shadow">
                                                        <div class="flex justify-between items-start mb-4">
                                                            <h3 class="text-lg font-semibold text-gray-900">{name.clone()}</h3>
                                                            <span class="px-2 py-1 text-xs font-semibold rounded-full bg-cyan-100 text-cyan-800">
                                                                {schema_count} " schemas"
                                                            </span>
                                                        </div>
                                                        <p class="text-sm text-gray-500 mb-4 line-clamp-2">
                                                            {lake.description.clone().unwrap_or_else(|| "No description".to_string())}
                                                        </p>
                                                        // Tags section
                                                        {(!tags.is_empty()).then(|| view! {
                                                            <div class="mb-3">
                                                                <TagBadges tags=tags />
                                                            </div>
                                                        })}
                                                        // Show schema names
                                                        <div class="mb-4">
                                                            <div class="flex flex-wrap gap-1">
                                                                {lake.schemas.iter().take(3).map(|s| {
                                                                    view! {
                                                                        <span class="px-2 py-0.5 text-xs bg-gray-100 rounded text-gray-600">
                                                                            {s.schema_name.clone()}
                                                                        </span>
                                                                    }
                                                                }).collect::<Vec<_>>()}
                                                                {if lake.schemas.len() > 3 {
                                                                    Some(view! {
                                                                        <span class="px-2 py-0.5 text-xs bg-gray-100 rounded text-gray-600">
                                                                            {"+"}{lake.schemas.len() - 3}{" more"}
                                                                        </span>
                                                                    })
                                                                } else {
                                                                    None
                                                                }}
                                                            </div>
                                                        </div>
                                                        <div class="flex justify-between items-center pt-4 border-t border-gray-100">
                                                            <A href=format!("/data-lakes/{}/records", name_for_records) attr:class="px-3 py-1 text-sm bg-cyan-50 text-cyan-700 hover:bg-cyan-100 rounded">"View Records"</A>
                                                            <div class="flex gap-2">
                                                                <A href=format!("/data-lakes/edit/{}", name_for_edit) attr:class="px-3 py-1 text-sm text-cyan-600 hover:bg-cyan-50 rounded">"Edit"</A>
                                                                <button
                                                                    class="px-3 py-1 text-sm text-red-600 hover:bg-red-50 rounded"
                                                                    on:click=move |_| set_delete_target.set(Some(name_for_delete.clone()))
                                                                >
                                                                    "Delete"
                                                                </button>
                                                            </div>
                                                        </div>
                                                    </div>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    }.into_any()
                                        }}

                                        // Pagination
                                        <Pagination
                                            current_page=current_page
                                            total_pages=Signal::derive(move || pages)
                                            total_items=Signal::derive(move || total_filtered)
                                            items_per_page=items_per_page
                                        />
                                    </div>
                                }.into_any()
                            }
                            _ => view! {
                                <div class="bg-white rounded-lg shadow p-8 text-center">
                                    <div class="text-gray-400 mb-4">
                                        <svg class="w-16 h-16 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7v10c0 2 1 3 3 3h10c2 0 3-1 3-3V7c0-2-1-3-3-3H7c-2 0-3 1-3 3zm0 5h16"/>
                                        </svg>
                                    </div>
                                    <h3 class="text-lg font-medium text-gray-900 mb-2">"No data lakes defined"</h3>
                                    <p class="text-gray-500 mb-4">"Data Lakes are collections of schemas that store structured data records. Create a data lake to start managing your test data."</p>
                                    <A href="/data-lakes/new" attr:class="inline-flex items-center px-4 py-2 bg-cyan-600 text-white rounded-lg hover:bg-cyan-700">
                                        <svg class="w-5 h-5 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/>
                                        </svg>
                                        "Create your first data lake"
                                    </A>
                                </div>
                            }.into_any()
                        }
                    })
                }}
            </Suspense>

            // Delete confirmation modal
            <Show when=move || delete_target.get().is_some()>
                <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
                    <div class="bg-white rounded-lg p-6 max-w-sm w-full mx-4">
                        <h3 class="text-lg font-semibold mb-4">"Delete Data Lake?"</h3>
                        <p class="text-gray-600 mb-2">
                            "Are you sure you want to delete data lake "
                            <strong>{move || delete_target.get().unwrap_or_default()}</strong>
                            "?"
                        </p>
                        <p class="text-red-600 text-sm mb-6">
                            "Warning: All records in this data lake will also be deleted."
                        </p>
                        <div class="flex justify-end gap-3">
                            <button
                                class="px-4 py-2 text-gray-600 hover:bg-gray-100 rounded"
                                on:click=move |_| set_delete_target.set(None)
                            >
                                "Cancel"
                            </button>
                            <button
                                class="px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700 disabled:opacity-50"
                                disabled=move || deleting.get()
                                on:click=on_delete_confirm
                            >
                                {move || if deleting.get() { "Deleting..." } else { "Delete" }}
                            </button>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    }
}

#[component]
fn NoResultsState() -> impl IntoView {
    view! {
        <div class="text-center py-12 bg-white rounded-lg shadow">
            <svg class="mx-auto h-12 w-12 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/>
            </svg>
            <h3 class="mt-2 text-sm font-medium text-gray-900">"No matching data lakes"</h3>
            <p class="mt-1 text-sm text-gray-500">"Try adjusting your search or filter criteria."</p>
        </div>
    }
}

/// Data Lake create form
#[component]
pub fn DataLakeForm() -> impl IntoView {
    let (active_tab, set_active_tab) = signal(DataLakeFormTab::Basic);
    let (name, set_name) = signal(String::new());
    let (description, set_description) = signal(String::new());
    let tags = RwSignal::new(Vec::<String>::new());
    let (schema_refs, set_schema_refs) = signal(Vec::<DataLakeSchemaRef>::new());
    let (saving, set_saving) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);

    // Storage configuration signals
    let (storage_mode, set_storage_mode) = signal(DataLakeStorageMode::Database);
    let (file_format, set_file_format) = signal(DataLakeFileFormat::Parquet);
    let (enable_sql_queries, set_enable_sql_queries) = signal(false);

    // Load available schemas for selection
    let available_schemas = LocalResource::new(|| async move {
        api::list_schemas().await.ok().unwrap_or_default()
    });

    let add_schema_ref = move |_| {
        set_schema_refs.update(|refs| {
            refs.push(DataLakeSchemaRef {
                schema_name: String::new(),
                schema_version: None,
                alias: None,
            });
        });
    };

    let remove_schema_ref = move |index: usize| {
        set_schema_refs.update(|refs| {
            if index < refs.len() {
                refs.remove(index);
            }
        });
    };

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();

        // Validate at least one schema
        if schema_refs.get().is_empty() || schema_refs.get().iter().all(|s| s.schema_name.is_empty()) {
            set_error.set(Some("Please add at least one schema reference".to_string()));
            return;
        }

        set_saving.set(true);
        set_error.set(None);

        let name_val = name.get();
        let desc_val = description.get();
        let schemas = schema_refs.get().into_iter().filter(|s| !s.schema_name.is_empty()).collect::<Vec<_>>();
        let storage_mode_val = storage_mode.get();
        let file_format_val = file_format.get();
        let enable_sql_val = enable_sql_queries.get();

        let data_lake = DataLake {
            name: name_val,
            description: if desc_val.is_empty() { None } else { Some(desc_val) },
            tags: tags.get(),
            schemas,
            metadata: None,
            storage_mode: storage_mode_val,
            file_format: file_format_val,
            enable_sql_queries: enable_sql_val,
        };

        wasm_bindgen_futures::spawn_local(async move {
            match api::create_data_lake(&data_lake).await {
                Ok(_) => {
                    let window = web_sys::window().unwrap();
                    window.location().set_href("/data-lakes").ok();
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_saving.set(false);
                }
            }
        });
    };

    view! {
        <div class="p-6 max-w-4xl mx-auto">
            <div class="flex items-center gap-4 mb-6">
                <A href="/data-lakes" attr:class="text-gray-500 hover:text-gray-700">
                    <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18"/>
                    </svg>
                </A>
                <h2 class="text-2xl font-bold">"New Data Lake"</h2>
            </div>

            <Show when=move || error.get().is_some()>
                <div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded mb-6">
                    {move || error.get().unwrap_or_default()}
                </div>
            </Show>

            <form on:submit=on_submit class="bg-white rounded-lg shadow p-6">
                // Tab Navigation
                <div class="border-b border-gray-200 mb-6">
                    <nav class="flex -mb-px space-x-8">
                        <button
                            type="button"
                            class=move || format!(
                                "py-2 px-1 border-b-2 font-medium text-sm transition-colors {}",
                                if active_tab.get() == DataLakeFormTab::Basic {
                                    "border-cyan-500 text-cyan-600"
                                } else {
                                    "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"
                                }
                            )
                            on:click=move |_| set_active_tab.set(DataLakeFormTab::Basic)
                        >
                            <span class="flex items-center gap-2">
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
                                </svg>
                                "Basic"
                            </span>
                        </button>
                        <button
                            type="button"
                            class=move || format!(
                                "py-2 px-1 border-b-2 font-medium text-sm transition-colors {}",
                                if active_tab.get() == DataLakeFormTab::Storage {
                                    "border-cyan-500 text-cyan-600"
                                } else {
                                    "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"
                                }
                            )
                            on:click=move |_| set_active_tab.set(DataLakeFormTab::Storage)
                        >
                            <span class="flex items-center gap-2">
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 8h14M5 8a2 2 0 110-4h14a2 2 0 110 4M5 8v10a2 2 0 002 2h10a2 2 0 002-2V8m-9 4h4"/>
                                </svg>
                                "Storage"
                            </span>
                        </button>
                    </nav>
                </div>

                // Basic Tab Content
                <div style=move || if active_tab.get() == DataLakeFormTab::Basic { "display: block" } else { "display: none" }>
                    <div class="space-y-6">
                        // Name
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"Name"</label>
                            <input
                                type="text"
                                required=true
                                class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-cyan-500 focus:border-cyan-500"
                                placeholder="e.g., users, orders, products"
                                prop:value=move || name.get()
                                on:input=move |ev| set_name.set(event_target_value(&ev))
                            />
                        </div>

                        // Description
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"Description"</label>
                            <textarea
                                class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-cyan-500 focus:border-cyan-500"
                                rows="2"
                                placeholder="Brief description of this data lake"
                                prop:value=move || description.get()
                                on:input=move |ev| set_description.set(event_target_value(&ev))
                            />
                        </div>

                        // Tags
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"Tags"</label>
                            <TagInput tags=tags />
                        </div>

                        // Schema References
                        <div>
                            <div class="flex justify-between items-center mb-2">
                                <label class="block text-sm font-medium text-gray-700">"Schema References"</label>
                                <button
                                    type="button"
                                    class="px-3 py-1 text-sm bg-cyan-50 text-cyan-700 rounded hover:bg-cyan-100"
                                    on:click=add_schema_ref
                                >
                                    "+ Add Schema"
                                </button>
                            </div>
                            <p class="text-xs text-gray-500 mb-3">
                                "Select schemas that will be used to store records in this data lake."
                            </p>

                        <Suspense fallback=move || view! { <div class="text-gray-500 text-sm">"Loading schemas..."</div> }>
                            {move || {
                                let schemas = available_schemas.get().map(|s| s).unwrap_or_default();
                                if schemas.is_empty() {
                                    view! {
                                        <div class="bg-yellow-50 border border-yellow-200 text-yellow-700 px-4 py-3 rounded text-sm">
                                            "No schemas available. "
                                            <A href="/schemas/new" attr:class="underline">"Create a schema first"</A>
                                            " to use in this data lake."
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div class="space-y-3">
                                            <For
                                                each=move || {
                                                    schema_refs.get().into_iter().enumerate().collect::<Vec<_>>()
                                                }
                                                key=|(i, _)| *i
                                                children=move |(index, schema_ref)| {
                                                    let schemas_for_select = schemas.clone();
                                                    view! {
                                                        <div class="flex items-start gap-3 p-3 bg-gray-50 rounded-lg">
                                                            <div class="flex-1 grid grid-cols-3 gap-3">
                                                                <div>
                                                                    <label class="block text-xs text-gray-500 mb-1">"Schema"</label>
                                                                    <select
                                                                        class="w-full px-2 py-1.5 text-sm border border-gray-300 rounded focus:ring-cyan-500 focus:border-cyan-500"
                                                                        prop:value=schema_ref.schema_name.clone()
                                                                        on:change=move |ev| {
                                                                            let value = event_target_value(&ev);
                                                                            set_schema_refs.update(|refs| {
                                                                                if let Some(r) = refs.get_mut(index) {
                                                                                    r.schema_name = value;
                                                                                }
                                                                            });
                                                                        }
                                                                    >
                                                                        <option value="">"Select schema..."</option>
                                                                        {schemas_for_select.iter().map(|s| {
                                                                            let schema_name = s.name.clone();
                                                                            let selected = schema_ref.schema_name == schema_name;
                                                                            view! {
                                                                                <option value=schema_name.clone() selected=selected>{schema_name.clone()}</option>
                                                                            }
                                                                        }).collect::<Vec<_>>()}
                                                                    </select>
                                                                </div>
                                                                <div>
                                                                    <label class="block text-xs text-gray-500 mb-1">"Version (optional)"</label>
                                                                    <input
                                                                        type="text"
                                                                        class="w-full px-2 py-1.5 text-sm border border-gray-300 rounded focus:ring-cyan-500 focus:border-cyan-500"
                                                                        placeholder="latest"
                                                                        prop:value=schema_ref.schema_version.clone().unwrap_or_default()
                                                                        on:input=move |ev| {
                                                                            let value = event_target_value(&ev);
                                                                            set_schema_refs.update(|refs| {
                                                                                if let Some(r) = refs.get_mut(index) {
                                                                                    r.schema_version = if value.is_empty() { None } else { Some(value) };
                                                                                }
                                                                            });
                                                                        }
                                                                    />
                                                                </div>
                                                                <div>
                                                                    <label class="block text-xs text-gray-500 mb-1">"Alias (optional)"</label>
                                                                    <input
                                                                        type="text"
                                                                        class="w-full px-2 py-1.5 text-sm border border-gray-300 rounded focus:ring-cyan-500 focus:border-cyan-500"
                                                                        placeholder="e.g., User"
                                                                        prop:value=schema_ref.alias.clone().unwrap_or_default()
                                                                        on:input=move |ev| {
                                                                            let value = event_target_value(&ev);
                                                                            set_schema_refs.update(|refs| {
                                                                                if let Some(r) = refs.get_mut(index) {
                                                                                    r.alias = if value.is_empty() { None } else { Some(value) };
                                                                                }
                                                                            });
                                                                        }
                                                                    />
                                                                </div>
                                                            </div>
                                                            <button
                                                                type="button"
                                                                class="mt-5 p-1 text-red-500 hover:bg-red-50 rounded"
                                                                on:click=move |_| remove_schema_ref(index)
                                                            >
                                                                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/>
                                                                </svg>
                                                            </button>
                                                        </div>
                                                    }
                                                }
                                            />
                                            {move || {
                                                if schema_refs.get().is_empty() {
                                                    Some(view! {
                                                        <div class="text-center py-6 border-2 border-dashed border-gray-200 rounded-lg">
                                                            <p class="text-gray-500 text-sm">"No schemas added yet"</p>
                                                            <button
                                                                type="button"
                                                                class="mt-2 text-cyan-600 text-sm hover:text-cyan-700"
                                                                on:click=add_schema_ref
                                                            >
                                                                "+ Add your first schema"
                                                            </button>
                                                        </div>
                                                    })
                                                } else {
                                                    None
                                                }
                                            }}
                                        </div>
                                    }.into_any()
                                }
                            }}
                        </Suspense>
                        </div>
                    </div>
                </div>

                // Storage Tab Content
                <div style=move || if active_tab.get() == DataLakeFormTab::Storage { "display: block" } else { "display: none" }>
                    <div class="space-y-6">
                        // Storage Mode
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"Storage Mode"</label>
                            <select
                                class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-cyan-500 focus:border-cyan-500"
                                on:change=move |ev| {
                                    let value = event_target_value(&ev);
                                    set_storage_mode.set(match value.as_str() {
                                        "file" => DataLakeStorageMode::File,
                                        "both" => DataLakeStorageMode::Both,
                                        _ => DataLakeStorageMode::Database,
                                    });
                                }
                            >
                                <option value="database" selected=move || storage_mode.get() == DataLakeStorageMode::Database>"Database"</option>
                                <option value="file" selected=move || storage_mode.get() == DataLakeStorageMode::File>"File"</option>
                                <option value="both" selected=move || storage_mode.get() == DataLakeStorageMode::Both>"Both"</option>
                            </select>
                            <p class="text-xs text-gray-500 mt-1">
                                "Database: Store in database only (default). File: Store as Parquet/JSONL files. Both: Write-through to database and files."
                            </p>
                        </div>

                        // File Format (shown when storage mode includes files)
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"File Format"</label>
                            <select
                                class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-cyan-500 focus:border-cyan-500"
                                disabled=move || storage_mode.get() == DataLakeStorageMode::Database
                                on:change=move |ev| {
                                    let value = event_target_value(&ev);
                                    set_file_format.set(match value.as_str() {
                                        "jsonl" => DataLakeFileFormat::Jsonl,
                                        _ => DataLakeFileFormat::Parquet,
                                    });
                                }
                            >
                                <option value="parquet" selected=move || file_format.get() == DataLakeFileFormat::Parquet>"Parquet"</option>
                                <option value="jsonl" selected=move || file_format.get() == DataLakeFileFormat::Jsonl>"JSONL"</option>
                            </select>
                            <p class="text-xs text-gray-500 mt-1">
                                "Parquet: Columnar format, best for analytics. JSONL: Human-readable, good for streaming."
                            </p>
                        </div>

                        // Enable SQL Queries
                        <div class="flex items-center gap-3">
                            <input
                                type="checkbox"
                                id="enable_sql_queries"
                                class="h-4 w-4 text-cyan-600 focus:ring-cyan-500 border-gray-300 rounded"
                                prop:checked=move || enable_sql_queries.get()
                                on:change=move |ev| {
                                    let target = event_target::<web_sys::HtmlInputElement>(&ev);
                                    set_enable_sql_queries.set(target.checked());
                                }
                            />
                            <label for="enable_sql_queries" class="text-sm font-medium text-gray-700">
                                "Enable SQL Queries"
                            </label>
                        </div>
                        <p class="text-xs text-gray-500 -mt-4 ml-7">
                            "Allow SQL queries via DataFusion on this data lake's files."
                        </p>
                    </div>
                </div>

                // Submit button
                <div class="flex justify-end gap-3 mt-6 pt-6 border-t border-gray-200">
                    <A href="/data-lakes" attr:class="px-4 py-2 text-gray-600 hover:bg-gray-100 rounded-lg">
                        "Cancel"
                    </A>
                    <button
                        type="submit"
                        class="px-4 py-2 bg-cyan-600 text-white rounded-lg hover:bg-cyan-700 disabled:opacity-50"
                        disabled=move || saving.get()
                    >
                        {move || if saving.get() { "Creating..." } else { "Create Data Lake" }}
                    </button>
                </div>
            </form>
        </div>
    }
}

/// Data Lake edit form
#[component]
pub fn DataLakeEditForm() -> impl IntoView {
    let params = use_params_map();
    let original_name = move || params.read().get("name").unwrap_or_default();

    let (active_tab, set_active_tab) = signal(DataLakeFormTab::Basic);
    let (name, set_name) = signal(String::new());
    let (description, set_description) = signal(String::new());
    let tags = RwSignal::new(Vec::<String>::new());
    let (schema_refs, set_schema_refs) = signal(Vec::<DataLakeSchemaRef>::new());
    let (saving, set_saving) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);
    let (loaded, set_loaded) = signal(false);

    // Storage configuration signals
    let (storage_mode, set_storage_mode) = signal(DataLakeStorageMode::Database);
    let (file_format, set_file_format) = signal(DataLakeFileFormat::Parquet);
    let (enable_sql_queries, set_enable_sql_queries) = signal(false);

    // Load available schemas for selection
    let available_schemas = LocalResource::new(|| async move {
        api::list_schemas().await.ok().unwrap_or_default()
    });

    // Load existing data lake
    Effect::new(move |_| {
        let lake_name = original_name();
        if !lake_name.is_empty() && !loaded.get() {
            wasm_bindgen_futures::spawn_local(async move {
                match api::get_data_lake(&lake_name).await {
                    Ok(lake) => {
                        set_name.set(lake.name);
                        set_description.set(lake.description.unwrap_or_default());
                        tags.set(lake.tags);
                        set_schema_refs.set(lake.schemas);
                        set_storage_mode.set(lake.storage_mode);
                        set_file_format.set(lake.file_format);
                        set_enable_sql_queries.set(lake.enable_sql_queries);
                        set_loaded.set(true);
                    }
                    Err(e) => {
                        set_error.set(Some(format!("Failed to load data lake: {}", e)));
                    }
                }
            });
        }
    });

    let add_schema_ref = move |_| {
        set_schema_refs.update(|refs| {
            refs.push(DataLakeSchemaRef {
                schema_name: String::new(),
                schema_version: None,
                alias: None,
            });
        });
    };

    let remove_schema_ref = move |index: usize| {
        set_schema_refs.update(|refs| {
            if index < refs.len() {
                refs.remove(index);
            }
        });
    };

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();

        if schema_refs.get().is_empty() || schema_refs.get().iter().all(|s| s.schema_name.is_empty()) {
            set_error.set(Some("Please add at least one schema reference".to_string()));
            return;
        }

        set_saving.set(true);
        set_error.set(None);

        let orig_name = original_name();
        let name_val = name.get();
        let desc_val = description.get();
        let schemas = schema_refs.get().into_iter().filter(|s| !s.schema_name.is_empty()).collect::<Vec<_>>();

        let storage_mode_val = storage_mode.get();
        let file_format_val = file_format.get();
        let enable_sql_val = enable_sql_queries.get();

        let data_lake = DataLake {
            name: name_val,
            description: if desc_val.is_empty() { None } else { Some(desc_val) },
            tags: tags.get(),
            schemas,
            metadata: None,
            storage_mode: storage_mode_val,
            file_format: file_format_val,
            enable_sql_queries: enable_sql_val,
        };

        wasm_bindgen_futures::spawn_local(async move {
            match api::update_data_lake(&orig_name, &data_lake).await {
                Ok(_) => {
                    let window = web_sys::window().unwrap();
                    window.location().set_href("/data-lakes").ok();
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_saving.set(false);
                }
            }
        });
    };

    view! {
        <div class="p-6 max-w-4xl mx-auto">
            <div class="flex items-center gap-4 mb-6">
                <A href="/data-lakes" attr:class="text-gray-500 hover:text-gray-700">
                    <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18"/>
                    </svg>
                </A>
                <h2 class="text-2xl font-bold">"Edit Data Lake"</h2>
            </div>

            <Show when=move || error.get().is_some()>
                <div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded mb-6">
                    {move || error.get().unwrap_or_default()}
                </div>
            </Show>

            // Loading spinner
            <div
                class="flex items-center justify-center py-12"
                style=move || if loaded.get() { "display: none" } else { "display: flex" }
            >
                <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-cyan-600"></div>
                <span class="ml-3 text-gray-500">"Loading data lake..."</span>
            </div>

            // Form - always rendered but hidden while loading
            <form
                on:submit=on_submit
                class="bg-white rounded-lg shadow p-6"
                style=move || if loaded.get() { "display: block" } else { "display: none" }
            >
                // Tab navigation
                <div class="border-b border-gray-200 mb-6">
                    <nav class="-mb-px flex space-x-8">
                        <button
                            type="button"
                            class=move || {
                                let base = "py-2 px-1 border-b-2 font-medium text-sm";
                                if active_tab.get() == DataLakeFormTab::Basic {
                                    format!("{} border-cyan-500 text-cyan-600", base)
                                } else {
                                    format!("{} border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300", base)
                                }
                            }
                            on:click=move |_| set_active_tab.set(DataLakeFormTab::Basic)
                        >
                            "Basic"
                        </button>
                        <button
                            type="button"
                            class=move || {
                                let base = "py-2 px-1 border-b-2 font-medium text-sm";
                                if active_tab.get() == DataLakeFormTab::Storage {
                                    format!("{} border-cyan-500 text-cyan-600", base)
                                } else {
                                    format!("{} border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300", base)
                                }
                            }
                            on:click=move |_| set_active_tab.set(DataLakeFormTab::Storage)
                        >
                            "Storage"
                        </button>
                    </nav>
                </div>

                // Basic Tab Content
                <div style=move || if active_tab.get() == DataLakeFormTab::Basic { "display: block" } else { "display: none" }>
                    <div class="space-y-6">
                        // Name
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"Name"</label>
                            <input
                                type="text"
                                required=true
                                class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-cyan-500 focus:border-cyan-500"
                                placeholder="e.g., users, orders, products"
                                prop:value=move || name.get()
                                on:input=move |ev| set_name.set(event_target_value(&ev))
                            />
                        </div>

                        // Description
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"Description"</label>
                            <textarea
                                class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-cyan-500 focus:border-cyan-500"
                                rows="2"
                                placeholder="Brief description of this data lake"
                                prop:value=move || description.get()
                                on:input=move |ev| set_description.set(event_target_value(&ev))
                            />
                        </div>

                        // Tags
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"Tags"</label>
                            <TagInput tags=tags />
                        </div>

                        // Schema References (same as create form)
                        <div>
                            <div class="flex justify-between items-center mb-2">
                                <label class="block text-sm font-medium text-gray-700">"Schema References"</label>
                                <button
                                    type="button"
                                    class="px-3 py-1 text-sm bg-cyan-50 text-cyan-700 rounded hover:bg-cyan-100"
                                    on:click=add_schema_ref
                                >
                                    "+ Add Schema"
                                </button>
                            </div>

                            <Suspense fallback=move || view! { <div class="text-gray-500 text-sm">"Loading schemas..."</div> }>
                                {move || {
                                    let schemas = available_schemas.get().map(|s| s).unwrap_or_default();
                                    view! {
                                        <div class="space-y-3">
                                            <For
                                                each=move || {
                                                    schema_refs.get().into_iter().enumerate().collect::<Vec<_>>()
                                                }
                                                key=|(i, _)| *i
                                                children=move |(index, schema_ref)| {
                                                    let schemas_for_select = schemas.clone();
                                                    view! {
                                                        <div class="flex items-start gap-3 p-3 bg-gray-50 rounded-lg">
                                                            <div class="flex-1 grid grid-cols-3 gap-3">
                                                                <div>
                                                                    <label class="block text-xs text-gray-500 mb-1">"Schema"</label>
                                                                    <select
                                                                        class="w-full px-2 py-1.5 text-sm border border-gray-300 rounded focus:ring-cyan-500 focus:border-cyan-500"
                                                                        prop:value=schema_ref.schema_name.clone()
                                                                        on:change=move |ev| {
                                                                            let value = event_target_value(&ev);
                                                                            set_schema_refs.update(|refs| {
                                                                                if let Some(r) = refs.get_mut(index) {
                                                                                    r.schema_name = value;
                                                                                }
                                                                            });
                                                                        }
                                                                    >
                                                                        <option value="">"Select schema..."</option>
                                                                        {schemas_for_select.iter().map(|s| {
                                                                            let schema_name = s.name.clone();
                                                                            let selected = schema_ref.schema_name == schema_name;
                                                                            view! {
                                                                                <option value=schema_name.clone() selected=selected>{schema_name.clone()}</option>
                                                                            }
                                                                        }).collect::<Vec<_>>()}
                                                                    </select>
                                                                </div>
                                                                <div>
                                                                    <label class="block text-xs text-gray-500 mb-1">"Version"</label>
                                                                    <input
                                                                        type="text"
                                                                        class="w-full px-2 py-1.5 text-sm border border-gray-300 rounded focus:ring-cyan-500 focus:border-cyan-500"
                                                                        placeholder="latest"
                                                                        prop:value=schema_ref.schema_version.clone().unwrap_or_default()
                                                                        on:input=move |ev| {
                                                                            let value = event_target_value(&ev);
                                                                            set_schema_refs.update(|refs| {
                                                                                if let Some(r) = refs.get_mut(index) {
                                                                                    r.schema_version = if value.is_empty() { None } else { Some(value) };
                                                                                }
                                                                            });
                                                                        }
                                                                    />
                                                                </div>
                                                                <div>
                                                                    <label class="block text-xs text-gray-500 mb-1">"Alias"</label>
                                                                    <input
                                                                        type="text"
                                                                        class="w-full px-2 py-1.5 text-sm border border-gray-300 rounded focus:ring-cyan-500 focus:border-cyan-500"
                                                                        placeholder="e.g., User"
                                                                        prop:value=schema_ref.alias.clone().unwrap_or_default()
                                                                        on:input=move |ev| {
                                                                            let value = event_target_value(&ev);
                                                                            set_schema_refs.update(|refs| {
                                                                                if let Some(r) = refs.get_mut(index) {
                                                                                    r.alias = if value.is_empty() { None } else { Some(value) };
                                                                                }
                                                                            });
                                                                        }
                                                                    />
                                                                </div>
                                                            </div>
                                                            <button
                                                                type="button"
                                                                class="mt-5 p-1 text-red-500 hover:bg-red-50 rounded"
                                                                on:click=move |_| remove_schema_ref(index)
                                                            >
                                                                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/>
                                                                </svg>
                                                            </button>
                                                        </div>
                                                    }
                                                }
                                            />
                                        </div>
                                    }
                                }}
                            </Suspense>
                        </div>
                    </div>
                </div>

                // Storage Tab Content
                <div style=move || if active_tab.get() == DataLakeFormTab::Storage { "display: block" } else { "display: none" }>
                    <div class="space-y-6">
                        // Storage Mode
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"Storage Mode"</label>
                            <select
                                class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-cyan-500 focus:border-cyan-500"
                                on:change=move |ev| {
                                    let value = event_target_value(&ev);
                                    set_storage_mode.set(match value.as_str() {
                                        "file" => DataLakeStorageMode::File,
                                        "both" => DataLakeStorageMode::Both,
                                        _ => DataLakeStorageMode::Database,
                                    });
                                }
                            >
                                <option value="database" selected=move || storage_mode.get() == DataLakeStorageMode::Database>"Database"</option>
                                <option value="file" selected=move || storage_mode.get() == DataLakeStorageMode::File>"File"</option>
                                <option value="both" selected=move || storage_mode.get() == DataLakeStorageMode::Both>"Both"</option>
                            </select>
                            <p class="text-xs text-gray-500 mt-1">
                                "Database: Store in SQL database. File: Store as Parquet/JSONL files. Both: Store in both."
                            </p>
                        </div>

                        // File Format
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"File Format"</label>
                            <select
                                class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-cyan-500 focus:border-cyan-500"
                                on:change=move |ev| {
                                    let value = event_target_value(&ev);
                                    set_file_format.set(match value.as_str() {
                                        "jsonl" => DataLakeFileFormat::Jsonl,
                                        _ => DataLakeFileFormat::Parquet,
                                    });
                                }
                            >
                                <option value="parquet" selected=move || file_format.get() == DataLakeFileFormat::Parquet>"Parquet"</option>
                                <option value="jsonl" selected=move || file_format.get() == DataLakeFileFormat::Jsonl>"JSONL"</option>
                            </select>
                            <p class="text-xs text-gray-500 mt-1">
                                "Parquet: Columnar format, best for analytics. JSONL: Human-readable, good for streaming."
                            </p>
                        </div>

                        // Enable SQL Queries
                        <div class="flex items-center gap-3">
                            <input
                                type="checkbox"
                                id="edit_enable_sql_queries"
                                class="h-4 w-4 text-cyan-600 focus:ring-cyan-500 border-gray-300 rounded"
                                prop:checked=move || enable_sql_queries.get()
                                on:change=move |ev| {
                                    let target = event_target::<web_sys::HtmlInputElement>(&ev);
                                    set_enable_sql_queries.set(target.checked());
                                }
                            />
                            <label for="edit_enable_sql_queries" class="text-sm font-medium text-gray-700">
                                "Enable SQL Queries"
                            </label>
                        </div>
                        <p class="text-xs text-gray-500 -mt-4 ml-7">
                            "Allow SQL queries via DataFusion on this data lake's files."
                        </p>
                    </div>
                </div>

                // Submit button
                    <div class="flex justify-end gap-3 mt-6 pt-6 border-t border-gray-200">
                        <A href="/data-lakes" attr:class="px-4 py-2 text-gray-600 hover:bg-gray-100 rounded-lg">
                            "Cancel"
                        </A>
                        <button
                            type="submit"
                            class="px-4 py-2 bg-cyan-600 text-white rounded-lg hover:bg-cyan-700 disabled:opacity-50"
                            disabled=move || saving.get()
                        >
                            {move || if saving.get() { "Saving..." } else { "Save Changes" }}
                        </button>
                    </div>
                </form>
        </div>
    }
}

/// Data Lake Records view - shows records with CRUD and generation
#[component]
pub fn DataLakeRecords() -> impl IntoView {
    let params = use_params_map();
    let data_lake_name = move || params.read().get("name").unwrap_or_default();

    // Tab state
    let (active_tab, set_active_tab) = signal(DataLakeRecordsTab::Records);

    // Records tab state
    let (refresh_trigger, set_refresh_trigger) = signal(0u32);
    let (selected_schema, set_selected_schema) = signal(Option::<String>::None);
    let (delete_target, set_delete_target) = signal(Option::<String>::None);
    let (deleting, set_deleting) = signal(false);
    let (show_generate_modal, set_show_generate_modal) = signal(false);
    let (show_add_modal, set_show_add_modal) = signal(false);
    let (edit_record, set_edit_record) = signal(Option::<DataRecord>::None);
    let (page, set_page) = signal(0usize);
    let page_size = 20;

    // Bulk selection state
    let selected_records: RwSignal<HashSet<String>> = RwSignal::new(HashSet::new());
    let (bulk_deleting, set_bulk_deleting) = signal(false);
    let (show_bulk_delete_confirm, set_show_bulk_delete_confirm) = signal(false);

    // Query tab state
    let sql_query = RwSignal::new(String::new());
    let selected_table = RwSignal::new(String::new()); // Format: datalake.schema
    let (query_running, set_query_running) = signal(false);
    let (query_result, set_query_result) = signal(Option::<SqlQueryResponse>::None);
    let (query_error, set_query_error) = signal(Option::<String>::None);

    // Fetch all data lakes for SQL editor autocomplete
    let all_data_lakes = LocalResource::new(move || async move {
        api::list_data_lakes().await.ok().unwrap_or_default()
    });

    // Build table references for autocomplete
    let table_refs = Signal::derive(move || {
        let lakes = all_data_lakes.get().unwrap_or_default();
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

    // Fetch all schemas for field autocomplete
    let all_schemas = LocalResource::new(move || async move {
        api::list_schemas().await.ok().unwrap_or_default()
    });

    // Build field references for autocomplete from schema definitions
    let field_refs = Signal::derive(move || {
        let schemas = all_schemas.get().unwrap_or_default();
        let mut refs: Vec<FieldRef> = Vec::new();
        for schema in schemas {
            let fields = extract_fields_from_schema(&schema.schema, &schema.name);
            refs.extend(fields);
        }
        refs
    });

    // Files tab state
    let (_files_refresh, set_files_refresh) = signal(0u32);
    let (files_loading, set_files_loading) = signal(false);
    let (files_list, set_files_list) = signal(Vec::<FileInfo>::new());
    let (files_error, set_files_error) = signal(Option::<String>::None);
    let (syncing, set_syncing) = signal(false);
    let (sync_schema, set_sync_schema) = signal(Option::<String>::None);
    let (show_sync_modal, set_show_sync_modal) = signal(false);

    // Load data lake info
    let data_lake = LocalResource::new(move || {
        let name = data_lake_name();
        async move {
            if name.is_empty() { None }
            else { api::get_data_lake(&name).await.ok() }
        }
    });

    // Load records with pagination and filtering
    let records = LocalResource::new(move || {
        let _ = refresh_trigger.get();
        let name = data_lake_name();
        let schema = selected_schema.get();
        let offset = page.get() * page_size;
        async move {
            if name.is_empty() { None }
            else { api::list_records(&name, schema.as_deref(), Some(page_size), Some(offset)).await.ok() }
        }
    });

    // Load record count
    let count = LocalResource::new(move || {
        let _ = refresh_trigger.get();
        let name = data_lake_name();
        let schema = selected_schema.get();
        async move {
            if name.is_empty() { None }
            else { api::count_records(&name, schema.as_deref()).await.ok() }
        }
    });

    let on_delete_confirm = move |_| {
        if let Some(id) = delete_target.get() {
            let name = data_lake_name();
            set_deleting.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                match api::delete_record(&name, &id).await {
                    Ok(_) => {
                        set_delete_target.set(None);
                        set_refresh_trigger.update(|n| *n += 1);
                    }
                    Err(e) => {
                        web_sys::window()
                            .and_then(|w| w.alert_with_message(&format!("Failed to delete: {}", e)).ok());
                    }
                }
                set_deleting.set(false);
            });
        }
    };

    // Bulk delete handler
    let on_bulk_delete_confirm = move |_| {
        let ids: Vec<String> = selected_records.get().into_iter().collect();
        if ids.is_empty() {
            return;
        }

        let name = data_lake_name();
        set_bulk_deleting.set(true);
        wasm_bindgen_futures::spawn_local(async move {
            match api::bulk_delete_records(&name, ids).await {
                Ok(response) => {
                    let msg = if response.failed == 0 {
                        format!("Successfully deleted {} record(s)", response.deleted)
                    } else {
                        format!(
                            "Deleted {} record(s), {} failed: {}",
                            response.deleted,
                            response.failed,
                            response.errors.join(", ")
                        )
                    };
                    web_sys::window().and_then(|w| w.alert_with_message(&msg).ok());
                    selected_records.set(HashSet::new());
                    set_show_bulk_delete_confirm.set(false);
                    set_refresh_trigger.update(|n| *n += 1);
                }
                Err(e) => {
                    web_sys::window()
                        .and_then(|w| w.alert_with_message(&format!("Bulk delete failed: {}", e)).ok());
                }
            }
            set_bulk_deleting.set(false);
        });
    };

    view! {
        <div class="p-6">
            // Header
            <div class="flex items-center gap-4 mb-6">
                <A href="/data-lakes" attr:class="text-gray-500 hover:text-gray-700">
                    <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18"/>
                    </svg>
                </A>
                <div>
                    <h2 class="text-2xl font-bold">{move || data_lake_name()} " Records"</h2>
                    <Suspense>
                        {move || count.get().flatten().map(|c| view! {
                            <p class="text-sm text-gray-500">{c.count} " total records"</p>
                        })}
                    </Suspense>
                </div>
            </div>

            // Tab navigation
            <div class="border-b border-gray-200 mb-6">
                <nav class="-mb-px flex space-x-8">
                    <button
                        type="button"
                        class=move || {
                            let base = "py-2 px-1 border-b-2 font-medium text-sm";
                            if active_tab.get() == DataLakeRecordsTab::Records {
                                format!("{} border-cyan-500 text-cyan-600", base)
                            } else {
                                format!("{} border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300", base)
                            }
                        }
                        on:click=move |_| set_active_tab.set(DataLakeRecordsTab::Records)
                    >
                        "Records"
                    </button>
                    <button
                        type="button"
                        class=move || {
                            let base = "py-2 px-1 border-b-2 font-medium text-sm";
                            if active_tab.get() == DataLakeRecordsTab::Query {
                                format!("{} border-cyan-500 text-cyan-600", base)
                            } else {
                                format!("{} border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300", base)
                            }
                        }
                        on:click=move |_| set_active_tab.set(DataLakeRecordsTab::Query)
                    >
                        "Query"
                    </button>
                    <button
                        type="button"
                        class=move || {
                            let base = "py-2 px-1 border-b-2 font-medium text-sm";
                            if active_tab.get() == DataLakeRecordsTab::Files {
                                format!("{} border-cyan-500 text-cyan-600", base)
                            } else {
                                format!("{} border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300", base)
                            }
                        }
                        on:click=move |_| set_active_tab.set(DataLakeRecordsTab::Files)
                    >
                        "Files"
                    </button>
                </nav>
            </div>

            // Records Tab Content
            <div style=move || if active_tab.get() == DataLakeRecordsTab::Records { "display: block" } else { "display: none" }>
            // Action bar
            <div class="flex justify-between items-center mb-4">
                // Schema filter
                <Suspense>
                    {move || {
                        data_lake.get().flatten().map(|lake| {
                            let schemas = lake.schemas.clone();
                            view! {
                                <div class="flex items-center gap-2">
                                    <label class="text-sm text-gray-600">"Filter by schema:"</label>
                                    <select
                                        class="px-3 py-1.5 text-sm border border-gray-300 rounded-lg focus:ring-cyan-500 focus:border-cyan-500"
                                        on:change=move |ev| {
                                            let value = event_target_value(&ev);
                                            set_selected_schema.set(if value.is_empty() { None } else { Some(value) });
                                            set_page.set(0);
                                        }
                                    >
                                        <option value="">"All schemas"</option>
                                        {schemas.iter().map(|s| {
                                            let schema_name = s.schema_name.clone();
                                            view! {
                                                <option value=schema_name.clone()>{schema_name.clone()}</option>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </select>
                                </div>
                            }
                        })
                    }}
                </Suspense>

                // Actions
                <div class="flex items-center gap-3">
                    <button
                        class="px-4 py-2 bg-purple-600 text-white rounded-lg hover:bg-purple-700 flex items-center gap-2"
                        on:click=move |_| set_show_generate_modal.set(true)
                    >
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19.428 15.428a2 2 0 00-1.022-.547l-2.387-.477a6 6 0 00-3.86.517l-.318.158a6 6 0 01-3.86.517L6.05 15.21a2 2 0 00-1.806.547M8 4h8l-1 1v5.172a2 2 0 00.586 1.414l5 5c1.26 1.26.367 3.414-1.415 3.414H4.828c-1.782 0-2.674-2.154-1.414-3.414l5-5A2 2 0 009 10.172V5L8 4z"/>
                        </svg>
                        "Generate Records"
                    </button>
                    <button
                        class="px-4 py-2 bg-cyan-600 text-white rounded-lg hover:bg-cyan-700 flex items-center gap-2"
                        on:click=move |_| set_show_add_modal.set(true)
                    >
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/>
                        </svg>
                        "Add Record"
                    </button>
                </div>
            </div>

            // Bulk action bar - shown when records are selected
            <Show when=move || !selected_records.get().is_empty()>
                <div class="bg-cyan-50 border border-cyan-200 rounded-lg p-3 mb-4 flex items-center justify-between">
                    <div class="flex items-center gap-3">
                        <span class="text-sm font-medium text-cyan-800">
                            {move || format!("{} record(s) selected", selected_records.get().len())}
                        </span>
                        <button
                            class="text-sm text-cyan-600 hover:text-cyan-800"
                            on:click=move |_| selected_records.set(HashSet::new())
                        >
                            "Clear selection"
                        </button>
                    </div>
                    <div class="flex items-center gap-2">
                        <button
                            class="px-3 py-1.5 bg-red-600 text-white text-sm rounded-lg hover:bg-red-700 flex items-center gap-1 disabled:opacity-50"
                            disabled=move || bulk_deleting.get()
                            on:click=move |_| set_show_bulk_delete_confirm.set(true)
                        >
                            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/>
                            </svg>
                            {move || if bulk_deleting.get() { "Deleting..." } else { "Delete Selected" }}
                        </button>
                    </div>
                </div>
            </Show>

            // Records table
            <Suspense fallback=move || view! { <div class="text-gray-500">"Loading records..."</div> }>
                {move || {
                    records.get().map(|maybe_records| {
                        match maybe_records {
                            Some(response) if !response.records.is_empty() => {
                                // Compute pagination values before the view macro
                                let current_page_val = page.get();
                                let pages = count.get()
                                    .flatten()
                                    .map(|c| (c.count + page_size - 1) / page_size)
                                    .unwrap_or(1);
                                // Must be closures for Leptos disabled attribute
                                let is_first_page = move || current_page_val == 0;
                                let is_last_page = move || current_page_val + 1 >= pages;

                                // Collect all record IDs for "select all" functionality
                                let all_record_ids: Vec<String> = response.records.iter().map(|r| r.id.clone()).collect();
                                let all_ids_for_check = all_record_ids.clone();

                                view! {
                                    <div class="bg-white rounded-lg shadow overflow-hidden">
                                        <table class="min-w-full divide-y divide-gray-200">
                                            <thead class="bg-gray-50">
                                                <tr>
                                                    <th class="px-4 py-3 text-left">
                                                        <input
                                                            type="checkbox"
                                                            class="w-4 h-4 text-cyan-600 border-gray-300 rounded focus:ring-cyan-500 cursor-pointer"
                                                            prop:checked=move || {
                                                                let selected = selected_records.get();
                                                                !all_ids_for_check.is_empty() && all_ids_for_check.iter().all(|id| selected.contains(id))
                                                            }
                                                            on:change={
                                                                let all_ids = all_record_ids.clone();
                                                                move |ev| {
                                                                    let checked = event_target_checked(&ev);
                                                                    selected_records.update(|set| {
                                                                        if checked {
                                                                            for id in &all_ids {
                                                                                set.insert(id.clone());
                                                                            }
                                                                        } else {
                                                                            for id in &all_ids {
                                                                                set.remove(id);
                                                                            }
                                                                        }
                                                                    });
                                                                }
                                                            }
                                                        />
                                                    </th>
                                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"ID"</th>
                                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Schema"</th>
                                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Data Preview"</th>
                                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Created"</th>
                                                    <th class="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">"Actions"</th>
                                                </tr>
                                            </thead>
                                            <tbody class="bg-white divide-y divide-gray-200">
                                                {response.records.into_iter().map(|record| {
                                                    let id = record.id.clone();
                                                    let id_for_checkbox = record.id.clone();
                                                    let id_for_check = record.id.clone();
                                                    let id_for_delete = record.id.clone();
                                                    let record_for_edit = record.clone();
                                                    let data_preview = serde_json::to_string(&record.data)
                                                        .unwrap_or_default()
                                                        .chars()
                                                        .take(80)
                                                        .collect::<String>();
                                                    view! {
                                                        <tr class="hover:bg-gray-50">
                                                            <td class="px-4 py-4 whitespace-nowrap">
                                                                <input
                                                                    type="checkbox"
                                                                    class="w-4 h-4 text-cyan-600 border-gray-300 rounded focus:ring-cyan-500 cursor-pointer"
                                                                    prop:checked=move || selected_records.get().contains(&id_for_check)
                                                                    on:change=move |ev| {
                                                                        let checked = event_target_checked(&ev);
                                                                        let id = id_for_checkbox.clone();
                                                                        selected_records.update(|set| {
                                                                            if checked {
                                                                                set.insert(id);
                                                                            } else {
                                                                                set.remove(&id);
                                                                            }
                                                                        });
                                                                    }
                                                                />
                                                            </td>
                                                            <td class="px-6 py-4 whitespace-nowrap">
                                                                <code class="text-xs bg-gray-100 px-2 py-1 rounded font-mono">{id.clone()}</code>
                                                            </td>
                                                            <td class="px-6 py-4 whitespace-nowrap">
                                                                <span class="px-2 inline-flex text-xs leading-5 font-semibold rounded-full bg-cyan-100 text-cyan-800">
                                                                    {record.schema_name.clone()}
                                                                </span>
                                                            </td>
                                                            <td class="px-6 py-4">
                                                                <code class="text-xs text-gray-600 font-mono truncate block max-w-md">
                                                                    {data_preview}{if serde_json::to_string(&record.data).unwrap_or_default().len() > 80 { "..." } else { "" }}
                                                                </code>
                                                            </td>
                                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                                                                {record.created_at.clone()}
                                                            </td>
                                                            <td class="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
                                                                <button
                                                                    class="text-cyan-600 hover:text-cyan-900 mr-4"
                                                                    on:click=move |_| set_edit_record.set(Some(record_for_edit.clone()))
                                                                >
                                                                    "Edit"
                                                                </button>
                                                                <button
                                                                    class="text-red-600 hover:text-red-900"
                                                                    on:click=move |_| set_delete_target.set(Some(id_for_delete.clone()))
                                                                >
                                                                    "Delete"
                                                                </button>
                                                            </td>
                                                        </tr>
                                                    }
                                                }).collect::<Vec<_>>()}
                                            </tbody>
                                        </table>

                                        // Pagination
                                        <div class="bg-gray-50 px-6 py-3 flex items-center justify-between border-t border-gray-200">
                                            <div class="text-sm text-gray-700">
                                                "Page " {current_page_val + 1} " of " {pages}
                                            </div>
                                            <div class="flex gap-2">
                                                <button
                                                    class="px-3 py-1 text-sm border border-gray-300 rounded hover:bg-gray-100 disabled:opacity-50 disabled:cursor-not-allowed"
                                                    disabled=is_first_page
                                                    on:click=move |_| set_page.update(|p| *p = p.saturating_sub(1))
                                                >
                                                    "Previous"
                                                </button>
                                                <button
                                                    class="px-3 py-1 text-sm border border-gray-300 rounded hover:bg-gray-100 disabled:opacity-50 disabled:cursor-not-allowed"
                                                    disabled=is_last_page
                                                    on:click=move |_| set_page.update(|p| *p += 1)
                                                >
                                                    "Next"
                                                </button>
                                            </div>
                                        </div>
                                    </div>
                                }.into_any()
                            }
                            _ => view! {
                                <div class="bg-white rounded-lg shadow p-8 text-center">
                                    <div class="text-gray-400 mb-4">
                                        <svg class="w-16 h-16 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
                                        </svg>
                                    </div>
                                    <h3 class="text-lg font-medium text-gray-900 mb-2">"No records yet"</h3>
                                    <p class="text-gray-500 mb-4">"Add records manually or generate them using mock strategies."</p>
                                    <div class="flex justify-center gap-3">
                                        <button
                                            class="px-4 py-2 bg-cyan-600 text-white rounded-lg hover:bg-cyan-700"
                                            on:click=move |_| set_show_add_modal.set(true)
                                        >
                                            "Add Record"
                                        </button>
                                        <button
                                            class="px-4 py-2 bg-purple-600 text-white rounded-lg hover:bg-purple-700"
                                            on:click=move |_| set_show_generate_modal.set(true)
                                        >
                                            "Generate Records"
                                        </button>
                                    </div>
                                </div>
                            }.into_any()
                        }
                    })
                }}
            </Suspense>
            </div>

            // Query Tab Content
            <div style=move || if active_tab.get() == DataLakeRecordsTab::Query { "display: block" } else { "display: none" }>
                <div class="bg-white rounded-lg shadow p-6">
                    <div class="flex justify-between items-start mb-4">
                        <div>
                            <h3 class="text-lg font-semibold">"SQL Query"</h3>
                            <p class="text-sm text-gray-500">
                                "Execute SQL queries with syntax highlighting and autocompletion."
                            </p>
                        </div>
                        <a
                            href="/docs/datafusion"
                            target="_blank"
                            class="text-sm text-cyan-600 hover:text-cyan-800 flex items-center gap-1"
                        >
                            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                            </svg>
                            "Query Help"
                        </a>
                    </div>

                    // Table selector with all data lakes
                    <div class="mb-4">
                        <Suspense fallback=move || view! { <div class="text-gray-400">"Loading tables..."</div> }>
                            {move || {
                                let current_lake = data_lake_name();
                                let all_lakes = all_data_lakes.get().unwrap_or_default();
                                let lakes_signal = Signal::derive(move || all_lakes.clone());
                                view! {
                                    <TableSelector
                                        data_lakes=lakes_signal
                                        selected=selected_table
                                        current_data_lake=current_lake
                                    />
                                }
                            }}
                        </Suspense>
                    </div>

                    // SQL Editor with syntax highlighting
                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-2">"SQL Query"</label>
                        <SqlEditor
                            value=sql_query
                            tables=table_refs
                            fields=field_refs
                            placeholder="SELECT * FROM $table WHERE json_get_str(data, 'status') = 'active' LIMIT 100"
                            rows=8
                        />
                        <div class="mt-2 flex flex-wrap gap-2 text-xs text-gray-500">
                            <span class="bg-gray-100 px-2 py-1 rounded">"$table → selected default table"</span>
                            <span class="bg-gray-100 px-2 py-1 rounded">"Tab/Enter → accept suggestion"</span>
                            <span class="bg-gray-100 px-2 py-1 rounded">"datalake.schema → explicit table"</span>
                        </div>
                    </div>

                    // Run button
                    <div class="flex justify-between items-center mb-6">
                        <div class="text-xs text-gray-500">
                            {move || {
                                let table = selected_table.get();
                                if table.is_empty() {
                                    "Select a default table or use explicit table names in query".to_string()
                                } else {
                                    format!("$table will be replaced with: {}", table)
                                }
                            }}
                        </div>
                        <button
                            class="px-4 py-2 bg-cyan-600 text-white rounded-lg hover:bg-cyan-700 disabled:opacity-50 flex items-center gap-2"
                            disabled=move || query_running.get() || sql_query.get().is_empty()
                            on:click=move |_| {
                                let table = selected_table.get();
                                let sql = sql_query.get();
                                let lake_name = data_lake_name();

                                // Parse the selected table to get data_lake and schema
                                let (target_lake, schema_name) = if table.contains('.') {
                                    let parts: Vec<&str> = table.splitn(2, '.').collect();
                                    (parts[0].to_string(), parts.get(1).map(|s| s.to_string()).unwrap_or_default())
                                } else if !table.is_empty() {
                                    (lake_name.clone(), table)
                                } else {
                                    // No table selected - try to extract from query or use current lake
                                    (lake_name.clone(), String::new())
                                };

                                set_query_running.set(true);
                                set_query_error.set(None);
                                set_query_result.set(None);

                                let request = SqlQueryRequest {
                                    sql: sql.clone(),
                                    schema_name: schema_name.clone(),
                                };

                                wasm_bindgen_futures::spawn_local(async move {
                                    match api::execute_query(&target_lake, &request).await {
                                        Ok(result) => {
                                            set_query_result.set(Some(result));
                                        }
                                        Err(e) => {
                                            set_query_error.set(Some(e));
                                        }
                                    }
                                    set_query_running.set(false);
                                });
                            }
                        >
                            {move || if query_running.get() {
                                view! {
                                    <svg class="animate-spin h-4 w-4" fill="none" viewBox="0 0 24 24">
                                        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                                        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                                    </svg>
                                    "Running..."
                                }.into_any()
                            } else {
                                view! {
                                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z"/>
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                                    </svg>
                                    "Run Query"
                                }.into_any()
                            }}
                        </button>
                    </div>

                    // Error display
                    <Show when=move || query_error.get().is_some()>
                        <div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded mb-4">
                            <div class="flex items-start gap-2">
                                <svg class="w-5 h-5 flex-shrink-0 mt-0.5" fill="currentColor" viewBox="0 0 20 20">
                                    <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z" clip-rule="evenodd"/>
                                </svg>
                                <div class="font-mono text-sm whitespace-pre-wrap">{move || query_error.get().unwrap_or_default()}</div>
                            </div>
                        </div>
                    </Show>

                    // Results display
                    <Show when=move || query_result.get().is_some()>
                        {move || query_result.get().map(|result| {
                            view! {
                                <div>
                                    <div class="flex justify-between items-center mb-3 pb-2 border-b border-gray-200">
                                        <span class="text-sm font-medium text-gray-700">
                                            {result.total_rows} " rows returned"
                                        </span>
                                        <button
                                            class="text-xs text-cyan-600 hover:text-cyan-800 flex items-center gap-1"
                                            on:click=move |_| {
                                                // Copy results as CSV
                                                let result = query_result.get();
                                                if let Some(r) = result {
                                                    let mut csv = r.columns.join(",") + "\n";
                                                    for row in &r.rows {
                                                        let values: Vec<String> = r.columns.iter().map(|col| {
                                                            row.get(col).map(|v| {
                                                                if v.is_string() {
                                                                    format!("\"{}\"", v.as_str().unwrap_or_default().replace('"', "\"\""))
                                                                } else {
                                                                    v.to_string()
                                                                }
                                                            }).unwrap_or_default()
                                                        }).collect();
                                                        csv.push_str(&values.join(","));
                                                        csv.push('\n');
                                                    }
                                                    // Copy to clipboard using JS
                                                    let _ = js_sys::eval(&format!(
                                                        "navigator.clipboard.writeText(`{}`)",
                                                        csv.replace('`', "\\`").replace('$', "\\$")
                                                    ));
                                                }
                                            }
                                        >
                                            <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-1M8 5a2 2 0 002 2h2a2 2 0 002-2M8 5a2 2 0 012-2h2a2 2 0 012 2m0 0h2a2 2 0 012 2v3m2 4H10m0 0l3-3m-3 3l3 3"/>
                                            </svg>
                                            "Copy as CSV"
                                        </button>
                                    </div>
                                    <div class="overflow-x-auto border border-gray-200 rounded-lg max-h-96">
                                        <table class="min-w-full divide-y divide-gray-200">
                                            <thead class="bg-gray-50 sticky top-0">
                                                <tr>
                                                    {result.columns.iter().map(|col| {
                                                        view! {
                                                            <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider bg-gray-50">{col.clone()}</th>
                                                        }
                                                    }).collect::<Vec<_>>()}
                                                </tr>
                                            </thead>
                                            <tbody class="bg-white divide-y divide-gray-200">
                                                {result.rows.iter().map(|row| {
                                                    view! {
                                                        <tr class="hover:bg-gray-50">
                                                            {result.columns.iter().map(|col| {
                                                                let value = row.get(col).map(|v| {
                                                                    if v.is_string() {
                                                                        v.as_str().unwrap_or_default().to_string()
                                                                    } else if v.is_null() {
                                                                        "null".to_string()
                                                                    } else {
                                                                        v.to_string()
                                                                    }
                                                                }).unwrap_or_else(|| "".to_string());
                                                                let is_null = value == "null";
                                                                let is_json = value.starts_with('{') || value.starts_with('[');
                                                                let value_for_title = value.clone();
                                                                view! {
                                                                    <td class=move || format!(
                                                                        "px-4 py-2 text-sm {} max-w-xs truncate",
                                                                        if is_null { "text-gray-400 italic" }
                                                                        else if is_json { "text-purple-600 font-mono text-xs" }
                                                                        else { "text-gray-600" }
                                                                    )
                                                                    title=value_for_title
                                                                    >{value}</td>
                                                                }
                                                            }).collect::<Vec<_>>()}
                                                        </tr>
                                                    }
                                                }).collect::<Vec<_>>()}
                                            </tbody>
                                        </table>
                                    </div>
                                </div>
                            }
                        })}
                    </Show>
                </div>
            </div>

            // Files Tab Content
            <div style=move || if active_tab.get() == DataLakeRecordsTab::Files { "display: block" } else { "display: none" }>
                <div class="bg-white rounded-lg shadow p-6">
                    <div class="flex justify-between items-center mb-4">
                        <div>
                            <h3 class="text-lg font-semibold">"Data Files"</h3>
                            <p class="text-sm text-gray-500">"Manage Parquet and JSONL files for this data lake."</p>
                        </div>
                        <div class="flex gap-3">
                            <button
                                class="px-4 py-2 border border-gray-300 rounded-lg hover:bg-gray-50 flex items-center gap-2"
                                on:click=move |_| {
                                    let lake_name = data_lake_name();
                                    set_files_loading.set(true);
                                    set_files_error.set(None);
                                    wasm_bindgen_futures::spawn_local(async move {
                                        match api::list_files(&lake_name).await {
                                            Ok(files) => {
                                                set_files_list.set(files);
                                            }
                                            Err(e) => {
                                                set_files_error.set(Some(e));
                                            }
                                        }
                                        set_files_loading.set(false);
                                    });
                                }
                            >
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/>
                                </svg>
                                "Refresh"
                            </button>
                            <button
                                class="px-4 py-2 bg-cyan-600 text-white rounded-lg hover:bg-cyan-700 flex items-center gap-2"
                                disabled=move || syncing.get()
                                on:click=move |_| set_show_sync_modal.set(true)
                            >
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"/>
                                </svg>
                                {move || if syncing.get() { "Syncing..." } else { "Sync to Files" }}
                            </button>
                        </div>
                    </div>

                    // Error display
                    <Show when=move || files_error.get().is_some()>
                        <div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded mb-4">
                            {move || files_error.get().unwrap_or_default()}
                        </div>
                    </Show>

                    // Loading
                    <Show when=move || files_loading.get()>
                        <div class="flex items-center justify-center py-8">
                            <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-cyan-600"></div>
                            <span class="ml-3 text-gray-500">"Loading files..."</span>
                        </div>
                    </Show>

                    // Files table
                    <Show when=move || !files_loading.get() && !files_list.get().is_empty()>
                        <div class="overflow-x-auto border border-gray-200 rounded-lg">
                            <table class="min-w-full divide-y divide-gray-200">
                                <thead class="bg-gray-50">
                                    <tr>
                                        <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Path"</th>
                                        <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Format"</th>
                                        <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Size"</th>
                                        <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Last Modified"</th>
                                    </tr>
                                </thead>
                                <tbody class="bg-white divide-y divide-gray-200">
                                    {move || files_list.get().iter().map(|file| {
                                        let size_str = if file.size_bytes > 1024 * 1024 {
                                            format!("{:.1} MB", file.size_bytes as f64 / (1024.0 * 1024.0))
                                        } else if file.size_bytes > 1024 {
                                            format!("{:.1} KB", file.size_bytes as f64 / 1024.0)
                                        } else {
                                            format!("{} B", file.size_bytes)
                                        };
                                        view! {
                                            <tr class="hover:bg-gray-50">
                                                <td class="px-4 py-2 text-sm font-mono text-gray-600">{file.path.clone()}</td>
                                                <td class="px-4 py-2">
                                                    <span class=format!(
                                                        "px-2 py-1 text-xs rounded {}",
                                                        if file.format == "parquet" { "bg-purple-100 text-purple-700" } else { "bg-blue-100 text-blue-700" }
                                                    )>
                                                        {file.format.clone().to_uppercase()}
                                                    </span>
                                                </td>
                                                <td class="px-4 py-2 text-sm text-gray-600">{size_str}</td>
                                                <td class="px-4 py-2 text-sm text-gray-500">{file.last_modified.clone()}</td>
                                            </tr>
                                        }
                                    }).collect::<Vec<_>>()}
                                </tbody>
                            </table>
                        </div>
                    </Show>

                    // Empty state
                    <Show when=move || !files_loading.get() && files_list.get().is_empty()>
                        <div class="text-center py-8">
                            <svg class="w-16 h-16 mx-auto text-gray-300 mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 19a2 2 0 01-2-2V7a2 2 0 012-2h4l2 2h4a2 2 0 012 2v1M5 19h14a2 2 0 002-2v-5a2 2 0 00-2-2H9a2 2 0 00-2 2v5a2 2 0 01-2 2z"/>
                            </svg>
                            <h4 class="text-lg font-medium text-gray-700 mb-2">"No files yet"</h4>
                            <p class="text-gray-500 mb-4">"Click \"Sync to Files\" to export records as Parquet or JSONL files."</p>
                            <p class="text-sm text-gray-400">"Click \"Refresh\" to load the file list."</p>
                        </div>
                    </Show>
                </div>
            </div>

            // Sync to files modal
            <Show when=move || show_sync_modal.get()>
                <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
                    <div class="bg-white rounded-lg p-6 max-w-md w-full mx-4">
                        <div class="flex justify-between items-center mb-4">
                            <h3 class="text-lg font-semibold">"Sync to Files"</h3>
                            <button class="text-gray-400 hover:text-gray-600" on:click=move |_| set_show_sync_modal.set(false)>
                                <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                                </svg>
                            </button>
                        </div>

                        <p class="text-sm text-gray-600 mb-4">"Export data lake records to file storage."</p>

                        // Schema filter
                        <div class="mb-4">
                            <label class="block text-sm font-medium text-gray-700 mb-1">"Schema (optional)"</label>
                            <Suspense>
                                {move || {
                                    data_lake.get().flatten().map(|lake| {
                                        let schemas = lake.schemas.clone();
                                        view! {
                                            <select
                                                class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-cyan-500 focus:border-cyan-500"
                                                on:change=move |ev| {
                                                    let value = event_target_value(&ev);
                                                    set_sync_schema.set(if value.is_empty() { None } else { Some(value) });
                                                }
                                            >
                                                <option value="">"All schemas"</option>
                                                {schemas.iter().map(|s| {
                                                    let schema_name = s.schema_name.clone();
                                                    view! {
                                                        <option value=schema_name.clone()>{schema_name.clone()}</option>
                                                    }
                                                }).collect::<Vec<_>>()}
                                            </select>
                                        }
                                    })
                                }}
                            </Suspense>
                        </div>

                        <div class="flex justify-end gap-3">
                            <button
                                class="px-4 py-2 text-gray-600 hover:bg-gray-100 rounded"
                                on:click=move |_| set_show_sync_modal.set(false)
                            >
                                "Cancel"
                            </button>
                            <button
                                class="px-4 py-2 bg-cyan-600 text-white rounded-lg hover:bg-cyan-700 disabled:opacity-50"
                                disabled=move || syncing.get()
                                on:click=move |_| {
                                    let lake_name = data_lake_name();
                                    let schema = sync_schema.get();
                                    set_syncing.set(true);

                                    let request = SyncRequest {
                                        schema_name: schema,
                                        format: None,
                                    };

                                    wasm_bindgen_futures::spawn_local(async move {
                                        match api::sync_to_files(&lake_name, &request).await {
                                            Ok(result) => {
                                                web_sys::window().and_then(|w|
                                                    w.alert_with_message(&format!(
                                                        "Synced {} records to {} files",
                                                        result.records_synced,
                                                        result.files_written
                                                    )).ok()
                                                );
                                                set_show_sync_modal.set(false);
                                                // Refresh file list
                                                set_files_refresh.update(|n| *n += 1);
                                            }
                                            Err(e) => {
                                                web_sys::window().and_then(|w|
                                                    w.alert_with_message(&format!("Sync failed: {}", e)).ok()
                                                );
                                            }
                                        }
                                        set_syncing.set(false);
                                    });
                                }
                            >
                                {move || if syncing.get() { "Syncing..." } else { "Sync" }}
                            </button>
                        </div>
                    </div>
                </div>
            </Show>

            // Delete confirmation modal
            <Show when=move || delete_target.get().is_some()>
                <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
                    <div class="bg-white rounded-lg p-6 max-w-sm w-full mx-4">
                        <h3 class="text-lg font-semibold mb-4">"Delete Record?"</h3>
                        <p class="text-gray-600 mb-6">
                            "Are you sure you want to delete this record?"
                        </p>
                        <div class="flex justify-end gap-3">
                            <button
                                class="px-4 py-2 text-gray-600 hover:bg-gray-100 rounded"
                                on:click=move |_| set_delete_target.set(None)
                            >
                                "Cancel"
                            </button>
                            <button
                                class="px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700 disabled:opacity-50"
                                disabled=move || deleting.get()
                                on:click=on_delete_confirm
                            >
                                {move || if deleting.get() { "Deleting..." } else { "Delete" }}
                            </button>
                        </div>
                    </div>
                </div>
            </Show>

            // Bulk delete confirmation modal
            <Show when=move || show_bulk_delete_confirm.get()>
                <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
                    <div class="bg-white rounded-lg p-6 max-w-sm w-full mx-4">
                        <h3 class="text-lg font-semibold mb-4">"Delete Selected Records?"</h3>
                        <p class="text-gray-600 mb-2">
                            "Are you sure you want to delete "
                            <strong>{move || selected_records.get().len()}</strong>
                            " selected record(s)?"
                        </p>
                        <p class="text-red-600 text-sm mb-6">
                            "This action cannot be undone."
                        </p>
                        <div class="flex justify-end gap-3">
                            <button
                                class="px-4 py-2 text-gray-600 hover:bg-gray-100 rounded"
                                on:click=move |_| set_show_bulk_delete_confirm.set(false)
                            >
                                "Cancel"
                            </button>
                            <button
                                class="px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700 disabled:opacity-50"
                                disabled=move || bulk_deleting.get()
                                on:click=on_bulk_delete_confirm
                            >
                                {move || if bulk_deleting.get() { "Deleting..." } else { "Delete All" }}
                            </button>
                        </div>
                    </div>
                </div>
            </Show>

            // Generate records modal
            <Show when=move || show_generate_modal.get()>
                <GenerateRecordsModal
                    data_lake_name=data_lake_name()
                    data_lake=data_lake.get().flatten()
                    on_close=move || set_show_generate_modal.set(false)
                    on_success=move || {
                        set_show_generate_modal.set(false);
                        set_refresh_trigger.update(|n| *n += 1);
                    }
                />
            </Show>

            // Add record modal
            <Show when=move || show_add_modal.get()>
                <AddRecordModal
                    data_lake_name=data_lake_name()
                    data_lake=data_lake.get().flatten()
                    on_close=move || set_show_add_modal.set(false)
                    on_success=move || {
                        set_show_add_modal.set(false);
                        set_refresh_trigger.update(|n| *n += 1);
                    }
                />
            </Show>

            // Edit record modal
            <Show when=move || edit_record.get().is_some()>
                <EditRecordModal
                    data_lake_name=data_lake_name()
                    record=edit_record.get().unwrap()
                    on_close=move || set_edit_record.set(None)
                    on_success=move || {
                        set_edit_record.set(None);
                        set_refresh_trigger.update(|n| *n += 1);
                    }
                />
            </Show>
        </div>
    }
}

/// Modal for generating records using mock strategies
#[component]
fn GenerateRecordsModal(
    data_lake_name: String,
    data_lake: Option<DataLake>,
    on_close: impl Fn() + 'static + Clone,
    on_success: impl Fn() + 'static + Clone,
) -> impl IntoView {
    let (selected_schema, set_selected_schema) = signal(String::new());
    let (count, set_count) = signal(10u32);
    let (strategy, set_strategy) = signal("random".to_string());
    let (strategy_config, set_strategy_config) = signal(String::new());
    let (generating, set_generating) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);
    let (loading_schema, set_loading_schema) = signal(false);

    // RwSignal for schema JSON (used by SchemaFormGenerator)
    let schema_json: RwSignal<serde_json::Value> = RwSignal::new(serde_json::Value::Null);
    // RwSignal for form output (as JSON string, required by SchemaFormGenerator)
    let form_output: RwSignal<String> = RwSignal::new(String::new());

    let on_close_clone = on_close.clone();
    let on_success_clone = on_success.clone();

    // Fetch schema when selection changes
    Effect::new(move |_| {
        let schema_name = selected_schema.get();
        if schema_name.is_empty() {
            schema_json.set(serde_json::Value::Null);
            return;
        }

        set_loading_schema.set(true);
        wasm_bindgen_futures::spawn_local(async move {
            match api::get_schema(&schema_name).await {
                Ok(schema) => {
                    schema_json.set(schema.schema);
                    // Reset form output when schema changes
                    form_output.set(String::new());
                }
                Err(_) => {
                    schema_json.set(serde_json::Value::Null);
                }
            }
            set_loading_schema.set(false);
        });
    });

    // Sync form output to strategy_config
    Effect::new(move |_| {
        let output = form_output.get();
        let strat = strategy.get();
        if (strat == "random" || strat == "static") && !output.is_empty() {
            set_strategy_config.set(output);
        }
    });

    let on_generate = move |_| {
        let schema = selected_schema.get();
        if schema.is_empty() {
            set_error.set(Some("Please select a schema".to_string()));
            return;
        }

        set_generating.set(true);
        set_error.set(None);

        let lake_name = data_lake_name.clone();
        let strategy_val = strategy.get();
        let count_val = count.get() as usize;
        let on_success = on_success_clone.clone();

        // Use form output for static/random strategies, otherwise parse from textarea
        let config: Option<serde_json::Value> = if strategy_val == "static" || strategy_val == "random" {
            let output = form_output.get();
            if !output.is_empty() && output != "{}" {
                match serde_json::from_str(&output) {
                    Ok(v) => Some(v),
                    Err(_) => None,
                }
            } else {
                None
            }
        } else {
            let config_str = strategy_config.get();
            if config_str.is_empty() {
                None
            } else {
                match serde_json::from_str(&config_str) {
                    Ok(v) => Some(v),
                    Err(e) => {
                        set_error.set(Some(format!("Invalid strategy config JSON: {}", e)));
                        set_generating.set(false);
                        return;
                    }
                }
            }
        };

        let request = GenerateRecordsRequest {
            schema_name: schema,
            count: count_val,
            strategy: strategy_val,
            strategy_config: config,
        };

        wasm_bindgen_futures::spawn_local(async move {
            match api::generate_records(&lake_name, &request).await {
                Ok(response) => {
                    web_sys::window().and_then(|w|
                        w.alert_with_message(&format!("Generated {} records", response.generated)).ok()
                    );
                    on_success();
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_generating.set(false);
                }
            }
        });
    };

    // Determine if we should show the schema form
    let show_schema_form = move || {
        let strat = strategy.get();
        let has_schema = schema_json.get() != serde_json::Value::Null;
        (strat == "random" || strat == "static") && has_schema && !loading_schema.get()
    };

    view! {
        <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
            <div class="bg-white rounded-lg p-6 max-w-2xl w-full mx-4 max-h-[90vh] overflow-y-auto">
                <div class="flex justify-between items-center mb-4">
                    <h3 class="text-lg font-semibold">"Generate Records"</h3>
                    <button class="text-gray-400 hover:text-gray-600" on:click=move |_| on_close_clone()>
                        <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                        </svg>
                    </button>
                </div>

                <Show when=move || error.get().is_some()>
                    <div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded mb-4 text-sm">
                        {move || error.get().unwrap_or_default()}
                    </div>
                </Show>

                <div class="space-y-4">
                    // Schema selection
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">"Schema"</label>
                        <select
                            class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-purple-500 focus:border-purple-500"
                            prop:value=move || selected_schema.get()
                            on:change=move |ev| set_selected_schema.set(event_target_value(&ev))
                        >
                            <option value="">"Select schema..."</option>
                            {data_lake.as_ref().map(|lake| {
                                lake.schemas.iter().map(|s| {
                                    let schema_name = s.schema_name.clone();
                                    view! {
                                        <option value=schema_name.clone()>{schema_name.clone()}</option>
                                    }
                                }).collect::<Vec<_>>()
                            }).unwrap_or_default()}
                        </select>
                    </div>

                    // Count
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">"Number of Records"</label>
                        <input
                            type="number"
                            min="1"
                            max="1000"
                            class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-purple-500 focus:border-purple-500"
                            prop:value=move || count.get().to_string()
                            on:input=move |ev| {
                                if let Ok(n) = event_target_value(&ev).parse() {
                                    set_count.set(n);
                                }
                            }
                        />
                    </div>

                    // Strategy
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">"Mock Strategy"</label>
                        <select
                            class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-purple-500 focus:border-purple-500"
                            prop:value=move || strategy.get()
                            on:change=move |ev| {
                                set_strategy.set(event_target_value(&ev));
                                // Reset form when strategy changes
                                form_output.set("{}".to_string());
                                set_strategy_config.set(String::new());
                            }
                        >
                            <option value="random">"Random (faker-based)"</option>
                            <option value="static">"Static (fixed value)"</option>
                            <option value="pattern">"Pattern (regex)"</option>
                            <option value="template">"Template (Tera)"</option>
                            <option value="llm">"LLM (AI-generated)"</option>
                        </select>
                    </div>

                    // Loading indicator
                    <Show when=move || loading_schema.get()>
                        <div class="text-sm text-gray-500 flex items-center gap-2">
                            <svg class="animate-spin h-4 w-4" fill="none" viewBox="0 0 24 24">
                                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"/>
                                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"/>
                            </svg>
                            "Loading schema..."
                        </div>
                    </Show>

                    // Schema-driven form for static strategy
                    <Show when=move || strategy.get() == "static" && schema_json.get() != serde_json::Value::Null && !loading_schema.get()>
                        <div class="border rounded-lg p-4 bg-gray-50">
                            <div class="text-sm font-medium mb-3 text-green-700">
                                "Static Value Configuration"
                            </div>
                            <SchemaFormGenerator
                                schema=schema_json.read_only()
                                mode=SchemaFormMode::StaticValue
                                output=form_output
                                color="green".to_string()
                                show_toggle=true
                            />
                        </div>
                    </Show>

                    // Schema-driven form for random/faker strategy
                    <Show when=move || strategy.get() == "random" && schema_json.get() != serde_json::Value::Null && !loading_schema.get()>
                        <div class="border rounded-lg p-4 bg-gray-50">
                            <div class="text-sm font-medium mb-3 text-purple-700">
                                "Faker Configuration"
                            </div>
                            <SchemaFormGenerator
                                schema=schema_json.read_only()
                                mode=SchemaFormMode::FakerConfig
                                output=form_output
                                color="purple".to_string()
                                show_toggle=true
                            />
                        </div>
                    </Show>

                    // Textarea fallback for other strategies or when schema not loaded
                    <Show when=move || !show_schema_form() && !loading_schema.get() && !selected_schema.get().is_empty()>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"Strategy Config (JSON)"</label>
                            <textarea
                                class="w-full px-3 py-2 border border-gray-300 rounded-lg font-mono text-sm focus:ring-purple-500 focus:border-purple-500"
                                rows="4"
                                placeholder=r#"{"locale": "en_US"}"#
                                prop:value=move || strategy_config.get()
                                on:input=move |ev| set_strategy_config.set(event_target_value(&ev))
                            />
                        </div>
                    </Show>
                </div>

                <div class="flex justify-end gap-3 mt-6 pt-4 border-t border-gray-200">
                    <button
                        class="px-4 py-2 text-gray-600 hover:bg-gray-100 rounded-lg"
                        on:click=move |_| on_close()
                    >
                        "Cancel"
                    </button>
                    <button
                        class="px-4 py-2 bg-purple-600 text-white rounded-lg hover:bg-purple-700 disabled:opacity-50"
                        disabled=move || generating.get() || selected_schema.get().is_empty() || loading_schema.get()
                        on:click=on_generate
                    >
                        {move || if generating.get() { "Generating..." } else { "Generate" }}
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Tab for Add Record modal input mode
#[derive(Clone, Copy, PartialEq)]
enum AddRecordInputMode {
    Form,
    Json,
}

/// Modal for adding a new record
#[component]
fn AddRecordModal(
    data_lake_name: String,
    data_lake: Option<DataLake>,
    on_close: impl Fn() + 'static + Clone,
    on_success: impl Fn() + 'static + Clone,
) -> impl IntoView {
    let (selected_schema, set_selected_schema) = signal(String::new());
    let (input_mode, set_input_mode) = signal(AddRecordInputMode::Form);
    let (saving, set_saving) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);

    // Schema JSON for form generation (fetched when schema is selected)
    let (schema_json, set_schema_json) = signal(serde_json::Value::Null);
    let (loading_schema, set_loading_schema) = signal(false);

    // Output from form or JSON editor - shared between both modes
    let form_output = RwSignal::new(String::from("{}"));

    // JSON text for raw editing mode
    let (json_text, set_json_text) = signal(String::from("{}"));
    let (json_error, set_json_error) = signal(Option::<String>::None);

    let on_close_clone = on_close.clone();
    let on_success_clone = on_success.clone();

    // Fetch schema JSON when schema is selected
    Effect::new(move || {
        let schema_name = selected_schema.get();
        if schema_name.is_empty() {
            set_schema_json.set(serde_json::Value::Null);
            return;
        }

        set_loading_schema.set(true);
        set_error.set(None);

        wasm_bindgen_futures::spawn_local(async move {
            match api::get_schema(&schema_name).await {
                Ok(schema) => {
                    set_schema_json.set(schema.schema);
                    // Reset form output when schema changes
                    form_output.set(String::from("{}"));
                    set_json_text.set(String::from("{}"));
                    set_loading_schema.set(false);
                }
                Err(e) => {
                    set_error.set(Some(format!("Failed to load schema: {}", e)));
                    set_schema_json.set(serde_json::Value::Null);
                    set_loading_schema.set(false);
                }
            }
        });
    });

    // Sync form output to JSON text when in form mode
    Effect::new(move || {
        let output = form_output.get();
        if input_mode.get() == AddRecordInputMode::Form {
            set_json_text.set(output);
        }
    });

    // Handle JSON text changes (in JSON mode)
    let on_json_change = move |ev: web_sys::Event| {
        use wasm_bindgen::JsCast;
        let target = ev.target().unwrap();
        let textarea: web_sys::HtmlTextAreaElement = target.dyn_into().unwrap();
        let new_text = textarea.value();
        set_json_text.set(new_text.clone());

        // Validate JSON
        match serde_json::from_str::<serde_json::Value>(&new_text) {
            Ok(_) => {
                set_json_error.set(None);
                // Sync to form output so form view updates
                form_output.set(new_text);
            }
            Err(e) => {
                set_json_error.set(Some(e.to_string()));
            }
        }
    };

    let on_save = move |_| {
        let schema = selected_schema.get();
        if schema.is_empty() {
            set_error.set(Some("Please select a schema".to_string()));
            return;
        }

        // Get data from the appropriate source
        let data_str = if input_mode.get() == AddRecordInputMode::Form {
            form_output.get()
        } else {
            json_text.get()
        };

        let data = match serde_json::from_str(&data_str) {
            Ok(v) => v,
            Err(e) => {
                set_error.set(Some(format!("Invalid JSON: {}", e)));
                return;
            }
        };

        set_saving.set(true);
        set_error.set(None);

        let lake_name = data_lake_name.clone();
        let on_success = on_success_clone.clone();

        let request = CreateRecordRequest {
            schema_name: schema,
            data,
            metadata: None,
        };

        wasm_bindgen_futures::spawn_local(async move {
            match api::create_record(&lake_name, &request).await {
                Ok(_) => {
                    on_success();
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_saving.set(false);
                }
            }
        });
    };

    view! {
        <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
            <div class="bg-white rounded-lg p-6 max-w-3xl w-full mx-4 max-h-[90vh] overflow-y-auto">
                <div class="flex justify-between items-center mb-4">
                    <h3 class="text-lg font-semibold">"Add Record"</h3>
                    <button class="text-gray-400 hover:text-gray-600" on:click=move |_| on_close_clone()>
                        <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                        </svg>
                    </button>
                </div>

                <Show when=move || error.get().is_some()>
                    <div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded mb-4 text-sm">
                        {move || error.get().unwrap_or_default()}
                    </div>
                </Show>

                <div class="space-y-4">
                    // Schema selection
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">"Schema"</label>
                        <select
                            class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-cyan-500 focus:border-cyan-500"
                            prop:value=move || selected_schema.get()
                            on:change=move |ev| set_selected_schema.set(event_target_value(&ev))
                        >
                            <option value="">"Select schema..."</option>
                            {data_lake.as_ref().map(|lake| {
                                lake.schemas.iter().map(|s| {
                                    let schema_name = s.schema_name.clone();
                                    view! {
                                        <option value=schema_name.clone()>{schema_name.clone()}</option>
                                    }
                                }).collect::<Vec<_>>()
                            }).unwrap_or_default()}
                        </select>
                    </div>

                    // Show input mode toggle and form only when schema is selected
                    <Show when=move || !selected_schema.get().is_empty()>
                        // Loading indicator
                        <Show when=move || loading_schema.get()>
                            <div class="flex items-center justify-center py-8">
                                <svg class="animate-spin h-6 w-6 text-cyan-600" fill="none" viewBox="0 0 24 24">
                                    <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"/>
                                    <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"/>
                                </svg>
                                <span class="ml-2 text-gray-600">"Loading schema..."</span>
                            </div>
                        </Show>

                        <Show when=move || !loading_schema.get() && schema_json.get() != serde_json::Value::Null>
                            // Input mode toggle
                            <div class="flex items-center justify-between mb-2">
                                <label class="block text-sm font-medium text-gray-700">"Record Data"</label>
                                <div class="inline-flex bg-gray-100 rounded-lg p-0.5">
                                    <button
                                        type="button"
                                        class=move || format!(
                                            "px-3 py-1 text-sm font-medium rounded transition-colors {}",
                                            if input_mode.get() == AddRecordInputMode::Form {
                                                "bg-white shadow text-gray-900"
                                            } else {
                                                "text-gray-600 hover:text-gray-900"
                                            }
                                        )
                                        on:click=move |_| set_input_mode.set(AddRecordInputMode::Form)
                                    >
                                        "Form"
                                    </button>
                                    <button
                                        type="button"
                                        class=move || format!(
                                            "px-3 py-1 text-sm font-medium rounded transition-colors {}",
                                            if input_mode.get() == AddRecordInputMode::Json {
                                                "bg-white shadow text-gray-900"
                                            } else {
                                                "text-gray-600 hover:text-gray-900"
                                            }
                                        )
                                        on:click=move |_| set_input_mode.set(AddRecordInputMode::Json)
                                    >
                                        "JSON"
                                    </button>
                                </div>
                            </div>

                            // Form view (schema-driven)
                            <div style=move || if input_mode.get() == AddRecordInputMode::Form { "display: block" } else { "display: none" }>
                                <div class="border border-gray-200 rounded-lg p-4 bg-gray-50 max-h-[50vh] overflow-y-auto">
                                    <SchemaFormGenerator
                                        schema=schema_json.into()
                                        mode=SchemaFormMode::StaticValue
                                        output=form_output
                                        color="cyan".to_string()
                                        show_toggle=false
                                    />
                                </div>
                                <p class="mt-1 text-xs text-gray-500">"Fill in the form fields based on the schema definition."</p>
                            </div>

                            // JSON view (raw editor)
                            <div style=move || if input_mode.get() == AddRecordInputMode::Json { "display: block" } else { "display: none" }>
                                <textarea
                                    class="w-full px-3 py-2 border border-gray-300 rounded-lg font-mono text-sm focus:ring-cyan-500 focus:border-cyan-500"
                                    rows="12"
                                    placeholder=r#"{"name": "John Doe", "email": "john@example.com"}"#
                                    prop:value=move || json_text.get()
                                    on:input=on_json_change
                                />
                                <Show when=move || json_error.get().is_some()>
                                    <p class="mt-1 text-xs text-red-500">{move || json_error.get().unwrap_or_default()}</p>
                                </Show>
                                <Show when=move || json_error.get().is_none()>
                                    <p class="mt-1 text-xs text-gray-500">"Enter the record data as JSON that conforms to the selected schema."</p>
                                </Show>
                            </div>
                        </Show>
                    </Show>

                    // Placeholder when no schema selected
                    <Show when=move || selected_schema.get().is_empty()>
                        <div class="text-center py-8 text-gray-500 bg-gray-50 rounded-lg border border-gray-200">
                            <svg class="mx-auto h-12 w-12 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
                            </svg>
                            <p class="mt-2 text-sm">"Select a schema to enter record data"</p>
                        </div>
                    </Show>
                </div>

                <div class="flex justify-end gap-3 mt-6 pt-4 border-t border-gray-200">
                    <button
                        class="px-4 py-2 text-gray-600 hover:bg-gray-100 rounded-lg"
                        on:click=move |_| on_close()
                    >
                        "Cancel"
                    </button>
                    <button
                        class="px-4 py-2 bg-cyan-600 text-white rounded-lg hover:bg-cyan-700 disabled:opacity-50"
                        disabled=move || saving.get() || selected_schema.get().is_empty() || loading_schema.get()
                        on:click=on_save
                    >
                        {move || if saving.get() { "Saving..." } else { "Add Record" }}
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Modal for editing an existing record
#[component]
fn EditRecordModal(
    data_lake_name: String,
    record: DataRecord,
    on_close: impl Fn() + 'static + Clone,
    on_success: impl Fn() + 'static + Clone,
) -> impl IntoView {
    let (input_mode, set_input_mode) = signal(AddRecordInputMode::Form);
    let (saving, set_saving) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);

    // Schema JSON for form generation (fetched on mount)
    let (schema_json, set_schema_json) = signal(serde_json::Value::Null);
    let (loading_schema, set_loading_schema) = signal(true);

    // Initialize form output with existing record data
    let initial_json = serde_json::to_string_pretty(&record.data).unwrap_or_default();
    let form_output = RwSignal::new(initial_json.clone());

    // JSON text for raw editing mode
    let (json_text, set_json_text) = signal(initial_json);
    let (json_error, set_json_error) = signal(Option::<String>::None);

    let record_id = record.id.clone();
    let schema_name = record.schema_name.clone();
    let schema_name_for_fetch = schema_name.clone();
    let schema_name_for_display = schema_name.clone();

    let on_close_clone = on_close.clone();
    let on_success_clone = on_success.clone();

    // Fetch schema JSON on mount
    Effect::new(move || {
        let schema_name = schema_name_for_fetch.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match api::get_schema(&schema_name).await {
                Ok(schema) => {
                    set_schema_json.set(schema.schema);
                    set_loading_schema.set(false);
                }
                Err(e) => {
                    set_error.set(Some(format!("Failed to load schema: {}", e)));
                    set_loading_schema.set(false);
                }
            }
        });
    });

    // Sync form output to JSON text when in form mode
    Effect::new(move || {
        let output = form_output.get();
        if input_mode.get() == AddRecordInputMode::Form {
            set_json_text.set(output);
        }
    });

    // Handle JSON text changes (in JSON mode)
    let on_json_change = move |ev: web_sys::Event| {
        use wasm_bindgen::JsCast;
        let target = ev.target().unwrap();
        let textarea: web_sys::HtmlTextAreaElement = target.dyn_into().unwrap();
        let new_text = textarea.value();
        set_json_text.set(new_text.clone());

        // Validate JSON
        match serde_json::from_str::<serde_json::Value>(&new_text) {
            Ok(_) => {
                set_json_error.set(None);
                // Sync to form output so form view updates
                form_output.set(new_text);
            }
            Err(e) => {
                set_json_error.set(Some(e.to_string()));
            }
        }
    };

    let on_save = move |_| {
        // Get data from the appropriate source
        let data_str = if input_mode.get() == AddRecordInputMode::Form {
            form_output.get()
        } else {
            json_text.get()
        };

        let data = match serde_json::from_str(&data_str) {
            Ok(v) => v,
            Err(e) => {
                set_error.set(Some(format!("Invalid JSON: {}", e)));
                return;
            }
        };

        set_saving.set(true);
        set_error.set(None);

        let lake_name = data_lake_name.clone();
        let id = record_id.clone();
        let on_success = on_success_clone.clone();

        let request = UpdateRecordRequest {
            data,
            metadata: None,
        };

        wasm_bindgen_futures::spawn_local(async move {
            match api::update_record(&lake_name, &id, &request).await {
                Ok(_) => {
                    on_success();
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_saving.set(false);
                }
            }
        });
    };

    view! {
        <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
            <div class="bg-white rounded-lg p-6 max-w-3xl w-full mx-4 max-h-[90vh] overflow-y-auto">
                <div class="flex justify-between items-center mb-4">
                    <div>
                        <h3 class="text-lg font-semibold">"Edit Record"</h3>
                        <p class="text-sm text-gray-500">
                            "Schema: "
                            <span class="px-2 py-0.5 text-xs font-semibold rounded-full bg-cyan-100 text-cyan-800">
                                {schema_name_for_display}
                            </span>
                        </p>
                    </div>
                    <button class="text-gray-400 hover:text-gray-600" on:click=move |_| on_close_clone()>
                        <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                        </svg>
                    </button>
                </div>

                <Show when=move || error.get().is_some()>
                    <div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded mb-4 text-sm">
                        {move || error.get().unwrap_or_default()}
                    </div>
                </Show>

                <div class="space-y-4">
                    // Loading indicator
                    <Show when=move || loading_schema.get()>
                        <div class="flex items-center justify-center py-8">
                            <svg class="animate-spin h-6 w-6 text-cyan-600" fill="none" viewBox="0 0 24 24">
                                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"/>
                                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"/>
                            </svg>
                            <span class="ml-2 text-gray-600">"Loading schema..."</span>
                        </div>
                    </Show>

                    <Show when=move || !loading_schema.get()>
                        // Input mode toggle
                        <div class="flex items-center justify-between mb-2">
                            <label class="block text-sm font-medium text-gray-700">"Record Data"</label>
                            <div class="inline-flex bg-gray-100 rounded-lg p-0.5">
                                <button
                                    type="button"
                                    class=move || format!(
                                        "px-3 py-1 text-sm font-medium rounded transition-colors {}",
                                        if input_mode.get() == AddRecordInputMode::Form {
                                            "bg-white shadow text-gray-900"
                                        } else {
                                            "text-gray-600 hover:text-gray-900"
                                        }
                                    )
                                    on:click=move |_| set_input_mode.set(AddRecordInputMode::Form)
                                    disabled=move || schema_json.get() == serde_json::Value::Null
                                >
                                    "Form"
                                </button>
                                <button
                                    type="button"
                                    class=move || format!(
                                        "px-3 py-1 text-sm font-medium rounded transition-colors {}",
                                        if input_mode.get() == AddRecordInputMode::Json {
                                            "bg-white shadow text-gray-900"
                                        } else {
                                            "text-gray-600 hover:text-gray-900"
                                        }
                                    )
                                    on:click=move |_| set_input_mode.set(AddRecordInputMode::Json)
                                >
                                    "JSON"
                                </button>
                            </div>
                        </div>

                        // Form view (schema-driven) - only show if schema loaded successfully
                        <Show when=move || schema_json.get() != serde_json::Value::Null>
                            <div style=move || if input_mode.get() == AddRecordInputMode::Form { "display: block" } else { "display: none" }>
                                <div class="border border-gray-200 rounded-lg p-4 bg-gray-50 max-h-[50vh] overflow-y-auto">
                                    <SchemaFormGenerator
                                        schema=schema_json.into()
                                        mode=SchemaFormMode::StaticValue
                                        output=form_output
                                        color="cyan".to_string()
                                        show_toggle=false
                                    />
                                </div>
                                <p class="mt-1 text-xs text-gray-500">"Edit the record data using the form fields."</p>
                            </div>
                        </Show>

                        // JSON view (raw editor)
                        <div style=move || if input_mode.get() == AddRecordInputMode::Json || schema_json.get() == serde_json::Value::Null { "display: block" } else { "display: none" }>
                            <textarea
                                class="w-full px-3 py-2 border border-gray-300 rounded-lg font-mono text-sm focus:ring-cyan-500 focus:border-cyan-500"
                                rows="16"
                                prop:value=move || json_text.get()
                                on:input=on_json_change
                            />
                            <Show when=move || json_error.get().is_some()>
                                <p class="mt-1 text-xs text-red-500">{move || json_error.get().unwrap_or_default()}</p>
                            </Show>
                            <Show when=move || json_error.get().is_none()>
                                <p class="mt-1 text-xs text-gray-500">"Edit the record data as JSON."</p>
                            </Show>
                        </div>
                    </Show>
                </div>

                <div class="flex justify-end gap-3 mt-6 pt-4 border-t border-gray-200">
                    <button
                        class="px-4 py-2 text-gray-600 hover:bg-gray-100 rounded-lg"
                        on:click=move |_| on_close()
                    >
                        "Cancel"
                    </button>
                    <button
                        class="px-4 py-2 bg-cyan-600 text-white rounded-lg hover:bg-cyan-700 disabled:opacity-50"
                        disabled=move || saving.get() || loading_schema.get()
                        on:click=on_save
                    >
                        {move || if saving.get() { "Saving..." } else { "Save Changes" }}
                    </button>
                </div>
            </div>
        </div>
    }
}
