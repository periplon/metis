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
use crate::api;
use crate::types::{DataLake, DataLakeSchemaRef, DataRecord, CreateRecordRequest, UpdateRecordRequest, GenerateRecordsRequest};
use crate::components::list_filter::{
    ListFilterBar, Pagination, TagBadges, TagInput,
    extract_tags, filter_items, paginate_items, total_pages,
    SortField, SortOrder, sort_items,
};

#[derive(Clone, Copy, PartialEq)]
enum ViewMode {
    Table,
    Card,
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

            // Content area
            <Suspense fallback=move || view! { <div class="text-gray-500">"Loading data lakes..."</div> }>
                {move || {
                    data_lakes.get().map(|maybe_lakes| {
                        match maybe_lakes {
                            Some(list) if !list.is_empty() => {
                                // Extract available tags from all data lakes
                                let available_tags = extract_tags(&list, |d: &DataLake| d.tags.as_slice());

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
                                let pages = total_pages(total_filtered, items_per_page);
                                let paginated = paginate_items(&filtered, current_page.get(), items_per_page);

                                view! {
                                    <div>
                                        // Filter bar with sorting
                                        <ListFilterBar
                                            search_query=search_query
                                            selected_tags=selected_tags
                                            available_tags=available_tags
                                            sort_field=sort_field
                                            sort_order=sort_order
                                            placeholder="Search data lakes by name or description..."
                                        />

                                        // Results info
                                        {(total_filtered != list.len()).then(|| view! {
                                            <p class="text-sm text-gray-500 mb-4">
                                                "Showing " {total_filtered} " of " {list.len()} " data lakes"
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
    let (name, set_name) = signal(String::new());
    let (description, set_description) = signal(String::new());
    let tags = RwSignal::new(Vec::<String>::new());
    let (schema_refs, set_schema_refs) = signal(Vec::<DataLakeSchemaRef>::new());
    let (saving, set_saving) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);

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

        let data_lake = DataLake {
            name: name_val,
            description: if desc_val.is_empty() { None } else { Some(desc_val) },
            tags: tags.get(),
            schemas,
            metadata: None,
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

    let (name, set_name) = signal(String::new());
    let (description, set_description) = signal(String::new());
    let tags = RwSignal::new(Vec::<String>::new());
    let (schema_refs, set_schema_refs) = signal(Vec::<DataLakeSchemaRef>::new());
    let (saving, set_saving) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);
    let (loaded, set_loaded) = signal(false);

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

        let data_lake = DataLake {
            name: name_val,
            description: if desc_val.is_empty() { None } else { Some(desc_val) },
            tags: tags.get(),
            schemas,
            metadata: None,
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

    let (refresh_trigger, set_refresh_trigger) = signal(0u32);
    let (selected_schema, set_selected_schema) = signal(Option::<String>::None);
    let (delete_target, set_delete_target) = signal(Option::<String>::None);
    let (deleting, set_deleting) = signal(false);
    let (show_generate_modal, set_show_generate_modal) = signal(false);
    let (show_add_modal, set_show_add_modal) = signal(false);
    let (edit_record, set_edit_record) = signal(Option::<DataRecord>::None);
    let (page, set_page) = signal(0usize);
    let page_size = 20;

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

    let total_pages = move || {
        count.get()
            .flatten()
            .map(|c| (c.count + page_size - 1) / page_size)
            .unwrap_or(1)
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

            // Records table
            <Suspense fallback=move || view! { <div class="text-gray-500">"Loading records..."</div> }>
                {move || {
                    records.get().map(|maybe_records| {
                        match maybe_records {
                            Some(response) if !response.records.is_empty() => {
                                view! {
                                    <div class="bg-white rounded-lg shadow overflow-hidden">
                                        <table class="min-w-full divide-y divide-gray-200">
                                            <thead class="bg-gray-50">
                                                <tr>
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
                                                    let id_for_delete = record.id.clone();
                                                    let record_for_edit = record.clone();
                                                    let data_preview = serde_json::to_string(&record.data)
                                                        .unwrap_or_default()
                                                        .chars()
                                                        .take(80)
                                                        .collect::<String>();
                                                    view! {
                                                        <tr class="hover:bg-gray-50">
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
                                                "Page " {move || page.get() + 1} " of " {total_pages}
                                            </div>
                                            <div class="flex gap-2">
                                                <button
                                                    class="px-3 py-1 text-sm border border-gray-300 rounded hover:bg-gray-100 disabled:opacity-50"
                                                    disabled=move || page.get() == 0
                                                    on:click=move |_| set_page.update(|p| *p = p.saturating_sub(1))
                                                >
                                                    "Previous"
                                                </button>
                                                <button
                                                    class="px-3 py-1 text-sm border border-gray-300 rounded hover:bg-gray-100 disabled:opacity-50"
                                                    disabled=move || page.get() + 1 >= total_pages()
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

    let on_close_clone = on_close.clone();
    let on_success_clone = on_success.clone();

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
        let config_str = strategy_config.get();
        let on_success = on_success_clone.clone();

        let config = if config_str.is_empty() {
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

    view! {
        <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
            <div class="bg-white rounded-lg p-6 max-w-lg w-full mx-4">
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
                            on:change=move |ev| set_strategy.set(event_target_value(&ev))
                        >
                            <option value="random">"Random (faker-based)"</option>
                            <option value="pattern">"Pattern (regex)"</option>
                            <option value="template">"Template (Tera)"</option>
                            <option value="llm">"LLM (AI-generated)"</option>
                            <option value="static">"Static (fixed value)"</option>
                        </select>
                    </div>

                    // Strategy config (optional)
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">"Strategy Config (JSON, optional)"</label>
                        <textarea
                            class="w-full px-3 py-2 border border-gray-300 rounded-lg font-mono text-sm focus:ring-purple-500 focus:border-purple-500"
                            rows="3"
                            placeholder=r#"{"locale": "en_US"}"#
                            prop:value=move || strategy_config.get()
                            on:input=move |ev| set_strategy_config.set(event_target_value(&ev))
                        />
                    </div>
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
                        disabled=move || generating.get()
                        on:click=on_generate
                    >
                        {move || if generating.get() { "Generating..." } else { "Generate" }}
                    </button>
                </div>
            </div>
        </div>
    }
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
    let (data_json, set_data_json) = signal(String::new());
    let (saving, set_saving) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);

    let on_close_clone = on_close.clone();
    let on_success_clone = on_success.clone();

    let on_save = move |_| {
        let schema = selected_schema.get();
        if schema.is_empty() {
            set_error.set(Some("Please select a schema".to_string()));
            return;
        }

        let data = match serde_json::from_str(&data_json.get()) {
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
            <div class="bg-white rounded-lg p-6 max-w-2xl w-full mx-4 max-h-[90vh] overflow-y-auto">
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

                    // Data JSON
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">"Data (JSON)"</label>
                        <textarea
                            class="w-full px-3 py-2 border border-gray-300 rounded-lg font-mono text-sm focus:ring-cyan-500 focus:border-cyan-500"
                            rows="12"
                            placeholder=r#"{"name": "John Doe", "email": "john@example.com"}"#
                            prop:value=move || data_json.get()
                            on:input=move |ev| set_data_json.set(event_target_value(&ev))
                        />
                        <p class="mt-1 text-xs text-gray-500">"Enter the record data as JSON that conforms to the selected schema."</p>
                    </div>
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
                        disabled=move || saving.get()
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
    let (data_json, set_data_json) = signal(
        serde_json::to_string_pretty(&record.data).unwrap_or_default()
    );
    let (saving, set_saving) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);

    let record_id = record.id.clone();
    let schema_name = record.schema_name.clone();

    let on_close_clone = on_close.clone();
    let on_success_clone = on_success.clone();

    let on_save = move |_| {
        let data = match serde_json::from_str(&data_json.get()) {
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
            <div class="bg-white rounded-lg p-6 max-w-2xl w-full mx-4 max-h-[90vh] overflow-y-auto">
                <div class="flex justify-between items-center mb-4">
                    <div>
                        <h3 class="text-lg font-semibold">"Edit Record"</h3>
                        <p class="text-sm text-gray-500">
                            "Schema: "
                            <span class="px-2 py-0.5 text-xs font-semibold rounded-full bg-cyan-100 text-cyan-800">
                                {schema_name}
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
                    // Data JSON
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">"Data (JSON)"</label>
                        <textarea
                            class="w-full px-3 py-2 border border-gray-300 rounded-lg font-mono text-sm focus:ring-cyan-500 focus:border-cyan-500"
                            rows="16"
                            prop:value=move || data_json.get()
                            on:input=move |ev| set_data_json.set(event_target_value(&ev))
                        />
                    </div>
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
                        disabled=move || saving.get()
                        on:click=on_save
                    >
                        {move || if saving.get() { "Saving..." } else { "Save Changes" }}
                    </button>
                </div>
            </div>
        </div>
    }
}
