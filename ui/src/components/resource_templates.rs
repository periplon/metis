//! Resource Templates UI Component
//!
//! Resource templates are URI patterns with {placeholder} variables that define
//! a family of dynamic resources. Unlike plain resources, templates have both
//! input_schema (for URI variables) and output_schema.

use leptos::prelude::*;
use leptos::web_sys;
use leptos_router::hooks::use_params_map;
use wasm_bindgen::JsCast;
use std::collections::HashSet;
use crate::api;
use crate::types::{
    ResourceTemplate, MockConfig, MockStrategyType, StatefulConfig, StateOperation,
    FileConfig, ScriptLang, LLMConfig, LLMProvider, MockDatabaseConfig,
};
use crate::components::schema_editor::FullSchemaEditor;
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
pub fn ResourceTemplates() -> impl IntoView {
    let (view_mode, set_view_mode) = signal(ViewMode::Table);
    let (refresh_trigger, set_refresh_trigger) = signal(0u32);
    let (delete_target, set_delete_target) = signal(Option::<String>::None);
    let (deleting, set_deleting) = signal(false);

    // Test modal state
    let (test_target, set_test_target) = signal(Option::<String>::None);
    let (test_input, set_test_input) = signal(String::from("{}"));
    let (test_result, set_test_result) = signal(Option::<Result<crate::types::TestResult, String>>::None);
    let (testing, set_testing) = signal(false);

    // Search and filter state
    let search_query = RwSignal::new(String::new());
    let selected_tags = RwSignal::new(HashSet::<String>::new());
    let sort_field = RwSignal::new(SortField::Name);
    let sort_order = RwSignal::new(SortOrder::Ascending);
    let current_page = RwSignal::new(0usize);
    let items_per_page = 10usize;

    let templates = LocalResource::new(move || {
        let _ = refresh_trigger.get();
        async move { api::list_resource_templates().await.ok() }
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
        if let Some(uri_template) = delete_target.get() {
            set_deleting.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                match api::delete_resource_template(&uri_template).await {
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

    let on_test_run = move |_| {
        if let Some(uri_template) = test_target.get() {
            set_testing.set(true);
            set_test_result.set(None);
            let input_json = test_input.get();
            wasm_bindgen_futures::spawn_local(async move {
                let args: serde_json::Value = serde_json::from_str(&input_json)
                    .unwrap_or(serde_json::json!({}));
                let result = api::test_resource_template(&uri_template, &args).await;
                set_test_result.set(Some(result));
                set_testing.set(false);
            });
        }
    };

    let on_test_close = move |_| {
        set_test_target.set(None);
        set_test_input.set("{}".to_string());
        set_test_result.set(None);
    };

    view! {
        <div class="p-6">
            // Header with title, view toggle, and new button
            <div class="flex justify-between items-center mb-6">
                <div>
                    <h2 class="text-2xl font-bold">"Resource Templates"</h2>
                    <p class="text-sm text-gray-500 mt-1">"URI patterns with {placeholder} variables for dynamic resources"</p>
                </div>
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
                    <a href="/resource-templates/new" class="bg-purple-500 hover:bg-purple-600 text-white px-4 py-2 rounded flex items-center gap-2">
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/>
                        </svg>
                        "New Template"
                    </a>
                </div>
            </div>

            // Delete confirmation modal
            {move || delete_target.get().map(|uri_template| view! {
                <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
                    <div class="bg-white rounded-lg shadow-xl p-6 max-w-md w-full mx-4">
                        <h3 class="text-lg font-semibold text-gray-900 mb-2">"Delete Resource Template?"</h3>
                        <p class="text-gray-600 mb-4">
                            "Are you sure you want to delete "
                            <span class="font-mono text-sm bg-gray-100 px-1 rounded">{uri_template.clone()}</span>
                            "? This action cannot be undone."
                        </p>
                        <div class="flex justify-end gap-3">
                            <button
                                class="px-4 py-2 text-gray-700 border border-gray-300 rounded hover:bg-gray-50"
                                on:click=move |_| set_delete_target.set(None)
                                disabled=move || deleting.get()
                            >
                                "Cancel"
                            </button>
                            <button
                                class="px-4 py-2 bg-red-500 text-white rounded hover:bg-red-600 disabled:opacity-50"
                                on:click=on_delete_confirm
                                disabled=move || deleting.get()
                            >
                                {move || if deleting.get() { "Deleting..." } else { "Delete" }}
                            </button>
                        </div>
                    </div>
                </div>
            })}

            // Test modal
            {move || test_target.get().map(|uri_template| view! {
                <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
                    <div class="bg-white rounded-lg shadow-xl p-6 max-w-2xl w-full mx-4 max-h-[90vh] overflow-y-auto">
                        <div class="flex justify-between items-center mb-4">
                            <h3 class="text-lg font-semibold text-gray-900">
                                "Test Resource Template: "
                                <span class="font-mono text-purple-600">{uri_template.clone()}</span>
                            </h3>
                            <button
                                class="text-gray-400 hover:text-gray-600"
                                on:click=on_test_close
                            >
                                <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                                </svg>
                            </button>
                        </div>

                        <div class="mb-4">
                            <div class="text-sm text-gray-600 p-3 bg-purple-50 border border-purple-200 rounded-md mb-3">
                                "Provide values for the URI template variables (e.g., " <code class="bg-purple-100 px-1 rounded">"{\"id\": 123}"</code> " for a template like " <code class="bg-purple-100 px-1 rounded">"postgres://db/users/{id}"</code> ")."
                            </div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"Template Variables (JSON)"</label>
                            <textarea
                                rows=4
                                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500 font-mono text-sm"
                                placeholder=r#"{"id": 123}"#
                                prop:value=move || test_input.get()
                                on:input=move |ev| {
                                    let target = ev.target().unwrap();
                                    let textarea: web_sys::HtmlTextAreaElement = target.dyn_into().unwrap();
                                    set_test_input.set(textarea.value());
                                }
                            />
                        </div>

                        <div class="flex gap-3 mb-4">
                            <button
                                class="px-4 py-2 bg-purple-500 text-white rounded hover:bg-purple-600 disabled:opacity-50 flex items-center gap-2"
                                on:click=on_test_run
                                disabled=move || testing.get()
                            >
                                {move || if testing.get() {
                                    view! {
                                        <span class="flex items-center gap-2">
                                            <svg class="animate-spin h-4 w-4" fill="none" viewBox="0 0 24 24">
                                                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                                                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                                            </svg>
                                            "Running..."
                                        </span>
                                    }.into_any()
                                } else {
                                    view! {
                                        <span class="flex items-center gap-2">
                                            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z"/>
                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                                            </svg>
                                            "Run Test"
                                        </span>
                                    }.into_any()
                                }}
                            </button>
                            <button
                                class="px-4 py-2 text-gray-700 border border-gray-300 rounded hover:bg-gray-50"
                                on:click=on_test_close
                            >
                                "Close"
                            </button>
                        </div>

                        // Test result display
                        {move || test_result.get().map(|result| {
                            match result {
                                Ok(res) => view! {
                                    <div class="border-t pt-4">
                                        <div class="flex items-center justify-between mb-2">
                                            <h4 class="font-medium text-gray-900">"Output"</h4>
                                            <span class="text-sm text-gray-500">
                                                {format!("{}ms", res.execution_time_ms)}
                                            </span>
                                        </div>
                                        {res.error.clone().map(|err| view! {
                                            <div class="mb-2 p-2 bg-red-50 border border-red-200 rounded text-red-700 text-sm">
                                                {err}
                                            </div>
                                        })}
                                        <pre class="bg-gray-900 text-purple-400 p-4 rounded-lg overflow-x-auto text-sm font-mono">
                                            {serde_json::to_string_pretty(&res.output).unwrap_or_else(|_| res.output.to_string())}
                                        </pre>
                                    </div>
                                }.into_any(),
                                Err(e) => view! {
                                    <div class="border-t pt-4">
                                        <div class="p-3 bg-red-50 border border-red-200 rounded-lg">
                                            <h4 class="font-medium text-red-800 mb-1">"Error"</h4>
                                            <p class="text-red-600 text-sm">{e}</p>
                                        </div>
                                    </div>
                                }.into_any(),
                            }
                        })}
                    </div>
                </div>
            })}

            <Suspense fallback=move || view! { <LoadingState /> }>
                {move || {
                    templates.get().map(|data| {
                        match data {
                            Some(list) if !list.is_empty() => {
                                // Extract available tags
                                let available_tags = extract_tags(&list, |t: &ResourceTemplate| t.tags.as_slice());

                                // Filter items
                                let mut filtered_list = filter_items(
                                    &list,
                                    &search_query.get(),
                                    &selected_tags.get(),
                                    |t: &ResourceTemplate| format!("{} {} {}",
                                        t.name,
                                        t.uri_template,
                                        t.description.as_ref().unwrap_or(&String::new())
                                    ),
                                    |t: &ResourceTemplate| t.tags.as_slice()
                                );

                                // Sort the filtered items
                                sort_items(&mut filtered_list, sort_field.get(), sort_order.get(), |t: &ResourceTemplate| &t.name);

                                // Paginate filtered results
                                let total_count = filtered_list.len();
                                let total_pages_count = total_pages(total_count, items_per_page);
                                let paginated_list = paginate_items(&filtered_list, current_page.get(), items_per_page);

                                view! {
                                    <div>
                                        // Filter bar
                                        <ListFilterBar
                                            search_query=search_query
                                            selected_tags=selected_tags
                                            available_tags=available_tags.clone()
                                            sort_field=sort_field
                                            sort_order=sort_order
                                            placeholder="Search templates by name, URI template, or description..."
                                        />

                                        // Results count when filtered
                                        {move || {
                                            let is_filtered = !search_query.get().is_empty() || !selected_tags.get().is_empty();
                                            if is_filtered {
                                                view! {
                                                    <div class="mb-4 text-sm text-gray-600">
                                                        {format!("Showing {} of {} templates", total_count, list.len())}
                                                    </div>
                                                }.into_any()
                                            } else {
                                                view! { <div></div> }.into_any()
                                            }
                                        }}

                                        // Content (table or cards)
                                        {if paginated_list.is_empty() {
                                            view! { <NoResultsState /> }.into_any()
                                        } else if view_mode.get() == ViewMode::Table {
                                            view! { <ResourceTemplateTable templates=paginated_list set_delete_target=set_delete_target set_test_target=set_test_target /> }.into_any()
                                        } else {
                                            view! { <ResourceTemplateCards templates=paginated_list set_delete_target=set_delete_target set_test_target=set_test_target /> }.into_any()
                                        }}

                                        // Pagination
                                        <Pagination
                                            current_page=current_page
                                            total_pages=total_pages_count
                                            total_items=total_count
                                            items_per_page=items_per_page
                                        />
                                    </div>
                                }.into_any()
                            },
                            Some(_) => view! { <EmptyState /> }.into_any(),
                            None => view! { <ErrorState /> }.into_any(),
                        }
                    })
                }}
            </Suspense>
        </div>
    }
}

#[component]
fn LoadingState() -> impl IntoView {
    view! {
        <div class="flex items-center justify-center py-12">
            <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-purple-500"></div>
            <span class="ml-3 text-gray-500">"Loading resource templates..."</span>
        </div>
    }
}

#[component]
fn EmptyState() -> impl IntoView {
    view! {
        <div class="text-center py-12 bg-white rounded-lg shadow">
            <svg class="mx-auto h-12 w-12 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 5a1 1 0 011-1h14a1 1 0 011 1v2a1 1 0 01-1 1H5a1 1 0 01-1-1V5zM4 13a1 1 0 011-1h6a1 1 0 011 1v6a1 1 0 01-1 1H5a1 1 0 01-1-1v-6zM16 13a1 1 0 011-1h2a1 1 0 011 1v6a1 1 0 01-1 1h-2a1 1 0 01-1-1v-6z"/>
            </svg>
            <h3 class="mt-2 text-sm font-medium text-gray-900">"No resource templates"</h3>
            <p class="mt-1 text-sm text-gray-500">"Create a resource template with URI pattern variables."</p>
            <div class="mt-6">
                <a href="/resource-templates/new" class="inline-flex items-center px-4 py-2 bg-purple-500 text-white rounded hover:bg-purple-600">
                    <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/>
                    </svg>
                    "New Template"
                </a>
            </div>
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
            <h3 class="mt-2 text-sm font-medium text-gray-900">"No matching templates"</h3>
            <p class="mt-1 text-sm text-gray-500">"Try adjusting your search or filter criteria."</p>
        </div>
    }
}

#[component]
fn ErrorState() -> impl IntoView {
    view! {
        <div class="text-center py-12 bg-red-50 rounded-lg border border-red-200">
            <svg class="mx-auto h-12 w-12 text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/>
            </svg>
            <h3 class="mt-2 text-sm font-medium text-red-800">"Failed to load resource templates"</h3>
            <p class="mt-1 text-sm text-red-600">"Please check your connection and try again."</p>
        </div>
    }
}

#[component]
fn ResourceTemplateTable(
    templates: Vec<ResourceTemplate>,
    set_delete_target: WriteSignal<Option<String>>,
    set_test_target: WriteSignal<Option<String>>,
) -> impl IntoView {
    view! {
        <div class="bg-white rounded-lg shadow overflow-hidden">
            <table class="min-w-full divide-y divide-gray-200">
                <thead class="bg-gray-50">
                    <tr>
                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Name"</th>
                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"URI Template"</th>
                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Type"</th>
                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Strategy"</th>
                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Tags"</th>
                        <th class="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">"Actions"</th>
                    </tr>
                </thead>
                <tbody class="bg-white divide-y divide-gray-200">
                    {templates.into_iter().map(|template| {
                        let uri_for_edit = template.uri_template.clone();
                        let uri_for_delete = template.uri_template.clone();
                        let uri_for_test = template.uri_template.clone();
                        let strategy = template.mock.as_ref()
                            .map(|m| format!("{:?}", m.strategy))
                            .unwrap_or_else(|| if template.content.is_some() { "Static".to_string() } else { "-".to_string() });
                        let mime = template.mime_type.clone().unwrap_or_else(|| "-".to_string());
                        let tags = template.tags.clone();

                        view! {
                            <tr class="hover:bg-gray-50">
                                <td class="px-6 py-4 whitespace-nowrap">
                                    <div class="font-medium text-gray-900">{template.name.clone()}</div>
                                    <div class="text-sm text-gray-500">{template.description.clone().unwrap_or_default()}</div>
                                </td>
                                <td class="px-6 py-4 whitespace-nowrap text-sm font-mono text-purple-600">
                                    {template.uri_template.clone()}
                                </td>
                                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                                    {mime}
                                </td>
                                <td class="px-6 py-4 whitespace-nowrap">
                                    <span class="px-2 py-1 text-xs font-semibold rounded-full bg-purple-100 text-purple-800">
                                        {strategy}
                                    </span>
                                </td>
                                <td class="px-6 py-4"><TagBadges tags=tags /></td>
                                <td class="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
                                    <button
                                        class="text-purple-600 hover:text-purple-900 mr-3"
                                        on:click=move |_| set_test_target.set(Some(uri_for_test.clone()))
                                    >
                                        "Test"
                                    </button>
                                    <a
                                        href=format!("/resource-templates/edit/{}", urlencoding::encode(&uri_for_edit))
                                        class="text-purple-600 hover:text-purple-900 mr-3"
                                    >
                                        "Edit"
                                    </a>
                                    <button
                                        class="text-red-600 hover:text-red-900"
                                        on:click=move |_| set_delete_target.set(Some(uri_for_delete.clone()))
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
    }
}

#[component]
fn ResourceTemplateCards(
    templates: Vec<ResourceTemplate>,
    set_delete_target: WriteSignal<Option<String>>,
    set_test_target: WriteSignal<Option<String>>,
) -> impl IntoView {
    view! {
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {templates.into_iter().map(|template| {
                let uri_for_edit = template.uri_template.clone();
                let uri_for_delete = template.uri_template.clone();
                let uri_for_test = template.uri_template.clone();
                let strategy = template.mock.as_ref()
                    .map(|m| format!("{:?}", m.strategy))
                    .unwrap_or_else(|| if template.content.is_some() { "Static".to_string() } else { "-".to_string() });
                let mime = template.mime_type.clone().unwrap_or_else(|| "-".to_string());
                let tags = template.tags.clone();

                view! {
                    <div class="bg-white rounded-lg shadow hover:shadow-md transition-shadow p-4 border-l-4 border-purple-500">
                        <div class="flex justify-between items-start mb-3">
                            <div class="flex-1 min-w-0">
                                <h3 class="font-semibold text-gray-900 truncate">{template.name.clone()}</h3>
                                <p class="text-sm text-gray-500 truncate">{template.description.clone().unwrap_or_default()}</p>
                            </div>
                            <span class="ml-2 px-2 py-1 text-xs font-semibold rounded-full bg-purple-100 text-purple-800 flex-shrink-0">
                                {strategy}
                            </span>
                        </div>
                        <div class="space-y-2 mb-4">
                            <div class="flex items-center text-sm">
                                <span class="text-gray-500 w-20">"Template:"</span>
                                <span class="font-mono text-purple-600 truncate">{template.uri_template.clone()}</span>
                            </div>
                            <div class="flex items-center text-sm">
                                <span class="text-gray-500 w-20">"Type:"</span>
                                <span class="text-gray-700">{mime}</span>
                            </div>
                            {if !tags.is_empty() {
                                view! {
                                    <div class="text-sm">
                                        <span class="text-gray-500">"Tags: "</span>
                                        <TagBadges tags=tags />
                                    </div>
                                }.into_any()
                            } else {
                                view! { <div></div> }.into_any()
                            }}
                        </div>
                        <div class="flex justify-end gap-2 pt-3 border-t border-gray-100">
                            <button
                                class="px-3 py-1 text-sm text-purple-600 hover:bg-purple-50 rounded"
                                on:click=move |_| set_test_target.set(Some(uri_for_test.clone()))
                            >
                                "Test"
                            </button>
                            <a
                                href=format!("/resource-templates/edit/{}", urlencoding::encode(&uri_for_edit))
                                class="px-3 py-1 text-sm text-purple-600 hover:bg-purple-50 rounded"
                            >
                                "Edit"
                            </a>
                            <button
                                class="px-3 py-1 text-sm text-red-600 hover:bg-red-50 rounded"
                                on:click=move |_| set_delete_target.set(Some(uri_for_delete.clone()))
                            >
                                "Delete"
                            </button>
                        </div>
                    </div>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
}

// URL encoding helper
mod urlencoding {
    pub fn encode(s: &str) -> String {
        s.chars()
            .map(|c| match c {
                'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
                _ => format!("%{:02X}", c as u8),
            })
            .collect()
    }

    pub fn decode(s: &str) -> String {
        let mut result = String::new();
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '%' {
                let hex: String = chars.by_ref().take(2).collect();
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                } else {
                    result.push('%');
                    result.push_str(&hex);
                }
            } else {
                result.push(c);
            }
        }
        result
    }
}

#[component]
pub fn ResourceTemplateForm() -> impl IntoView {
    let (name, set_name) = signal(String::new());
    let (uri_template, set_uri_template) = signal(String::new());
    let (description, set_description) = signal(String::new());
    let (mime_type, set_mime_type) = signal(String::new());
    let (content, set_content) = signal(String::new());
    let (error, set_error) = signal(Option::<String>::None);
    let (saving, set_saving) = signal(false);
    let tags = RwSignal::new(Vec::new());

    // Schema signals - using Value for full JSON schema support
    let default_schema = serde_json::json!({
        "type": "object",
        "properties": {}
    });
    let (input_schema, set_input_schema) = signal(default_schema.clone());
    let (output_schema, set_output_schema) = signal(default_schema);

    // Mock strategy signals
    let (mock_strategy, set_mock_strategy) = signal("none".to_string());
    let (mock_template, set_mock_template) = signal(String::new());
    let (mock_faker_type, set_mock_faker_type) = signal(String::new());
    let (mock_state_key, set_mock_state_key) = signal(String::new());
    let (mock_state_operation, set_mock_state_operation) = signal("get".to_string());
    let (mock_script, set_mock_script) = signal(String::new());
    let (mock_script_lang, set_mock_script_lang) = signal("rhai".to_string());
    let (mock_file_path, set_mock_file_path) = signal(String::new());
    let (mock_file_selection, set_mock_file_selection) = signal("random".to_string());
    let (mock_pattern, set_mock_pattern) = signal(String::new());
    let (mock_llm_provider, set_mock_llm_provider) = signal("openai".to_string());
    let (mock_llm_model, set_mock_llm_model) = signal(String::new());
    let (mock_llm_api_key_env, set_mock_llm_api_key_env) = signal(String::new());
    let (mock_llm_system_prompt, set_mock_llm_system_prompt) = signal(String::new());
    let (mock_db_url, set_mock_db_url) = signal(String::new());
    let (mock_db_query, set_mock_db_query) = signal(String::new());

    let build_mock_config = move || -> Option<MockConfig> {
        let strategy = mock_strategy.get();
        if strategy == "none" {
            return None;
        }

        let strategy_type = match strategy.as_str() {
            "static" => MockStrategyType::Static,
            "template" => MockStrategyType::Template,
            "random" => MockStrategyType::Random,
            "stateful" => MockStrategyType::Stateful,
            "script" => MockStrategyType::Script,
            "file" => MockStrategyType::File,
            "pattern" => MockStrategyType::Pattern,
            "llm" => MockStrategyType::LLM,
            "database" => MockStrategyType::Database,
            _ => return None,
        };

        let mut config = MockConfig {
            strategy: strategy_type,
            ..Default::default()
        };

        match strategy.as_str() {
            "template" => {
                config.template = Some(mock_template.get());
            }
            "random" => {
                if !mock_faker_type.get().is_empty() {
                    config.faker_type = Some(mock_faker_type.get());
                }
            }
            "stateful" => {
                config.stateful = Some(StatefulConfig {
                    state_key: mock_state_key.get(),
                    operation: match mock_state_operation.get().as_str() {
                        "set" => StateOperation::Set,
                        "increment" => StateOperation::Increment,
                        _ => StateOperation::Get,
                    },
                    template: if mock_template.get().is_empty() { None } else { Some(mock_template.get()) },
                });
            }
            "script" => {
                config.script = Some(mock_script.get());
                config.script_lang = Some(match mock_script_lang.get().as_str() {
                    "lua" => ScriptLang::Lua,
                    "js" => ScriptLang::Js,
                    "python" => ScriptLang::Python,
                    _ => ScriptLang::Rhai,
                });
            }
            "file" => {
                config.file = Some(FileConfig {
                    path: mock_file_path.get(),
                    selection: mock_file_selection.get(),
                });
            }
            "pattern" => {
                config.pattern = Some(mock_pattern.get());
            }
            "llm" => {
                config.llm = Some(LLMConfig {
                    provider: match mock_llm_provider.get().as_str() {
                        "anthropic" => LLMProvider::Anthropic,
                        _ => LLMProvider::OpenAI,
                    },
                    api_key_env: if mock_llm_api_key_env.get().is_empty() { None } else { Some(mock_llm_api_key_env.get()) },
                    model: mock_llm_model.get(),
                    system_prompt: if mock_llm_system_prompt.get().is_empty() { None } else { Some(mock_llm_system_prompt.get()) },
                    temperature: None,
                    max_tokens: None,
                    stream: false,
                });
            }
            "database" => {
                config.database = Some(MockDatabaseConfig {
                    url: mock_db_url.get(),
                    query: mock_db_query.get(),
                    params: vec![],
                });
            }
            _ => {}
        }

        Some(config)
    };

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        set_saving.set(true);
        set_error.set(None);

        // Get schemas
        let in_schema = input_schema.get();
        let input_schema_opt = if in_schema.get("properties")
            .and_then(|p| p.as_object())
            .map(|o| o.is_empty())
            .unwrap_or(true)
        {
            None
        } else {
            Some(in_schema)
        };

        let out_schema = output_schema.get();
        let output_schema_opt = if out_schema.get("properties")
            .and_then(|p| p.as_object())
            .map(|o| o.is_empty())
            .unwrap_or(true)
        {
            None
        } else {
            Some(out_schema)
        };

        let template = ResourceTemplate {
            name: name.get(),
            uri_template: uri_template.get(),
            description: if description.get().is_empty() { None } else { Some(description.get()) },
            tags: tags.get(),
            mime_type: if mime_type.get().is_empty() { None } else { Some(mime_type.get()) },
            input_schema: input_schema_opt,
            output_schema: output_schema_opt,
            content: if content.get().is_empty() { None } else { Some(content.get()) },
            mock: build_mock_config(),
        };

        wasm_bindgen_futures::spawn_local(async move {
            match api::create_resource_template(&template).await {
                Ok(_) => {
                    if let Some(window) = web_sys::window() {
                        let _ = window.location().set_href("/resource-templates");
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
                <a href="/resource-templates" class="text-purple-500 hover:underline flex items-center gap-1">
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7"/>
                    </svg>
                    "Back to Resource Templates"
                </a>
            </div>

            <h2 class="text-2xl font-bold mb-6">"New Resource Template"</h2>

            <form on:submit=on_submit class="bg-white rounded-lg shadow p-6 max-w-3xl">
                {move || error.get().map(|e| view! {
                    <div class="mb-4 p-3 bg-red-100 border border-red-400 text-red-700 rounded">
                        {e}
                    </div>
                })}

                <ResourceTemplateFormFields
                    name=name set_name=set_name
                    uri_template=uri_template set_uri_template=set_uri_template
                    description=description set_description=set_description
                    mime_type=mime_type set_mime_type=set_mime_type
                    content=content set_content=set_content
                    input_schema=input_schema
                    set_input_schema=set_input_schema
                    output_schema=output_schema
                    set_output_schema=set_output_schema
                    mock_strategy=mock_strategy set_mock_strategy=set_mock_strategy
                    mock_template=mock_template set_mock_template=set_mock_template
                    mock_faker_type=mock_faker_type set_mock_faker_type=set_mock_faker_type
                    mock_state_key=mock_state_key set_mock_state_key=set_mock_state_key
                    mock_state_operation=mock_state_operation set_mock_state_operation=set_mock_state_operation
                    mock_script=mock_script set_mock_script=set_mock_script
                    mock_script_lang=mock_script_lang set_mock_script_lang=set_mock_script_lang
                    mock_file_path=mock_file_path set_mock_file_path=set_mock_file_path
                    mock_file_selection=mock_file_selection set_mock_file_selection=set_mock_file_selection
                    mock_pattern=mock_pattern set_mock_pattern=set_mock_pattern
                    mock_llm_provider=mock_llm_provider set_mock_llm_provider=set_mock_llm_provider
                    mock_llm_model=mock_llm_model set_mock_llm_model=set_mock_llm_model
                    mock_llm_api_key_env=mock_llm_api_key_env set_mock_llm_api_key_env=set_mock_llm_api_key_env
                    mock_llm_system_prompt=mock_llm_system_prompt set_mock_llm_system_prompt=set_mock_llm_system_prompt
                    mock_db_url=mock_db_url set_mock_db_url=set_mock_db_url
                    mock_db_query=mock_db_query set_mock_db_query=set_mock_db_query
                />

                // Tags - directly in the form to avoid signal propagation issues
                <div class="mt-4">
                    <TagInput tags=tags />
                </div>

                <div class="mt-6 flex gap-3">
                    <button
                        type="submit"
                        disabled=move || saving.get()
                        class="px-4 py-2 bg-purple-500 text-white rounded hover:bg-purple-600 disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                        {move || if saving.get() { "Creating..." } else { "Create Template" }}
                    </button>
                    <a
                        href="/resource-templates"
                        class="px-4 py-2 border border-gray-300 text-gray-700 rounded hover:bg-gray-50"
                    >
                        "Cancel"
                    </a>
                </div>
            </form>
        </div>
    }
}

#[allow(clippy::too_many_arguments)]
#[component]
fn ResourceTemplateFormFields(
    name: ReadSignal<String>,
    set_name: WriteSignal<String>,
    uri_template: ReadSignal<String>,
    set_uri_template: WriteSignal<String>,
    description: ReadSignal<String>,
    set_description: WriteSignal<String>,
    mime_type: ReadSignal<String>,
    set_mime_type: WriteSignal<String>,
    content: ReadSignal<String>,
    set_content: WriteSignal<String>,
    input_schema: ReadSignal<serde_json::Value>,
    set_input_schema: WriteSignal<serde_json::Value>,
    output_schema: ReadSignal<serde_json::Value>,
    set_output_schema: WriteSignal<serde_json::Value>,
    mock_strategy: ReadSignal<String>,
    set_mock_strategy: WriteSignal<String>,
    mock_template: ReadSignal<String>,
    set_mock_template: WriteSignal<String>,
    mock_faker_type: ReadSignal<String>,
    set_mock_faker_type: WriteSignal<String>,
    mock_state_key: ReadSignal<String>,
    set_mock_state_key: WriteSignal<String>,
    mock_state_operation: ReadSignal<String>,
    set_mock_state_operation: WriteSignal<String>,
    mock_script: ReadSignal<String>,
    set_mock_script: WriteSignal<String>,
    mock_script_lang: ReadSignal<String>,
    set_mock_script_lang: WriteSignal<String>,
    mock_file_path: ReadSignal<String>,
    set_mock_file_path: WriteSignal<String>,
    mock_file_selection: ReadSignal<String>,
    set_mock_file_selection: WriteSignal<String>,
    mock_pattern: ReadSignal<String>,
    set_mock_pattern: WriteSignal<String>,
    mock_llm_provider: ReadSignal<String>,
    set_mock_llm_provider: WriteSignal<String>,
    mock_llm_model: ReadSignal<String>,
    set_mock_llm_model: WriteSignal<String>,
    mock_llm_api_key_env: ReadSignal<String>,
    set_mock_llm_api_key_env: WriteSignal<String>,
    mock_llm_system_prompt: ReadSignal<String>,
    set_mock_llm_system_prompt: WriteSignal<String>,
    mock_db_url: ReadSignal<String>,
    set_mock_db_url: WriteSignal<String>,
    mock_db_query: ReadSignal<String>,
    set_mock_db_query: WriteSignal<String>,
) -> impl IntoView {
    view! {
        <div class="space-y-6">
            // Basic Info Section
            <div class="border-b pb-4">
                <h3 class="text-lg font-semibold text-gray-800 mb-4">"Basic Information"</h3>
                <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">"Name *"</label>
                        <input
                            type="text"
                            required=true
                            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
                            placeholder="user-lookup"
                            prop:value=move || name.get()
                            on:input=move |ev| {
                                let target = ev.target().unwrap();
                                let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                set_name.set(input.value());
                            }
                        />
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">"URI Template *"</label>
                        <input
                            type="text"
                            required=true
                            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500 font-mono"
                            placeholder="postgres://db/users/{id}"
                            prop:value=move || uri_template.get()
                            on:input=move |ev| {
                                let target = ev.target().unwrap();
                                let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                set_uri_template.set(input.value());
                            }
                        />
                        <p class="mt-1 text-xs text-gray-500">"Use {placeholder} for variables, e.g., postgres://db/users/{id}"</p>
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">"Description"</label>
                        <input
                            type="text"
                            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
                            placeholder="Fetch user by ID"
                            prop:value=move || description.get()
                            on:input=move |ev| {
                                let target = ev.target().unwrap();
                                let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                set_description.set(input.value());
                            }
                        />
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">"MIME Type"</label>
                        <input
                            type="text"
                            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
                            placeholder="application/json"
                            prop:value=move || mime_type.get()
                            on:input=move |ev| {
                                let target = ev.target().unwrap();
                                let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                set_mime_type.set(input.value());
                            }
                        />
                    </div>
                </div>
            </div>

            // Schema Section (templates have both input and output schemas)
            <div class="border-b pb-4">
                <h3 class="text-lg font-semibold text-gray-800 mb-4">"Input Variables & Output Schema"</h3>
                <div class="space-y-4">
                    <div class="bg-purple-50 border border-purple-200 rounded-lg p-4">
                        <h4 class="font-medium text-purple-800 mb-2">"Input Variables (URI Template Parameters)"</h4>
                        <p class="text-sm text-purple-600 mb-4">"Define the schema for the {placeholder} variables in your URI template."</p>
                        <FullSchemaEditor
                            label="Input Parameters"
                            color="purple"
                            schema=input_schema
                            set_schema=set_input_schema
                            show_definitions=true
                        />
                    </div>
                    <div>
                        <FullSchemaEditor
                            label="Output Schema"
                            color="green"
                            schema=output_schema
                            set_schema=set_output_schema
                            show_definitions=true
                        />
                    </div>
                </div>
            </div>

            // Mock Strategy Section
            <div class="border-b pb-4">
                <h3 class="text-lg font-semibold text-gray-800 mb-4">"Response Strategy"</h3>

                <div class="mb-4">
                    <label class="block text-sm font-medium text-gray-700 mb-1">"Strategy Type"</label>
                    <select
                        class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
                        prop:value=move || mock_strategy.get()
                        on:change=move |ev| {
                            let target = ev.target().unwrap();
                            let select: web_sys::HtmlSelectElement = target.dyn_into().unwrap();
                            set_mock_strategy.set(select.value());
                        }
                    >
                        <option value="none">"No Mock (Use Static Content)"</option>
                        <option value="static">"Static"</option>
                        <option value="template">"Template (Handlebars)"</option>
                        <option value="random">"Random (Faker)"</option>
                        <option value="stateful">"Stateful"</option>
                        <option value="script">"Script"</option>
                        <option value="file">"File"</option>
                        <option value="pattern">"Pattern (Regex)"</option>
                        <option value="llm">"LLM (AI Generated)"</option>
                        <option value="database">"Database Query"</option>
                    </select>
                    <p class="mt-1 text-xs text-gray-500">"Choose how the template content should be generated. Use Template strategy to access input variables via {{variable_name}}."</p>
                </div>

                // Strategy-specific fields
                <MockStrategyFields
                    strategy=mock_strategy
                    content=content set_content=set_content
                    template=mock_template set_template=set_mock_template
                    faker_type=mock_faker_type set_faker_type=set_mock_faker_type
                    state_key=mock_state_key set_state_key=set_mock_state_key
                    state_operation=mock_state_operation set_state_operation=set_mock_state_operation
                    script=mock_script set_script=set_mock_script
                    script_lang=mock_script_lang set_script_lang=set_mock_script_lang
                    file_path=mock_file_path set_file_path=set_mock_file_path
                    file_selection=mock_file_selection set_file_selection=set_mock_file_selection
                    pattern=mock_pattern set_pattern=set_mock_pattern
                    llm_provider=mock_llm_provider set_llm_provider=set_mock_llm_provider
                    llm_model=mock_llm_model set_llm_model=set_mock_llm_model
                    llm_api_key_env=mock_llm_api_key_env set_llm_api_key_env=set_mock_llm_api_key_env
                    llm_system_prompt=mock_llm_system_prompt set_llm_system_prompt=set_mock_llm_system_prompt
                    db_url=mock_db_url set_db_url=set_mock_db_url
                    db_query=mock_db_query set_db_query=set_mock_db_query
                />
            </div>
        </div>
    }
}

// Reuse the MockStrategyFields component from resources.rs
// For brevity, we'll use a simplified version here
#[allow(clippy::too_many_arguments)]
#[component]
fn MockStrategyFields(
    strategy: ReadSignal<String>,
    content: ReadSignal<String>,
    set_content: WriteSignal<String>,
    template: ReadSignal<String>,
    set_template: WriteSignal<String>,
    faker_type: ReadSignal<String>,
    set_faker_type: WriteSignal<String>,
    state_key: ReadSignal<String>,
    set_state_key: WriteSignal<String>,
    state_operation: ReadSignal<String>,
    set_state_operation: WriteSignal<String>,
    script: ReadSignal<String>,
    set_script: WriteSignal<String>,
    script_lang: ReadSignal<String>,
    set_script_lang: WriteSignal<String>,
    file_path: ReadSignal<String>,
    set_file_path: WriteSignal<String>,
    file_selection: ReadSignal<String>,
    set_file_selection: WriteSignal<String>,
    pattern: ReadSignal<String>,
    set_pattern: WriteSignal<String>,
    llm_provider: ReadSignal<String>,
    set_llm_provider: WriteSignal<String>,
    llm_model: ReadSignal<String>,
    set_llm_model: WriteSignal<String>,
    llm_api_key_env: ReadSignal<String>,
    set_llm_api_key_env: WriteSignal<String>,
    llm_system_prompt: ReadSignal<String>,
    set_llm_system_prompt: WriteSignal<String>,
    db_url: ReadSignal<String>,
    set_db_url: WriteSignal<String>,
    db_query: ReadSignal<String>,
    set_db_query: WriteSignal<String>,
) -> impl IntoView {
    view! {
        {move || {
            let strat = strategy.get();
            match strat.as_str() {
                "none" | "static" => view! {
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">"Static Content"</label>
                        <textarea
                            rows=6
                            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500 font-mono text-sm"
                            placeholder="Static content (use {variable} syntax for URI template variable substitution)"
                            prop:value=move || content.get()
                            on:input=move |ev| {
                                let target = ev.target().unwrap();
                                let textarea: web_sys::HtmlTextAreaElement = target.dyn_into().unwrap();
                                set_content.set(textarea.value());
                            }
                        />
                    </div>
                }.into_any(),
                "template" => view! {
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">"Handlebars Template *"</label>
                        <textarea
                            rows=6
                            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500 font-mono text-sm"
                            placeholder=r#"{"id": {{id}}, "name": "User {{id}}", "email": "user{{id}}@example.com"}"#
                            prop:value=move || template.get()
                            on:input=move |ev| {
                                let target = ev.target().unwrap();
                                let textarea: web_sys::HtmlTextAreaElement = target.dyn_into().unwrap();
                                set_template.set(textarea.value());
                            }
                        />
                        <p class="mt-1 text-xs text-gray-500">"Access input variables via {{variable_name}}. Helpers: now, uuid, random_int, random_float, random_bool, random_string, json_encode"</p>
                    </div>
                }.into_any(),
                "random" => view! {
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">"Faker Type"</label>
                        <select
                            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
                            prop:value=move || faker_type.get()
                            on:change=move |ev| {
                                let target = ev.target().unwrap();
                                let select: web_sys::HtmlSelectElement = target.dyn_into().unwrap();
                                set_faker_type.set(select.value());
                            }
                        >
                            <option value="">"Default (lorem ipsum)"</option>
                            <option value="name">"Name"</option>
                            <option value="email">"Email"</option>
                            <option value="phone">"Phone"</option>
                            <option value="address">"Address"</option>
                            <option value="company">"Company"</option>
                            <option value="uuid">"UUID"</option>
                            <option value="sentence">"Sentence"</option>
                            <option value="paragraph">"Paragraph"</option>
                        </select>
                    </div>
                }.into_any(),
                "stateful" => view! {
                    <div class="space-y-4">
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"State Key *"</label>
                            <input
                                type="text"
                                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
                                placeholder="user-{id}-counter"
                                prop:value=move || state_key.get()
                                on:input=move |ev| {
                                    let target = ev.target().unwrap();
                                    let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                    set_state_key.set(input.value());
                                }
                            />
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"Operation"</label>
                            <select
                                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
                                prop:value=move || state_operation.get()
                                on:change=move |ev| {
                                    let target = ev.target().unwrap();
                                    let select: web_sys::HtmlSelectElement = target.dyn_into().unwrap();
                                    set_state_operation.set(select.value());
                                }
                            >
                                <option value="get">"Get"</option>
                                <option value="set">"Set"</option>
                                <option value="increment">"Increment"</option>
                            </select>
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"Response Template (optional)"</label>
                            <textarea
                                rows=3
                                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500 font-mono text-sm"
                                placeholder=r#"{"counter": {{state}}, "user_id": {{id}}}"#
                                prop:value=move || template.get()
                                on:input=move |ev| {
                                    let target = ev.target().unwrap();
                                    let textarea: web_sys::HtmlTextAreaElement = target.dyn_into().unwrap();
                                    set_template.set(textarea.value());
                                }
                            />
                        </div>
                    </div>
                }.into_any(),
                "script" => view! {
                    <div class="space-y-4">
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"Script Language"</label>
                            <select
                                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
                                prop:value=move || script_lang.get()
                                on:change=move |ev| {
                                    let target = ev.target().unwrap();
                                    let select: web_sys::HtmlSelectElement = target.dyn_into().unwrap();
                                    set_script_lang.set(select.value());
                                }
                            >
                                <option value="rhai">"Rhai"</option>
                                <option value="lua">"Lua"</option>
                                <option value="js">"JavaScript"</option>
                                <option value="python">"Python"</option>
                            </select>
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"Script *"</label>
                            <textarea
                                rows=8
                                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500 font-mono text-sm"
                                placeholder="// Access input variables via args.variable_name"
                                prop:value=move || script.get()
                                on:input=move |ev| {
                                    let target = ev.target().unwrap();
                                    let textarea: web_sys::HtmlTextAreaElement = target.dyn_into().unwrap();
                                    set_script.set(textarea.value());
                                }
                            />
                        </div>
                    </div>
                }.into_any(),
                "file" => view! {
                    <div class="space-y-4">
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"File Path *"</label>
                            <input
                                type="text"
                                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
                                placeholder="/path/to/responses.json"
                                prop:value=move || file_path.get()
                                on:input=move |ev| {
                                    let target = ev.target().unwrap();
                                    let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                    set_file_path.set(input.value());
                                }
                            />
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"Selection Mode"</label>
                            <select
                                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
                                prop:value=move || file_selection.get()
                                on:change=move |ev| {
                                    let target = ev.target().unwrap();
                                    let select: web_sys::HtmlSelectElement = target.dyn_into().unwrap();
                                    set_file_selection.set(select.value());
                                }
                            >
                                <option value="random">"Random"</option>
                                <option value="sequential">"Sequential"</option>
                                <option value="first">"First"</option>
                                <option value="last">"Last"</option>
                            </select>
                        </div>
                    </div>
                }.into_any(),
                "pattern" => view! {
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">"Pattern *"</label>
                        <input
                            type="text"
                            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500 font-mono"
                            placeholder="[A-Z]{3}-[0-9]{4}"
                            prop:value=move || pattern.get()
                            on:input=move |ev| {
                                let target = ev.target().unwrap();
                                let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                set_pattern.set(input.value());
                            }
                        />
                    </div>
                }.into_any(),
                "llm" => view! {
                    <div class="space-y-4">
                        <div class="grid grid-cols-2 gap-4">
                            <div>
                                <label class="block text-sm font-medium text-gray-700 mb-1">"Provider"</label>
                                <select
                                    class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
                                    prop:value=move || llm_provider.get()
                                    on:change=move |ev| {
                                        let target = ev.target().unwrap();
                                        let select: web_sys::HtmlSelectElement = target.dyn_into().unwrap();
                                        set_llm_provider.set(select.value());
                                    }
                                >
                                    <option value="openai">"OpenAI"</option>
                                    <option value="anthropic">"Anthropic"</option>
                                </select>
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-gray-700 mb-1">"Model *"</label>
                                <input
                                    type="text"
                                    class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
                                    placeholder="gpt-4"
                                    prop:value=move || llm_model.get()
                                    on:input=move |ev| {
                                        let target = ev.target().unwrap();
                                        let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                        set_llm_model.set(input.value());
                                    }
                                />
                            </div>
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"API Key Env Var"</label>
                            <input
                                type="text"
                                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
                                placeholder="OPENAI_API_KEY"
                                prop:value=move || llm_api_key_env.get()
                                on:input=move |ev| {
                                    let target = ev.target().unwrap();
                                    let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                    set_llm_api_key_env.set(input.value());
                                }
                            />
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"System Prompt"</label>
                            <textarea
                                rows=4
                                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
                                placeholder="You are a helpful assistant..."
                                prop:value=move || llm_system_prompt.get()
                                on:input=move |ev| {
                                    let target = ev.target().unwrap();
                                    let textarea: web_sys::HtmlTextAreaElement = target.dyn_into().unwrap();
                                    set_llm_system_prompt.set(textarea.value());
                                }
                            />
                        </div>
                    </div>
                }.into_any(),
                "database" => view! {
                    <div class="space-y-4">
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"Database URL *"</label>
                            <input
                                type="text"
                                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500 font-mono"
                                placeholder="postgres://user:pass@host/db"
                                prop:value=move || db_url.get()
                                on:input=move |ev| {
                                    let target = ev.target().unwrap();
                                    let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                    set_db_url.set(input.value());
                                }
                            />
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"SQL Query *"</label>
                            <textarea
                                rows=4
                                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500 font-mono text-sm"
                                placeholder="SELECT * FROM users WHERE id = $1"
                                prop:value=move || db_query.get()
                                on:input=move |ev| {
                                    let target = ev.target().unwrap();
                                    let textarea: web_sys::HtmlTextAreaElement = target.dyn_into().unwrap();
                                    set_db_query.set(textarea.value());
                                }
                            />
                        </div>
                    </div>
                }.into_any(),
                _ => view! { <div></div> }.into_any(),
            }
        }}
    }
}

#[component]
pub fn ResourceTemplateEditForm() -> impl IntoView {
    let params = use_params_map();
    let uri_template_param = move || {
        params.read().get("uri_template").unwrap_or_default()
    };

    let (name, set_name) = signal(String::new());
    let (uri_template, set_uri_template) = signal(String::new());
    let (description, set_description) = signal(String::new());
    let (mime_type, set_mime_type) = signal(String::new());
    let (content, set_content) = signal(String::new());
    let (error, set_error) = signal(Option::<String>::None);
    let (saving, set_saving) = signal(false);
    let (loading, set_loading) = signal(true);
    let (original_uri_template, set_original_uri_template) = signal(String::new());
    let tags = RwSignal::new(Vec::new());
    let (has_loaded, set_has_loaded) = signal(false);

    // Schema signals - using Value for full JSON schema support
    let default_schema_edit = serde_json::json!({
        "type": "object",
        "properties": {}
    });
    let (input_schema, set_input_schema) = signal(default_schema_edit.clone());
    let (output_schema, set_output_schema) = signal(default_schema_edit);

    // Mock strategy signals
    let (mock_strategy, set_mock_strategy) = signal("none".to_string());
    let (mock_template, set_mock_template) = signal(String::new());
    let (mock_faker_type, set_mock_faker_type) = signal(String::new());
    let (mock_state_key, set_mock_state_key) = signal(String::new());
    let (mock_state_operation, set_mock_state_operation) = signal("get".to_string());
    let (mock_script, set_mock_script) = signal(String::new());
    let (mock_script_lang, set_mock_script_lang) = signal("rhai".to_string());
    let (mock_file_path, set_mock_file_path) = signal(String::new());
    let (mock_file_selection, set_mock_file_selection) = signal("random".to_string());
    let (mock_pattern, set_mock_pattern) = signal(String::new());
    let (mock_llm_provider, set_mock_llm_provider) = signal("openai".to_string());
    let (mock_llm_model, set_mock_llm_model) = signal(String::new());
    let (mock_llm_api_key_env, set_mock_llm_api_key_env) = signal(String::new());
    let (mock_llm_system_prompt, set_mock_llm_system_prompt) = signal(String::new());
    let (mock_db_url, set_mock_db_url) = signal(String::new());
    let (mock_db_query, set_mock_db_query) = signal(String::new());

    // Load existing template (only once)
    Effect::new(move |_| {
        let param = uri_template_param();
        if param.is_empty() {
            return;
        }
        // Skip if already loaded to prevent overwriting user changes
        if has_loaded.get() {
            return;
        }
        let decoded = urlencoding::decode(&param);
        set_loading.set(true);
        wasm_bindgen_futures::spawn_local(async move {
            match api::get_resource_template(&decoded).await {
                Ok(template) => {
                    set_original_uri_template.set(template.uri_template.clone());
                    set_name.set(template.name.clone());
                    set_uri_template.set(template.uri_template.clone());
                    set_description.set(template.description.clone().unwrap_or_default());
                    set_mime_type.set(template.mime_type.clone().unwrap_or_default());
                    set_content.set(template.content.clone().unwrap_or_default());
                    tags.set(template.tags.clone());

                    // Load schemas directly
                    if let Some(in_schema) = &template.input_schema {
                        set_input_schema.set(in_schema.clone());
                    }
                    if let Some(out_schema) = &template.output_schema {
                        set_output_schema.set(out_schema.clone());
                    }

                    // Load mock config
                    if let Some(mock) = &template.mock {
                        let strategy = match mock.strategy {
                            MockStrategyType::Static => "static",
                            MockStrategyType::Template => "template",
                            MockStrategyType::Random => "random",
                            MockStrategyType::Stateful => "stateful",
                            MockStrategyType::Script => "script",
                            MockStrategyType::File => "file",
                            MockStrategyType::Pattern => "pattern",
                            MockStrategyType::LLM => "llm",
                            MockStrategyType::Database => "database",
                        };
                        set_mock_strategy.set(strategy.to_string());

                        if let Some(t) = &mock.template {
                            set_mock_template.set(t.clone());
                        }
                        if let Some(ft) = &mock.faker_type {
                            set_mock_faker_type.set(ft.clone());
                        }
                        if let Some(s) = &mock.stateful {
                            set_mock_state_key.set(s.state_key.clone());
                            let op = match s.operation {
                                StateOperation::Get => "get",
                                StateOperation::Set => "set",
                                StateOperation::Increment => "increment",
                            };
                            set_mock_state_operation.set(op.to_string());
                            if let Some(t) = &s.template {
                                set_mock_template.set(t.clone());
                            }
                        }
                        if let Some(s) = &mock.script {
                            set_mock_script.set(s.clone());
                        }
                        if let Some(l) = &mock.script_lang {
                            let lang = match l {
                                ScriptLang::Rhai => "rhai",
                                ScriptLang::Lua => "lua",
                                ScriptLang::Js => "js",
                                ScriptLang::Python => "python",
                            };
                            set_mock_script_lang.set(lang.to_string());
                        }
                        if let Some(f) = &mock.file {
                            set_mock_file_path.set(f.path.clone());
                            set_mock_file_selection.set(f.selection.clone());
                        }
                        if let Some(p) = &mock.pattern {
                            set_mock_pattern.set(p.clone());
                        }
                        if let Some(llm) = &mock.llm {
                            let provider = match llm.provider {
                                LLMProvider::OpenAI => "openai",
                                LLMProvider::Anthropic => "anthropic",
                            };
                            set_mock_llm_provider.set(provider.to_string());
                            set_mock_llm_model.set(llm.model.clone());
                            if let Some(env) = &llm.api_key_env {
                                set_mock_llm_api_key_env.set(env.clone());
                            }
                            if let Some(p) = &llm.system_prompt {
                                set_mock_llm_system_prompt.set(p.clone());
                            }
                        }
                        if let Some(db) = &mock.database {
                            set_mock_db_url.set(db.url.clone());
                            set_mock_db_query.set(db.query.clone());
                        }
                    }
                    set_has_loaded.set(true);
                }
                Err(e) => {
                    set_error.set(Some(format!("Failed to load template: {}", e)));
                }
            }
            set_loading.set(false);
        });
    });

    let build_mock_config = move || -> Option<MockConfig> {
        let strategy = mock_strategy.get();
        if strategy == "none" {
            return None;
        }

        let strategy_type = match strategy.as_str() {
            "static" => MockStrategyType::Static,
            "template" => MockStrategyType::Template,
            "random" => MockStrategyType::Random,
            "stateful" => MockStrategyType::Stateful,
            "script" => MockStrategyType::Script,
            "file" => MockStrategyType::File,
            "pattern" => MockStrategyType::Pattern,
            "llm" => MockStrategyType::LLM,
            "database" => MockStrategyType::Database,
            _ => return None,
        };

        let mut config = MockConfig {
            strategy: strategy_type,
            ..Default::default()
        };

        match strategy.as_str() {
            "template" => {
                config.template = Some(mock_template.get());
            }
            "random" => {
                if !mock_faker_type.get().is_empty() {
                    config.faker_type = Some(mock_faker_type.get());
                }
            }
            "stateful" => {
                config.stateful = Some(StatefulConfig {
                    state_key: mock_state_key.get(),
                    operation: match mock_state_operation.get().as_str() {
                        "set" => StateOperation::Set,
                        "increment" => StateOperation::Increment,
                        _ => StateOperation::Get,
                    },
                    template: if mock_template.get().is_empty() { None } else { Some(mock_template.get()) },
                });
            }
            "script" => {
                config.script = Some(mock_script.get());
                config.script_lang = Some(match mock_script_lang.get().as_str() {
                    "lua" => ScriptLang::Lua,
                    "js" => ScriptLang::Js,
                    "python" => ScriptLang::Python,
                    _ => ScriptLang::Rhai,
                });
            }
            "file" => {
                config.file = Some(FileConfig {
                    path: mock_file_path.get(),
                    selection: mock_file_selection.get(),
                });
            }
            "pattern" => {
                config.pattern = Some(mock_pattern.get());
            }
            "llm" => {
                config.llm = Some(LLMConfig {
                    provider: match mock_llm_provider.get().as_str() {
                        "anthropic" => LLMProvider::Anthropic,
                        _ => LLMProvider::OpenAI,
                    },
                    api_key_env: if mock_llm_api_key_env.get().is_empty() { None } else { Some(mock_llm_api_key_env.get()) },
                    model: mock_llm_model.get(),
                    system_prompt: if mock_llm_system_prompt.get().is_empty() { None } else { Some(mock_llm_system_prompt.get()) },
                    temperature: None,
                    max_tokens: None,
                    stream: false,
                });
            }
            "database" => {
                config.database = Some(MockDatabaseConfig {
                    url: mock_db_url.get(),
                    query: mock_db_query.get(),
                    params: vec![],
                });
            }
            _ => {}
        }

        Some(config)
    };

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        set_saving.set(true);
        set_error.set(None);

        let orig = original_uri_template.get();

        // Get schemas
        let in_schema = input_schema.get();
        let input_schema_opt = if in_schema.get("properties")
            .and_then(|p| p.as_object())
            .map(|o| o.is_empty())
            .unwrap_or(true)
        {
            None
        } else {
            Some(in_schema)
        };

        let out_schema = output_schema.get();
        let output_schema_opt = if out_schema.get("properties")
            .and_then(|p| p.as_object())
            .map(|o| o.is_empty())
            .unwrap_or(true)
        {
            None
        } else {
            Some(out_schema)
        };

        let template = ResourceTemplate {
            name: name.get(),
            uri_template: uri_template.get(),
            description: if description.get().is_empty() { None } else { Some(description.get()) },
            tags: tags.get(),
            mime_type: if mime_type.get().is_empty() { None } else { Some(mime_type.get()) },
            input_schema: input_schema_opt,
            output_schema: output_schema_opt,
            content: if content.get().is_empty() { None } else { Some(content.get()) },
            mock: build_mock_config(),
        };

        wasm_bindgen_futures::spawn_local(async move {
            match api::update_resource_template(&orig, &template).await {
                Ok(_) => {
                    if let Some(window) = web_sys::window() {
                        let _ = window.location().set_href("/resource-templates");
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
                <a href="/resource-templates" class="text-purple-500 hover:underline flex items-center gap-1">
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7"/>
                    </svg>
                    "Back to Resource Templates"
                </a>
            </div>

            <h2 class="text-2xl font-bold mb-6">"Edit Resource Template"</h2>

            // Loading spinner
            <div
                class="flex items-center justify-center py-12"
                style=move || if loading.get() { "display: flex" } else { "display: none" }
            >
                <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-purple-500"></div>
                <span class="ml-3 text-gray-500">"Loading template..."</span>
            </div>

            // Form - always rendered but hidden while loading
            <form
                on:submit=on_submit
                class="bg-white rounded-lg shadow p-6 max-w-3xl"
                style=move || if loading.get() { "display: none" } else { "display: block" }
            >
                {move || error.get().map(|e| view! {
                    <div class="mb-4 p-3 bg-red-100 border border-red-400 text-red-700 rounded">
                        {e}
                    </div>
                })}

                <ResourceTemplateFormFields
                    name=name set_name=set_name
                    uri_template=uri_template set_uri_template=set_uri_template
                    description=description set_description=set_description
                    mime_type=mime_type set_mime_type=set_mime_type
                    content=content set_content=set_content
                    input_schema=input_schema
                    set_input_schema=set_input_schema
                    output_schema=output_schema
                    set_output_schema=set_output_schema
                    mock_strategy=mock_strategy set_mock_strategy=set_mock_strategy
                    mock_template=mock_template set_mock_template=set_mock_template
                    mock_faker_type=mock_faker_type set_mock_faker_type=set_mock_faker_type
                    mock_state_key=mock_state_key set_mock_state_key=set_mock_state_key
                    mock_state_operation=mock_state_operation set_mock_state_operation=set_mock_state_operation
                    mock_script=mock_script set_mock_script=set_mock_script
                    mock_script_lang=mock_script_lang set_mock_script_lang=set_mock_script_lang
                    mock_file_path=mock_file_path set_mock_file_path=set_mock_file_path
                    mock_file_selection=mock_file_selection set_mock_file_selection=set_mock_file_selection
                    mock_pattern=mock_pattern set_mock_pattern=set_mock_pattern
                    mock_llm_provider=mock_llm_provider set_mock_llm_provider=set_mock_llm_provider
                    mock_llm_model=mock_llm_model set_mock_llm_model=set_mock_llm_model
                    mock_llm_api_key_env=mock_llm_api_key_env set_mock_llm_api_key_env=set_mock_llm_api_key_env
                    mock_llm_system_prompt=mock_llm_system_prompt set_mock_llm_system_prompt=set_mock_llm_system_prompt
                    mock_db_url=mock_db_url set_mock_db_url=set_mock_db_url
                    mock_db_query=mock_db_query set_mock_db_query=set_mock_db_query
                />

                // Tags - directly in the form to avoid signal propagation issues
                <div class="mt-4">
                    <TagInput tags=tags />
                </div>

                <div class="mt-6 flex gap-3">
                    <button
                        type="submit"
                        disabled=move || saving.get()
                        class="px-4 py-2 bg-purple-500 text-white rounded hover:bg-purple-600 disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                        {move || if saving.get() { "Saving..." } else { "Save Changes" }}
                    </button>
                    <a
                        href="/resource-templates"
                        class="px-4 py-2 border border-gray-300 text-gray-700 rounded hover:bg-gray-50"
                    >
                        "Cancel"
                    </a>
                </div>
            </form>
        </div>
    }
}
