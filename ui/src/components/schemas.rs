//! Schemas management page for reusable JSON Schema definitions
//!
//! Provides CRUD operations for schemas that can be referenced
//! across tools, agents, workflows, and other archetypes.

use std::collections::HashSet;
use leptos::prelude::*;
use leptos::web_sys;
use leptos_router::hooks::use_params_map;
use leptos_router::components::A;
use crate::api;
use crate::types::Schema;
use crate::components::schema_editor::{
    JsonSchemaEditor, SchemaPreview, SchemaProperty, SchemaMetadata, SchemaDefinition,
    DefinitionsEditor, schema_to_properties, schema_to_metadata, schema_to_definitions,
    full_schema_to_value,
};
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

#[component]
pub fn Schemas() -> impl IntoView {
    let (view_mode, set_view_mode) = signal(ViewMode::Table);
    let (refresh_trigger, set_refresh_trigger) = signal(0u32);
    let (delete_target, set_delete_target) = signal(Option::<String>::None);
    let (deleting, set_deleting) = signal(false);

    // Search and filter state
    let search_query = RwSignal::new(String::new());
    let selected_tags = RwSignal::new(HashSet::<String>::new());
    let sort_field = RwSignal::new(SortField::Name);
    let sort_order = RwSignal::new(SortOrder::Ascending);
    let current_page = RwSignal::new(0usize);
    let items_per_page = 10usize;

    let schemas = LocalResource::new(move || {
        let _ = refresh_trigger.get();
        async move { api::list_schemas().await.ok() }
    });

    // Reset page when filters change
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
                match api::delete_schema(&name).await {
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
                <h2 class="text-2xl font-bold">"Schemas"</h2>
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
                    <A href="/schemas/new" attr:class="px-4 py-2 bg-teal-600 text-white rounded-lg hover:bg-teal-700 flex items-center gap-2">
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/>
                        </svg>
                        "New Schema"
                    </A>
                </div>
            </div>

            // Content area
            <Suspense fallback=move || view! { <div class="text-gray-500">"Loading schemas..."</div> }>
                {move || {
                    schemas.get().map(|maybe_schemas| {
                        match maybe_schemas {
                            Some(list) if !list.is_empty() => {
                                // Extract available tags
                                let available_tags = extract_tags(&list, |s: &Schema| s.tags.as_slice());

                                // Filter items based on search and tags
                                let mut filtered = filter_items(
                                    &list,
                                    &search_query.get(),
                                    &selected_tags.get(),
                                    |s: &Schema| {
                                        format!("{} {}", s.name, s.description.as_ref().unwrap_or(&String::new()))
                                    },
                                    |s: &Schema| s.tags.as_slice(),
                                );

                                // Sort the filtered items
                                sort_items(&mut filtered, sort_field.get(), sort_order.get(), |s: &Schema| &s.name);

                                let total_filtered = filtered.len();
                                let total = list.len();
                                let pages = total_pages(total_filtered, items_per_page);
                                let items = paginate_items(&filtered, current_page.get(), items_per_page);

                                view! {
                                    <div>
                                        // Filter bar
                                        <ListFilterBar
                                            search_query=search_query
                                            selected_tags=selected_tags
                                            available_tags=available_tags
                                            sort_field=sort_field
                                            sort_order=sort_order
                                            placeholder="Search schemas..."
                                        />

                                        // Results count when filtered
                                        {(total_filtered != total).then(|| view! {
                                            <p class="text-sm text-gray-500 mb-4">
                                                "Showing " {total_filtered} " of " {total} " schemas"
                                            </p>
                                        })}

                                        // Content
                                        {if items.is_empty() {
                                            view! { <NoResultsState /> }.into_any()
                                        } else if view_mode.get() == ViewMode::Table {
                                            view! {
                                                <div class="bg-white rounded-lg shadow overflow-hidden">
                                                    <table class="min-w-full divide-y divide-gray-200">
                                                        <thead class="bg-gray-50">
                                                            <tr>
                                                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Name"</th>
                                                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Description"</th>
                                                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Properties"</th>
                                                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Tags"</th>
                                                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Reference"</th>
                                                                <th class="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">"Actions"</th>
                                                            </tr>
                                                        </thead>
                                                        <tbody class="bg-white divide-y divide-gray-200">
                                                            {items.into_iter().map(|schema| {
                                                                let name = schema.name.clone();
                                                                let name_for_delete = schema.name.clone();
                                                                let name_for_edit = schema.name.clone();
                                                                let tags = schema.tags.clone();
                                                                let prop_count = schema.schema.get("properties")
                                                                    .and_then(|p| p.as_object())
                                                                    .map(|o| o.len())
                                                                    .unwrap_or(0);
                                                                view! {
                                                                    <tr class="hover:bg-gray-50">
                                                                        <td class="px-6 py-4 whitespace-nowrap">
                                                                            <div class="text-sm font-medium text-gray-900">{name.clone()}</div>
                                                                        </td>
                                                                        <td class="px-6 py-4">
                                                                            <div class="text-sm text-gray-500 max-w-md truncate">
                                                                                {schema.description.clone().unwrap_or_else(|| "—".to_string())}
                                                                            </div>
                                                                        </td>
                                                                        <td class="px-6 py-4 whitespace-nowrap">
                                                                            <span class="px-2 inline-flex text-xs leading-5 font-semibold rounded-full bg-teal-100 text-teal-800">
                                                                                {prop_count} " properties"
                                                                            </span>
                                                                        </td>
                                                                        <td class="px-6 py-4">
                                                                            <TagBadges tags=tags />
                                                                        </td>
                                                                        <td class="px-6 py-4 whitespace-nowrap">
                                                                            <code class="text-xs bg-gray-100 px-2 py-1 rounded font-mono text-teal-600">
                                                                                {format!(r#"{{"$ref": "{}"}}"#, name)}
                                                                            </code>
                                                                        </td>
                                                                        <td class="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
                                                                            <A href=format!("/schemas/edit/{}", name_for_edit) attr:class="text-teal-600 hover:text-teal-900 mr-4">"Edit"</A>
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
                                                    {items.into_iter().map(|schema| {
                                                        let name = schema.name.clone();
                                                        let name_for_delete = schema.name.clone();
                                                        let name_for_edit = schema.name.clone();
                                                        let tags = schema.tags.clone();
                                                        let prop_count = schema.schema.get("properties")
                                                            .and_then(|p| p.as_object())
                                                            .map(|o| o.len())
                                                            .unwrap_or(0);
                                                        view! {
                                                            <div class="bg-white rounded-lg shadow p-6 hover:shadow-lg transition-shadow">
                                                                <div class="flex justify-between items-start mb-4">
                                                                    <h3 class="text-lg font-semibold text-gray-900">{name.clone()}</h3>
                                                                    <span class="px-2 py-1 text-xs font-semibold rounded-full bg-teal-100 text-teal-800">
                                                                        {prop_count} " props"
                                                                    </span>
                                                                </div>
                                                                <p class="text-sm text-gray-500 mb-4 line-clamp-2">
                                                                    {schema.description.clone().unwrap_or_else(|| "No description".to_string())}
                                                                </p>
                                                                {(!tags.is_empty()).then(|| view! {
                                                                    <div class="mb-4">
                                                                        <TagBadges tags=tags.clone() />
                                                                    </div>
                                                                })}
                                                                <div class="mb-4">
                                                                    <code class="text-xs bg-gray-100 px-2 py-1 rounded font-mono text-teal-600 block truncate">
                                                                        {format!(r#"{{"$ref": "{}"}}"#, name)}
                                                                    </code>
                                                                </div>
                                                                <div class="flex justify-end gap-2 pt-4 border-t border-gray-100">
                                                                    <A href=format!("/schemas/edit/{}", name_for_edit) attr:class="px-3 py-1 text-sm text-teal-600 hover:bg-teal-50 rounded">"Edit"</A>
                                                                    <button
                                                                        class="px-3 py-1 text-sm text-red-600 hover:bg-red-50 rounded"
                                                                        on:click=move |_| set_delete_target.set(Some(name_for_delete.clone()))
                                                                    >
                                                                        "Delete"
                                                                    </button>
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
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
                                        </svg>
                                    </div>
                                    <h3 class="text-lg font-medium text-gray-900 mb-2">"No schemas defined"</h3>
                                    <p class="text-gray-500 mb-4">"Create reusable JSON schemas that can be referenced across tools, agents, and workflows."</p>
                                    <A href="/schemas/new" attr:class="inline-flex items-center px-4 py-2 bg-teal-600 text-white rounded-lg hover:bg-teal-700">
                                        <svg class="w-5 h-5 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/>
                                        </svg>
                                        "Create your first schema"
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
                        <h3 class="text-lg font-semibold mb-4">"Delete Schema?"</h3>
                        <p class="text-gray-600 mb-2">
                            "Are you sure you want to delete schema "
                            <strong>{move || delete_target.get().unwrap_or_default()}</strong>
                            "?"
                        </p>
                        <p class="text-red-600 text-sm mb-6">
                            "Warning: Any tools, agents, or workflows referencing this schema will fail."
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
            <h3 class="mt-2 text-sm font-medium text-gray-900">"No matching schemas"</h3>
            <p class="mt-1 text-sm text-gray-500">"Try adjusting your search or filter criteria."</p>
        </div>
    }
}

/// Schema create/edit form
#[component]
pub fn SchemaForm() -> impl IntoView {
    let (name, set_name) = signal(String::new());
    let (description, set_description) = signal(String::new());
    let tags = RwSignal::new(Vec::<String>::new());
    let (properties, set_properties) = signal(Vec::<SchemaProperty>::new());
    let (metadata, set_metadata) = signal(SchemaMetadata::default());
    let (definitions, set_definitions) = signal(Vec::<SchemaDefinition>::new());
    let (json_mode, set_json_mode) = signal(false);
    let (json_text, set_json_text) = signal(String::new());
    let (saving, set_saving) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);
    let (show_metadata, set_show_metadata) = signal(false);

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        set_saving.set(true);
        set_error.set(None);

        let name_val = name.get();
        let desc_val = description.get();
        let is_json_mode = json_mode.get();

        let schema_value = if is_json_mode {
            match serde_json::from_str::<serde_json::Value>(&json_text.get()) {
                Ok(v) => v,
                Err(e) => {
                    set_error.set(Some(format!("Invalid JSON: {}", e)));
                    set_saving.set(false);
                    return;
                }
            }
        } else {
            // Use full schema with metadata and definitions
            full_schema_to_value(&metadata.get(), &definitions.get(), &properties.get())
        };

        let schema = Schema {
            name: name_val,
            description: if desc_val.is_empty() { None } else { Some(desc_val) },
            tags: tags.get(),
            schema: schema_value,
        };

        wasm_bindgen_futures::spawn_local(async move {
            match api::create_schema(&schema).await {
                Ok(_) => {
                    let window = web_sys::window().unwrap();
                    window.location().set_href("/schemas").ok();
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
                <A href="/schemas" attr:class="text-gray-500 hover:text-gray-700">
                    <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18"/>
                    </svg>
                </A>
                <h2 class="text-2xl font-bold">"New Schema"</h2>
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
                            class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-teal-500 focus:border-teal-500"
                            placeholder="e.g., UserInput, AddressSchema"
                            prop:value=move || name.get()
                            on:input=move |ev| set_name.set(event_target_value(&ev))
                        />
                        <p class="mt-1 text-xs text-gray-500">
                            "This name will be used in $ref references: "
                            <code class="bg-gray-100 px-1 rounded">{move || {
                                let n = name.get();
                                format!(r#"{{"$ref": "{}"}}"#, if n.is_empty() { "SchemaName".to_string() } else { n })
                            }}</code>
                        </p>
                    </div>

                    // Description
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">"Description"</label>
                        <input
                            type="text"
                            class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-teal-500 focus:border-teal-500"
                            placeholder="Brief description of this schema"
                            prop:value=move || description.get()
                            on:input=move |ev| set_description.set(event_target_value(&ev))
                        />
                    </div>

                    // Tags
                    <TagInput tags=tags />

                    // Mode toggle
                    <div class="flex items-center gap-4 border-b border-gray-200 pb-4">
                        <span class="text-sm font-medium text-gray-700">"Edit Mode:"</span>
                        <div class="flex bg-gray-100 rounded-lg p-1">
                            <button
                                type="button"
                                class=move || format!(
                                    "px-3 py-1 rounded text-sm font-medium transition-colors {}",
                                    if !json_mode.get() { "bg-white shadow text-gray-900" } else { "text-gray-600 hover:text-gray-900" }
                                )
                                on:click=move |_| {
                                    // Convert JSON to properties when switching to visual
                                    if json_mode.get() {
                                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json_text.get()) {
                                            set_properties.set(schema_to_properties(&val));
                                            set_metadata.set(schema_to_metadata(&val));
                                            set_definitions.set(schema_to_definitions(&val));
                                        }
                                    }
                                    set_json_mode.set(false);
                                }
                            >
                                "Visual Editor"
                            </button>
                            <button
                                type="button"
                                class=move || format!(
                                    "px-3 py-1 rounded text-sm font-medium transition-colors {}",
                                    if json_mode.get() { "bg-white shadow text-gray-900" } else { "text-gray-600 hover:text-gray-900" }
                                )
                                on:click=move |_| {
                                    // Convert properties to JSON when switching to JSON
                                    if !json_mode.get() {
                                        let schema = full_schema_to_value(&metadata.get(), &definitions.get(), &properties.get());
                                        set_json_text.set(serde_json::to_string_pretty(&schema).unwrap_or_default());
                                    }
                                    set_json_mode.set(true);
                                }
                            >
                                "JSON"
                            </button>
                        </div>
                    </div>

                    // Schema definition
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-2">"Schema Definition"</label>
                        <Show
                            when=move || json_mode.get()
                            fallback=move || view! {
                                // Metadata section (collapsible)
                                <details class="mb-4" open=show_metadata>
                                    <summary
                                        class="text-sm font-medium text-gray-600 cursor-pointer hover:text-gray-800 select-none"
                                        on:click=move |_| set_show_metadata.update(|v| *v = !*v)
                                    >
                                        "▶ Schema Metadata ($schema, $id, title)"
                                    </summary>
                                    <div class="mt-2 p-3 bg-blue-50 rounded border border-blue-200 space-y-3">
                                        <div class="grid grid-cols-2 gap-3">
                                            <div>
                                                <label class="block text-xs text-gray-600 mb-1">"$schema (JSON Schema version)"</label>
                                                <select
                                                    class="w-full px-2 py-1 text-sm border border-gray-300 rounded focus:ring-teal-500"
                                                    prop:value=move || metadata.get().schema_uri
                                                    on:change=move |ev| {
                                                        let value = event_target_value(&ev);
                                                        set_metadata.update(|m| m.schema_uri = value);
                                                    }
                                                >
                                                    <option value="">"(none)"</option>
                                                    <option value="http://json-schema.org/draft-07/schema#">"Draft-07"</option>
                                                    <option value="https://json-schema.org/draft/2019-09/schema">"Draft 2019-09"</option>
                                                    <option value="https://json-schema.org/draft/2020-12/schema">"Draft 2020-12"</option>
                                                </select>
                                            </div>
                                            <div>
                                                <label class="block text-xs text-gray-600 mb-1">"$id (Schema URI)"</label>
                                                <input
                                                    type="text"
                                                    class="w-full px-2 py-1 text-sm border border-gray-300 rounded focus:ring-teal-500"
                                                    placeholder="https://example.com/schemas/my-schema.json"
                                                    prop:value=move || metadata.get().id
                                                    on:input=move |ev| {
                                                        let value = event_target_value(&ev);
                                                        set_metadata.update(|m| m.id = value);
                                                    }
                                                />
                                            </div>
                                        </div>
                                        <div class="grid grid-cols-2 gap-3">
                                            <div>
                                                <label class="block text-xs text-gray-600 mb-1">"Title"</label>
                                                <input
                                                    type="text"
                                                    class="w-full px-2 py-1 text-sm border border-gray-300 rounded focus:ring-teal-500"
                                                    placeholder="Schema Title"
                                                    prop:value=move || metadata.get().title
                                                    on:input=move |ev| {
                                                        let value = event_target_value(&ev);
                                                        set_metadata.update(|m| m.title = value);
                                                    }
                                                />
                                            </div>
                                            <div>
                                                <label class="block text-xs text-gray-600 mb-1">"Description"</label>
                                                <input
                                                    type="text"
                                                    class="w-full px-2 py-1 text-sm border border-gray-300 rounded focus:ring-teal-500"
                                                    placeholder="Schema description"
                                                    prop:value=move || metadata.get().description
                                                    on:input=move |ev| {
                                                        let value = event_target_value(&ev);
                                                        set_metadata.update(|m| m.description = value);
                                                    }
                                                />
                                            </div>
                                        </div>
                                    </div>
                                </details>

                                // Definitions section (editable)
                                <div class="mb-4">
                                    <DefinitionsEditor
                                        definitions=definitions
                                        set_definitions=set_definitions
                                    />
                                </div>

                                <JsonSchemaEditor
                                    properties=properties
                                    set_properties=set_properties
                                    label="Properties"
                                    color="teal"
                                />
                                <SchemaPreview properties=properties />
                            }
                        >
                            <textarea
                                class="w-full h-96 px-3 py-2 border border-gray-300 rounded-lg font-mono text-sm focus:ring-teal-500 focus:border-teal-500"
                                placeholder=r#"{"type": "object", "properties": { ... }}"#
                                prop:value=move || json_text.get()
                                on:input=move |ev| set_json_text.set(event_target_value(&ev))
                            />
                        </Show>
                    </div>
                </div>

                // Submit button
                <div class="flex justify-end gap-3 mt-6 pt-6 border-t border-gray-200">
                    <A href="/schemas" attr:class="px-4 py-2 text-gray-600 hover:bg-gray-100 rounded-lg">
                        "Cancel"
                    </A>
                    <button
                        type="submit"
                        class="px-4 py-2 bg-teal-600 text-white rounded-lg hover:bg-teal-700 disabled:opacity-50"
                        disabled=move || saving.get()
                    >
                        {move || if saving.get() { "Creating..." } else { "Create Schema" }}
                    </button>
                </div>
            </form>
        </div>
    }
}

/// Schema edit form
#[component]
pub fn SchemaEditForm() -> impl IntoView {
    let params = use_params_map();
    let original_name = move || params.read().get("name").unwrap_or_default();

    let (name, set_name) = signal(String::new());
    let (description, set_description) = signal(String::new());
    let tags = RwSignal::new(Vec::<String>::new());
    let (properties, set_properties) = signal(Vec::<SchemaProperty>::new());
    let (metadata, set_metadata) = signal(SchemaMetadata::default());
    let (definitions, set_definitions) = signal(Vec::<SchemaDefinition>::new());
    let (json_mode, set_json_mode) = signal(false);
    let (json_text, set_json_text) = signal(String::new());
    let (saving, set_saving) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);
    let (loaded, set_loaded) = signal(false);
    let (show_metadata, set_show_metadata) = signal(false);

    // Load existing schema
    Effect::new(move |_| {
        let schema_name = original_name();
        if !schema_name.is_empty() && !loaded.get() {
            wasm_bindgen_futures::spawn_local(async move {
                match api::get_schema(&schema_name).await {
                    Ok(schema) => {
                        set_name.set(schema.name);
                        set_description.set(schema.description.unwrap_or_default());
                        tags.set(schema.tags.clone());
                        set_properties.set(schema_to_properties(&schema.schema));
                        set_metadata.set(schema_to_metadata(&schema.schema));
                        set_definitions.set(schema_to_definitions(&schema.schema));
                        set_json_text.set(serde_json::to_string_pretty(&schema.schema).unwrap_or_default());
                        // Auto-expand metadata section if it has content
                        let meta = schema_to_metadata(&schema.schema);
                        if !meta.schema_uri.is_empty() || !meta.id.is_empty() || !meta.title.is_empty() {
                            set_show_metadata.set(true);
                        }
                        set_loaded.set(true);
                    }
                    Err(e) => {
                        set_error.set(Some(format!("Failed to load schema: {}", e)));
                    }
                }
            });
        }
    });

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        set_saving.set(true);
        set_error.set(None);

        let orig_name = original_name();
        let name_val = name.get();
        let desc_val = description.get();
        let is_json_mode = json_mode.get();

        let schema_value = if is_json_mode {
            match serde_json::from_str::<serde_json::Value>(&json_text.get()) {
                Ok(v) => v,
                Err(e) => {
                    set_error.set(Some(format!("Invalid JSON: {}", e)));
                    set_saving.set(false);
                    return;
                }
            }
        } else {
            // Use full schema with metadata and definitions
            full_schema_to_value(&metadata.get(), &definitions.get(), &properties.get())
        };

        let schema = Schema {
            name: name_val,
            description: if desc_val.is_empty() { None } else { Some(desc_val) },
            tags: tags.get(),
            schema: schema_value,
        };

        wasm_bindgen_futures::spawn_local(async move {
            match api::update_schema(&orig_name, &schema).await {
                Ok(_) => {
                    let window = web_sys::window().unwrap();
                    window.location().set_href("/schemas").ok();
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
                <A href="/schemas" attr:class="text-gray-500 hover:text-gray-700">
                    <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18"/>
                    </svg>
                </A>
                <h2 class="text-2xl font-bold">"Edit Schema"</h2>
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
                <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-teal-600"></div>
                <span class="ml-3 text-gray-500">"Loading schema..."</span>
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
                                class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-teal-500 focus:border-teal-500"
                                placeholder="e.g., UserInput, AddressSchema"
                                prop:value=move || name.get()
                                on:input=move |ev| set_name.set(event_target_value(&ev))
                            />
                            <p class="mt-1 text-xs text-gray-500">
                                "Reference: "
                                <code class="bg-gray-100 px-1 rounded">{move || format!(r#"{{"$ref": "{}"}}"#, name.get())}</code>
                            </p>
                        </div>

                        // Description
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"Description"</label>
                            <input
                                type="text"
                                class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-teal-500 focus:border-teal-500"
                                placeholder="Brief description of this schema"
                                prop:value=move || description.get()
                                on:input=move |ev| set_description.set(event_target_value(&ev))
                            />
                        </div>

                        // Tags
                        <TagInput tags=tags />

                        // Mode toggle
                        <div class="flex items-center gap-4 border-b border-gray-200 pb-4">
                            <span class="text-sm font-medium text-gray-700">"Edit Mode:"</span>
                            <div class="flex bg-gray-100 rounded-lg p-1">
                                <button
                                    type="button"
                                    class=move || format!(
                                        "px-3 py-1 rounded text-sm font-medium transition-colors {}",
                                        if !json_mode.get() { "bg-white shadow text-gray-900" } else { "text-gray-600 hover:text-gray-900" }
                                    )
                                    on:click=move |_| {
                                        if json_mode.get() {
                                            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json_text.get()) {
                                                set_properties.set(schema_to_properties(&val));
                                                set_metadata.set(schema_to_metadata(&val));
                                                set_definitions.set(schema_to_definitions(&val));
                                            }
                                        }
                                        set_json_mode.set(false);
                                    }
                                >
                                    "Visual Editor"
                                </button>
                                <button
                                    type="button"
                                    class=move || format!(
                                        "px-3 py-1 rounded text-sm font-medium transition-colors {}",
                                        if json_mode.get() { "bg-white shadow text-gray-900" } else { "text-gray-600 hover:text-gray-900" }
                                    )
                                    on:click=move |_| {
                                        if !json_mode.get() {
                                            let schema = full_schema_to_value(&metadata.get(), &definitions.get(), &properties.get());
                                            set_json_text.set(serde_json::to_string_pretty(&schema).unwrap_or_default());
                                        }
                                        set_json_mode.set(true);
                                    }
                                >
                                    "JSON"
                                </button>
                            </div>
                        </div>

                        // Schema definition
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-2">"Schema Definition"</label>
                            <Show
                                when=move || json_mode.get()
                                fallback=move || view! {
                                    // Metadata section (collapsible)
                                    <details class="mb-4" open=show_metadata>
                                        <summary
                                            class="text-sm font-medium text-gray-600 cursor-pointer hover:text-gray-800 select-none"
                                            on:click=move |_| set_show_metadata.update(|v| *v = !*v)
                                        >
                                            "▶ Schema Metadata ($schema, $id, title)"
                                        </summary>
                                        <div class="mt-2 p-3 bg-blue-50 rounded border border-blue-200 space-y-3">
                                            <div class="grid grid-cols-2 gap-3">
                                                <div>
                                                    <label class="block text-xs text-gray-600 mb-1">"$schema (JSON Schema version)"</label>
                                                    <select
                                                        class="w-full px-2 py-1 text-sm border border-gray-300 rounded focus:ring-teal-500"
                                                        prop:value=move || metadata.get().schema_uri
                                                        on:change=move |ev| {
                                                            let value = event_target_value(&ev);
                                                            set_metadata.update(|m| m.schema_uri = value);
                                                        }
                                                    >
                                                        <option value="">"(none)"</option>
                                                        <option value="http://json-schema.org/draft-07/schema#">"Draft-07"</option>
                                                        <option value="https://json-schema.org/draft/2019-09/schema">"Draft 2019-09"</option>
                                                        <option value="https://json-schema.org/draft/2020-12/schema">"Draft 2020-12"</option>
                                                    </select>
                                                </div>
                                                <div>
                                                    <label class="block text-xs text-gray-600 mb-1">"$id (Schema URI)"</label>
                                                    <input
                                                        type="text"
                                                        class="w-full px-2 py-1 text-sm border border-gray-300 rounded focus:ring-teal-500"
                                                        placeholder="https://example.com/schemas/my-schema.json"
                                                        prop:value=move || metadata.get().id
                                                        on:input=move |ev| {
                                                            let value = event_target_value(&ev);
                                                            set_metadata.update(|m| m.id = value);
                                                        }
                                                    />
                                                </div>
                                            </div>
                                            <div class="grid grid-cols-2 gap-3">
                                                <div>
                                                    <label class="block text-xs text-gray-600 mb-1">"Title"</label>
                                                    <input
                                                        type="text"
                                                        class="w-full px-2 py-1 text-sm border border-gray-300 rounded focus:ring-teal-500"
                                                        placeholder="Schema Title"
                                                        prop:value=move || metadata.get().title
                                                        on:input=move |ev| {
                                                            let value = event_target_value(&ev);
                                                            set_metadata.update(|m| m.title = value);
                                                        }
                                                    />
                                                </div>
                                                <div>
                                                    <label class="block text-xs text-gray-600 mb-1">"Description"</label>
                                                    <input
                                                        type="text"
                                                        class="w-full px-2 py-1 text-sm border border-gray-300 rounded focus:ring-teal-500"
                                                        placeholder="Schema description"
                                                        prop:value=move || metadata.get().description
                                                        on:input=move |ev| {
                                                            let value = event_target_value(&ev);
                                                            set_metadata.update(|m| m.description = value);
                                                        }
                                                    />
                                                </div>
                                            </div>
                                        </div>
                                    </details>

                                    // Definitions section (editable)
                                    <div class="mb-4">
                                        <DefinitionsEditor
                                            definitions=definitions
                                            set_definitions=set_definitions
                                        />
                                    </div>

                                    <JsonSchemaEditor
                                        properties=properties
                                        set_properties=set_properties
                                        label="Properties"
                                        color="teal"
                                    />
                                    <SchemaPreview properties=properties />
                                }
                            >
                                <textarea
                                    class="w-full h-96 px-3 py-2 border border-gray-300 rounded-lg font-mono text-sm focus:ring-teal-500 focus:border-teal-500"
                                    placeholder=r#"{"type": "object", "properties": { ... }}"#
                                    prop:value=move || json_text.get()
                                    on:input=move |ev| set_json_text.set(event_target_value(&ev))
                                />
                            </Show>
                        </div>
                </div>

                // Submit button
                <div class="flex justify-end gap-3 mt-6 pt-6 border-t border-gray-200">
                    <A href="/schemas" attr:class="px-4 py-2 text-gray-600 hover:bg-gray-100 rounded-lg">
                        "Cancel"
                    </A>
                    <button
                        type="submit"
                        class="px-4 py-2 bg-teal-600 text-white rounded-lg hover:bg-teal-700 disabled:opacity-50"
                        disabled=move || saving.get()
                    >
                        {move || if saving.get() { "Saving..." } else { "Save Changes" }}
                    </button>
                </div>
            </form>
        </div>
    }
}
