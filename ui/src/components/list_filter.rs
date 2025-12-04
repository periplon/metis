//! Reusable list filtering, search, sorting and pagination components.
//!
//! This module provides components for:
//! - Search bar with text filtering
//! - Tag-based filtering with multi-select
//! - Sorting by various fields
//! - Pagination controls
//!
//! These components work together to provide a consistent filtering
//! experience across all archetype list views.

use leptos::prelude::*;
use std::collections::HashSet;

/// Sort field options
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum SortField {
    #[default]
    Name,
    Description,
    DateCreated,
    DateModified,
}

impl SortField {
    pub fn from_str(s: &str) -> Self {
        match s {
            "description" => SortField::Description,
            "date_created" => SortField::DateCreated,
            "date_modified" => SortField::DateModified,
            _ => SortField::Name,
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            SortField::Name => "name",
            SortField::Description => "description",
            SortField::DateCreated => "date_created",
            SortField::DateModified => "date_modified",
        }
    }
}

/// Sort order
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum SortOrder {
    #[default]
    Ascending,
    Descending,
}

impl SortOrder {
    pub fn toggle(&self) -> Self {
        match self {
            SortOrder::Ascending => SortOrder::Descending,
            SortOrder::Descending => SortOrder::Ascending,
        }
    }
}

/// Configuration for the list filter bar
#[allow(dead_code)]
#[derive(Clone, Default)]
pub struct ListFilterConfig {
    /// Placeholder text for the search input
    pub search_placeholder: String,
    /// Available tags for filtering (extracted from items)
    pub available_tags: Vec<String>,
    /// Whether to show the search input
    pub show_search: bool,
    /// Whether to show tag filters
    pub show_tags: bool,
    /// Whether to show pagination
    pub show_pagination: bool,
    /// Items per page for pagination
    pub items_per_page: usize,
}

#[allow(dead_code)]
impl ListFilterConfig {
    pub fn new() -> Self {
        Self {
            search_placeholder: "Search...".to_string(),
            available_tags: Vec::new(),
            show_search: true,
            show_tags: true,
            show_pagination: true,
            items_per_page: 10,
        }
    }

    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.search_placeholder = placeholder.into();
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.available_tags = tags;
        self
    }

    pub fn with_items_per_page(mut self, count: usize) -> Self {
        self.items_per_page = count;
        self
    }
}

/// State for list filtering
#[allow(dead_code)]
#[derive(Clone, Default)]
pub struct ListFilterState {
    /// Current search query
    pub search_query: String,
    /// Currently selected tags
    pub selected_tags: HashSet<String>,
    /// Current page (0-indexed)
    pub current_page: usize,
}

/// Extract unique tags from a list of items
pub fn extract_tags<T, F>(items: &[T], get_tags: F) -> Vec<String>
where
    F: Fn(&T) -> &[String],
{
    let mut tags: HashSet<String> = HashSet::new();
    for item in items {
        for tag in get_tags(item) {
            tags.insert(tag.clone());
        }
    }
    let mut sorted: Vec<_> = tags.into_iter().collect();
    sorted.sort();
    sorted
}

/// Filter items by search query and tags
pub fn filter_items<T, F, G>(
    items: &[T],
    search_query: &str,
    selected_tags: &HashSet<String>,
    get_searchable: F,
    get_tags: G,
) -> Vec<T>
where
    T: Clone,
    F: Fn(&T) -> String,
    G: Fn(&T) -> &[String],
{
    items
        .iter()
        .filter(|item| {
            // Search filter
            let search_match = if search_query.is_empty() {
                true
            } else {
                let searchable = get_searchable(item).to_lowercase();
                search_query.to_lowercase().split_whitespace().all(|term| searchable.contains(term))
            };

            // Tag filter - item must have ALL selected tags
            let tag_match = if selected_tags.is_empty() {
                true
            } else {
                let item_tags: HashSet<_> = get_tags(item).iter().cloned().collect();
                selected_tags.iter().all(|tag| item_tags.contains(tag))
            };

            search_match && tag_match
        })
        .cloned()
        .collect()
}

/// Sort items by field and order
pub fn sort_items<T, F>(
    items: &mut [T],
    sort_field: SortField,
    sort_order: SortOrder,
    get_name: F,
) where
    F: Fn(&T) -> &str,
{
    items.sort_by(|a, b| {
        let cmp = match sort_field {
            SortField::Name | SortField::Description => {
                get_name(a).to_lowercase().cmp(&get_name(b).to_lowercase())
            }
            // For now, fallback to name sorting for date fields
            // since most archetypes don't have date fields
            SortField::DateCreated | SortField::DateModified => {
                get_name(a).to_lowercase().cmp(&get_name(b).to_lowercase())
            }
        };
        match sort_order {
            SortOrder::Ascending => cmp,
            SortOrder::Descending => cmp.reverse(),
        }
    });
}

/// Paginate items
pub fn paginate_items<T: Clone>(items: &[T], page: usize, per_page: usize) -> Vec<T> {
    let start = page * per_page;
    let end = (start + per_page).min(items.len());
    if start >= items.len() {
        Vec::new()
    } else {
        items[start..end].to_vec()
    }
}

/// Calculate total pages
pub fn total_pages(total_items: usize, per_page: usize) -> usize {
    if per_page == 0 {
        0
    } else {
        (total_items + per_page - 1) / per_page
    }
}

/// Search and filter bar component
#[component]
pub fn ListFilterBar(
    /// Search query signal
    search_query: RwSignal<String>,
    /// Selected tags signal
    selected_tags: RwSignal<HashSet<String>>,
    /// Available tags for filtering (reactive signal)
    #[prop(into)] available_tags: Signal<Vec<String>>,
    /// Sort field signal (optional)
    #[prop(optional)] sort_field: Option<RwSignal<SortField>>,
    /// Sort order signal (optional)
    #[prop(optional)] sort_order: Option<RwSignal<SortOrder>>,
    /// Placeholder text for search
    #[prop(into, default = "Search...".to_string())]
    placeholder: String,
    /// Whether to show search input
    #[prop(default = true)]
    show_search: bool,
    /// Whether to show tag filters
    #[prop(default = true)]
    show_tags: bool,
) -> impl IntoView {
    let (show_tag_dropdown, set_show_tag_dropdown) = signal(false);
    let show_sort = sort_field.is_some() && sort_order.is_some();

    // Build search input (static, not reactive to avoid focus loss)
    let search_input = if show_search {
        let placeholder_clone = placeholder.clone();
        Some(view! {
            <div class="relative flex-1 max-w-md">
                <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                    <svg class="h-5 w-5 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/>
                    </svg>
                </div>
                <input
                    type="text"
                    class="block w-full pl-10 pr-8 py-2 border border-gray-300 rounded-md leading-5 bg-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-green-500 focus:border-green-500 sm:text-sm"
                    placeholder=placeholder_clone
                    prop:value=move || search_query.get()
                    on:input=move |ev| {
                        let value = event_target_value(&ev);
                        search_query.set(value);
                    }
                />
                // Clear button - only visibility changes, not the DOM structure
                <button
                    class=move || {
                        let base = "absolute inset-y-0 right-0 pr-3 flex items-center text-gray-400 hover:text-gray-600";
                        if search_query.get().is_empty() {
                            format!("{} invisible", base)
                        } else {
                            base.to_string()
                        }
                    }
                    on:click=move |_| search_query.set(String::new())
                >
                    <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                    </svg>
                </button>
            </div>
        })
    } else {
        None
    };

    // Build tag filter dropdown (reactive to available_tags changes)
    let tag_dropdown = if show_tags {
        Some(view! {
            // Only show when there are tags available
            {move || {
                let tags = available_tags.get();
                if tags.is_empty() {
                    None
                } else {
                    Some(view! {
                        <div class="relative">
                            <button
                                class="inline-flex items-center px-3 py-2 border border-gray-300 rounded-md bg-white text-sm font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-green-500"
                                on:click=move |_| set_show_tag_dropdown.update(|v| *v = !*v)
                            >
                                <svg class="h-4 w-4 mr-2 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A1.994 1.994 0 013 12V7a4 4 0 014-4z"/>
                                </svg>
                                "Tags"
                                {move || {
                                    let count = selected_tags.get().len();
                                    (count > 0).then(|| view! {
                                        <span class="ml-2 inline-flex items-center justify-center px-2 py-0.5 text-xs font-bold leading-none text-green-100 bg-green-600 rounded-full">
                                            {count}
                                        </span>
                                    })
                                }}
                                <svg class="ml-2 h-4 w-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"/>
                                </svg>
                            </button>

                            // Dropdown menu
                            {move || show_tag_dropdown.get().then(|| {
                                let dropdown_tags = available_tags.get();
                                view! {
                                    <div class="absolute z-10 mt-1 w-56 bg-white rounded-md shadow-lg ring-1 ring-black ring-opacity-5">
                                        <div class="py-1 max-h-60 overflow-y-auto">
                                            {dropdown_tags.into_iter().map(|tag| {
                                                let tag_for_check = tag.clone();
                                                let tag_for_toggle = tag.clone();
                                                view! {
                                                    <label class="flex items-center px-4 py-2 text-sm text-gray-700 hover:bg-gray-100 cursor-pointer">
                                                        <input
                                                            type="checkbox"
                                                            class="h-4 w-4 text-green-600 focus:ring-green-500 border-gray-300 rounded"
                                                            prop:checked=move || selected_tags.get().contains(&tag_for_check)
                                                            on:change=move |_| {
                                                                selected_tags.update(|tags| {
                                                                    if tags.contains(&tag_for_toggle) {
                                                                        tags.remove(&tag_for_toggle);
                                                                    } else {
                                                                        tags.insert(tag_for_toggle.clone());
                                                                    }
                                                                });
                                                            }
                                                        />
                                                        <span class="ml-2">{tag}</span>
                                                    </label>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                        // Clear all button
                                        {move || (!selected_tags.get().is_empty()).then(|| view! {
                                            <div class="border-t border-gray-100 px-4 py-2">
                                                <button
                                                    class="text-sm text-red-600 hover:text-red-800"
                                                    on:click=move |_| {
                                                        selected_tags.set(HashSet::new());
                                                        set_show_tag_dropdown.set(false);
                                                    }
                                                >
                                                    "Clear all"
                                                </button>
                                            </div>
                                        })}
                                    </div>
                                }
                            })}
                        </div>
                    })
                }
            }}
        })
    } else {
        None
    };

    // Build sort dropdown (if signals are provided)
    let sort_dropdown = if show_sort {
        let sf = sort_field.unwrap();
        let so = sort_order.unwrap();
        Some(view! {
            <div class="flex items-center gap-2">
                <span class="text-sm text-gray-500">"Sort:"</span>
                <select
                    class="px-3 py-2 border border-gray-300 rounded-md bg-white text-sm font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-green-500"
                    prop:value=move || sf.get().to_str()
                    on:change=move |ev| {
                        let value = event_target_value(&ev);
                        sf.set(SortField::from_str(&value));
                    }
                >
                    <option value="name">"Name"</option>
                    <option value="description">"Description"</option>
                </select>
                <button
                    class="inline-flex items-center px-2 py-2 border border-gray-300 rounded-md bg-white text-sm font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-green-500"
                    title=move || if so.get() == SortOrder::Ascending { "Sort Ascending (click to reverse)" } else { "Sort Descending (click to reverse)" }
                    on:click=move |_| so.update(|o| *o = o.toggle())
                >
                    {move || if so.get() == SortOrder::Ascending {
                        view! {
                            <svg class="h-4 w-4 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 4h13M3 8h9m-9 4h6m4 0l4-4m0 0l4 4m-4-4v12"/>
                            </svg>
                        }.into_any()
                    } else {
                        view! {
                            <svg class="h-4 w-4 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 4h13M3 8h9m-9 4h9m5-4v12m0 0l-4-4m4 4l4-4"/>
                            </svg>
                        }.into_any()
                    }}
                </button>
            </div>
        })
    } else {
        None
    };

    view! {
        <div class="flex flex-col sm:flex-row gap-3 mb-4">
            {search_input}
            {tag_dropdown}
            {sort_dropdown}
        </div>

        // Selected tags display
        {move || {
            let tags: Vec<_> = selected_tags.get().iter().cloned().collect();
            (!tags.is_empty()).then(|| view! {
                <div class="flex flex-wrap gap-2 mb-4">
                    {tags.into_iter().map(|tag| {
                        let tag_for_remove = tag.clone();
                        view! {
                            <span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800">
                                {tag}
                                <button
                                    class="ml-1 inline-flex items-center justify-center text-green-400 hover:text-green-600"
                                    on:click=move |_| {
                                        selected_tags.update(|tags| {
                                            tags.remove(&tag_for_remove);
                                        });
                                    }
                                >
                                    <svg class="h-3 w-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                                    </svg>
                                </button>
                            </span>
                        }
                    }).collect::<Vec<_>>()}
                </div>
            })
        }}
    }
}

/// Pagination component
#[component]
pub fn Pagination(
    /// Current page (0-indexed)
    current_page: RwSignal<usize>,
    /// Total number of pages
    #[prop(into)] total_pages: Signal<usize>,
    /// Total number of items
    #[prop(into)] total_items: Signal<usize>,
    /// Items per page
    #[prop(default = 10)]
    items_per_page: usize,
) -> impl IntoView {
    view! {
        {move || {
            let pages = total_pages.get();
            let items = total_items.get();
            let page = current_page.get();

            (pages > 1).then(|| {
                let start = page * items_per_page + 1;
                let end = ((page + 1) * items_per_page).min(items);

                view! {
                    <div class="flex items-center justify-between px-4 py-3 bg-white border-t border-gray-200 sm:px-6">
                        <div class="flex-1 flex justify-between sm:hidden">
                            <button
                                class="relative inline-flex items-center px-4 py-2 border border-gray-300 text-sm font-medium rounded-md text-gray-700 bg-white hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed"
                                disabled=move || page == 0
                                on:click=move |_| current_page.update(|p| *p = p.saturating_sub(1))
                            >
                                "Previous"
                            </button>
                            <button
                                class="ml-3 relative inline-flex items-center px-4 py-2 border border-gray-300 text-sm font-medium rounded-md text-gray-700 bg-white hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed"
                                disabled={move || page + 1 >= pages}
                                on:click=move |_| current_page.update(|p| *p += 1)
                            >
                                "Next"
                            </button>
                        </div>
                        <div class="hidden sm:flex-1 sm:flex sm:items-center sm:justify-between">
                            <div>
                                <p class="text-sm text-gray-700">
                                    "Showing "
                                    <span class="font-medium">{start}</span>
                                    " to "
                                    <span class="font-medium">{end}</span>
                                    " of "
                                    <span class="font-medium">{items}</span>
                                    " results"
                                </p>
                            </div>
                            <div>
                                <nav class="relative z-0 inline-flex rounded-md shadow-sm -space-x-px">
                                    // Previous button
                                    <button
                                        class="relative inline-flex items-center px-2 py-2 rounded-l-md border border-gray-300 bg-white text-sm font-medium text-gray-500 hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed"
                                        disabled=move || page == 0
                                        on:click=move |_| current_page.update(|p| *p = p.saturating_sub(1))
                                    >
                                        <svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7"/>
                                        </svg>
                                    </button>

                                    // Page numbers
                                    {(0..pages).map(|p| {
                                        let is_current = p == page;
                                        view! {
                                            <button
                                                class=move || if is_current {
                                                    "z-10 bg-green-50 border-green-500 text-green-600 relative inline-flex items-center px-4 py-2 border text-sm font-medium"
                                                } else {
                                                    "bg-white border-gray-300 text-gray-500 hover:bg-gray-50 relative inline-flex items-center px-4 py-2 border text-sm font-medium"
                                                }
                                                on:click=move |_| current_page.set(p)
                                            >
                                                {p + 1}
                                            </button>
                                        }
                                    }).collect::<Vec<_>>()}

                                    // Next button
                                    <button
                                        class="relative inline-flex items-center px-2 py-2 rounded-r-md border border-gray-300 bg-white text-sm font-medium text-gray-500 hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed"
                                        disabled={move || page + 1 >= pages}
                                        on:click=move |_| current_page.update(|p| *p += 1)
                                    >
                                        <svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"/>
                                        </svg>
                                    </button>
                                </nav>
                            </div>
                        </div>
                    </div>
                }
            })
        }}
    }
}

/// Tag display component for table/card views
#[component]
pub fn TagBadges(
    /// Tags to display
    #[prop(into)] tags: Vec<String>,
    /// Maximum tags to show before truncating
    #[prop(default = 3)]
    max_display: usize,
) -> impl IntoView {
    let display_tags: Vec<_> = tags.iter().take(max_display).cloned().collect();
    let remaining = tags.len().saturating_sub(max_display);

    view! {
        <div class="flex flex-wrap gap-1">
            {display_tags.into_iter().map(|tag| view! {
                <span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-gray-100 text-gray-600">
                    {tag}
                </span>
            }).collect::<Vec<_>>()}
            {(remaining > 0).then(|| view! {
                <span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-gray-200 text-gray-500">
                    {format!("+{}", remaining)}
                </span>
            })}
        </div>
    }
}

/// Tag input component for editing forms
#[component]
pub fn TagInput(
    /// Current tags
    tags: RwSignal<Vec<String>>,
    /// Label for the input
    #[prop(into, default = "Tags".to_string())]
    label: String,
    /// Placeholder for the input
    #[prop(into, default = "Add tag...".to_string())]
    placeholder: String,
) -> impl IntoView {
    let (new_tag, set_new_tag) = signal(String::new());

    // Use explicit get/set instead of update for better signal propagation through component hierarchies
    let add_tag = {
        let tags = tags;
        move |_| {
            let tag = new_tag.get().trim().to_string();
            if !tag.is_empty() {
                let mut current = tags.get();
                if !current.contains(&tag) {
                    current.push(tag.clone());
                    web_sys::console::log_1(&format!("TagInput: Adding tag '{}', new tags: {:?}", tag, current).into());
                    tags.set(current);
                }
                set_new_tag.set(String::new());
            }
        }
    };

    let remove_tag = {
        let tags = tags;
        move |tag_to_remove: String| {
            let mut current = tags.get();
            current.retain(|x| x != &tag_to_remove);
            web_sys::console::log_1(&format!("TagInput: Removing tag, new tags: {:?}", current).into());
            tags.set(current);
        }
    };

    view! {
        <div class="space-y-2">
            <label class="block text-sm font-medium text-gray-700">{label}</label>

            // Current tags
            <div class="flex flex-wrap gap-2">
                {move || tags.get().into_iter().map(|tag| {
                    let tag_clone = tag.clone();
                    let remove = remove_tag.clone();
                    view! {
                        <span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-sm font-medium bg-green-100 text-green-800">
                            {tag}
                            <button
                                type="button"
                                class="ml-1 inline-flex items-center justify-center text-green-400 hover:text-green-600"
                                on:click=move |_| remove(tag_clone.clone())
                            >
                                <svg class="h-3 w-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                                </svg>
                            </button>
                        </span>
                    }
                }).collect::<Vec<_>>()}
            </div>

            // Add new tag input
            <div class="flex gap-2">
                <input
                    type="text"
                    class="flex-1 px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500 text-sm"
                    placeholder=placeholder
                    prop:value=move || new_tag.get()
                    on:input=move |ev| set_new_tag.set(event_target_value(&ev))
                    on:keydown=move |ev| {
                        if ev.key() == "Enter" {
                            ev.prevent_default();
                            add_tag(());
                        }
                    }
                />
                <button
                    type="button"
                    class="px-3 py-2 bg-green-500 text-white rounded-md hover:bg-green-600 text-sm"
                    on:click=move |_| add_tag(())
                >
                    "Add"
                </button>
            </div>
        </div>
    }
}
