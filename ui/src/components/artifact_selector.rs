//! Unified Artifact Selector Component
//!
//! A searchable dropdown for selecting tools, resources, agents, workflows, and MCP tools.
//! Supports both single-select (for workflow steps) and multi-select (for agents) modes.

use leptos::prelude::*;
use leptos::ev::KeyboardEvent;

/// Artifact category for visual distinction
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ArtifactCategory {
    Tool,
    Resource,
    ResourceTemplate,
    Agent,
    Workflow,
    McpTool,
}

impl ArtifactCategory {
    /// Short label for badge display
    pub fn label(&self) -> &'static str {
        match self {
            Self::Tool => "Tool",
            Self::Resource => "Resource",
            Self::ResourceTemplate => "Template",
            Self::Agent => "Agent",
            Self::Workflow => "Workflow",
            Self::McpTool => "MCP",
        }
    }

    /// Icon SVG path for the category
    pub fn icon_path(&self) -> &'static str {
        match self {
            // Wrench icon for tools
            Self::Tool => "M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z M15 12a3 3 0 11-6 0 3 3 0 016 0z",
            // Document icon for resources
            Self::Resource => "M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z",
            // Template/document-duplicate icon
            Self::ResourceTemplate => "M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z",
            // User/robot icon for agents
            Self::Agent => "M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z",
            // Flow/workflow icon
            Self::Workflow => "M4 5a1 1 0 011-1h14a1 1 0 011 1v2a1 1 0 01-1 1H5a1 1 0 01-1-1V5zM4 13a1 1 0 011-1h6a1 1 0 011 1v6a1 1 0 01-1 1H5a1 1 0 01-1-1v-6zM16 13a1 1 0 011-1h2a1 1 0 011 1v6a1 1 0 01-1 1h-2a1 1 0 01-1-1v-6z",
            // Plugin/external icon for MCP
            Self::McpTool => "M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1",
        }
    }

    /// Background color class for badges
    pub fn badge_bg(&self) -> &'static str {
        match self {
            Self::Tool => "bg-blue-100 dark:bg-blue-900/40",
            Self::Resource => "bg-emerald-100 dark:bg-emerald-900/40",
            Self::ResourceTemplate => "bg-teal-100 dark:bg-teal-900/40",
            Self::Agent => "bg-indigo-100 dark:bg-indigo-900/40",
            Self::Workflow => "bg-orange-100 dark:bg-orange-900/40",
            Self::McpTool => "bg-purple-100 dark:bg-purple-900/40",
        }
    }

    /// Text color class for badges
    pub fn badge_text(&self) -> &'static str {
        match self {
            Self::Tool => "text-blue-700 dark:text-blue-300",
            Self::Resource => "text-emerald-700 dark:text-emerald-300",
            Self::ResourceTemplate => "text-teal-700 dark:text-teal-300",
            Self::Agent => "text-indigo-700 dark:text-indigo-300",
            Self::Workflow => "text-orange-700 dark:text-orange-300",
            Self::McpTool => "text-purple-700 dark:text-purple-300",
        }
    }

    /// Border color for focus states
    #[allow(dead_code)]
    pub fn border_class(&self) -> &'static str {
        match self {
            Self::Tool => "border-blue-300 focus:border-blue-500 focus:ring-blue-500",
            Self::Resource => "border-emerald-300 focus:border-emerald-500 focus:ring-emerald-500",
            Self::ResourceTemplate => "border-teal-300 focus:border-teal-500 focus:ring-teal-500",
            Self::Agent => "border-indigo-300 focus:border-indigo-500 focus:ring-indigo-500",
            Self::Workflow => "border-orange-300 focus:border-orange-500 focus:ring-orange-500",
            Self::McpTool => "border-purple-300 focus:border-purple-500 focus:ring-purple-500",
        }
    }

    /// Hover background for dropdown items
    pub fn hover_bg(&self) -> &'static str {
        match self {
            Self::Tool => "hover:bg-blue-50 dark:hover:bg-blue-900/20",
            Self::Resource => "hover:bg-emerald-50 dark:hover:bg-emerald-900/20",
            Self::ResourceTemplate => "hover:bg-teal-50 dark:hover:bg-teal-900/20",
            Self::Agent => "hover:bg-indigo-50 dark:hover:bg-indigo-900/20",
            Self::Workflow => "hover:bg-orange-50 dark:hover:bg-orange-900/20",
            Self::McpTool => "hover:bg-purple-50 dark:hover:bg-purple-900/20",
        }
    }

    /// Chip styling for selected items
    pub fn chip_class(&self) -> &'static str {
        match self {
            Self::Tool => "bg-blue-100 text-blue-800 border-blue-200 dark:bg-blue-900/50 dark:text-blue-200 dark:border-blue-700",
            Self::Resource => "bg-emerald-100 text-emerald-800 border-emerald-200 dark:bg-emerald-900/50 dark:text-emerald-200 dark:border-emerald-700",
            Self::ResourceTemplate => "bg-teal-100 text-teal-800 border-teal-200 dark:bg-teal-900/50 dark:text-teal-200 dark:border-teal-700",
            Self::Agent => "bg-indigo-100 text-indigo-800 border-indigo-200 dark:bg-indigo-900/50 dark:text-indigo-200 dark:border-indigo-700",
            Self::Workflow => "bg-orange-100 text-orange-800 border-orange-200 dark:bg-orange-900/50 dark:text-orange-200 dark:border-orange-700",
            Self::McpTool => "bg-purple-100 text-purple-800 border-purple-200 dark:bg-purple-900/50 dark:text-purple-200 dark:border-purple-700",
        }
    }

    /// Remove button styling for chips
    pub fn chip_remove_class(&self) -> &'static str {
        match self {
            Self::Tool => "text-blue-500 hover:text-blue-700 dark:text-blue-400 dark:hover:text-blue-200",
            Self::Resource => "text-emerald-500 hover:text-emerald-700 dark:text-emerald-400 dark:hover:text-emerald-200",
            Self::ResourceTemplate => "text-teal-500 hover:text-teal-700 dark:text-teal-400 dark:hover:text-teal-200",
            Self::Agent => "text-indigo-500 hover:text-indigo-700 dark:text-indigo-400 dark:hover:text-indigo-200",
            Self::Workflow => "text-orange-500 hover:text-orange-700 dark:text-orange-400 dark:hover:text-orange-200",
            Self::McpTool => "text-purple-500 hover:text-purple-700 dark:text-purple-400 dark:hover:text-purple-200",
        }
    }
}

/// An artifact item that can be selected
#[derive(Clone, Debug)]
pub struct ArtifactItem {
    /// Unique identifier (tool name, resource URI, etc.)
    pub id: String,
    /// Display name
    pub name: String,
    /// Description text
    pub description: String,
    /// Category of the artifact
    pub category: ArtifactCategory,
    /// Optional server name for MCP tools
    pub server: Option<String>,
}

impl ArtifactItem {
    pub fn tool(name: impl Into<String>, description: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            id: name.clone(),
            name,
            description: description.into(),
            category: ArtifactCategory::Tool,
            server: None,
        }
    }

    pub fn resource(uri: impl Into<String>, name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: uri.into(),
            name: name.into(),
            description: description.into(),
            category: ArtifactCategory::Resource,
            server: None,
        }
    }

    pub fn resource_template(uri: impl Into<String>, name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: uri.into(),
            name: name.into(),
            description: description.into(),
            category: ArtifactCategory::ResourceTemplate,
            server: None,
        }
    }

    pub fn agent(name: impl Into<String>, description: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            id: name.clone(),
            name,
            description: description.into(),
            category: ArtifactCategory::Agent,
            server: None,
        }
    }

    pub fn workflow(name: impl Into<String>, description: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            id: format!("workflow_{}", name),
            name,
            description: description.into(),
            category: ArtifactCategory::Workflow,
            server: None,
        }
    }

    pub fn mcp_tool(server: impl Into<String>, tool_name: impl Into<String>, description: impl Into<String>) -> Self {
        let server = server.into();
        let tool_name = tool_name.into();
        Self {
            id: format!("{}:{}", server, tool_name),
            name: tool_name,
            description: description.into(),
            category: ArtifactCategory::McpTool,
            server: Some(server),
        }
    }
}

/// Selection mode for the artifact selector
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum SelectionMode {
    /// Allow only one selection (for workflow steps)
    Single,
    /// Allow multiple selections (for agents)
    #[default]
    Multi,
}

/// Category icon component
#[component]
fn CategoryIcon(category: ArtifactCategory, #[prop(default = "w-4 h-4")] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="1.5">
            <path stroke-linecap="round" stroke-linejoin="round" d=category.icon_path() />
        </svg>
    }
}

/// Category badge with icon and label
#[component]
fn CategoryBadge(category: ArtifactCategory, server: Option<String>) -> impl IntoView {
    view! {
        <span class=format!(
            "inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium {}",
            format!("{} {}", category.badge_bg(), category.badge_text())
        )>
            <CategoryIcon category=category class="w-3 h-3" />
            {category.label()}
            {server.map(|s| view! {
                <span class="opacity-70">{format!("@{}", s)}</span>
            })}
        </span>
    }
}

/// Unified Artifact Selector Component
///
/// A searchable dropdown that supports selecting tools, resources, agents, workflows, and MCP tools.
/// Supports both single-select and multi-select modes with category grouping and visual badges.
#[component]
pub fn ArtifactSelector(
    /// All available artifacts to select from
    items: Signal<Vec<ArtifactItem>>,
    /// Currently selected artifact IDs
    selected: Signal<Vec<String>>,
    /// Callback when selection changes
    on_change: Callback<Vec<String>>,
    /// Selection mode (single or multi)
    #[prop(default = SelectionMode::Multi)]
    mode: SelectionMode,
    /// Placeholder text for search input
    #[prop(default = "Search artifacts...")]
    placeholder: &'static str,
    /// Label for the component
    #[prop(optional)]
    label: Option<&'static str>,
    /// Help text below the component
    #[prop(optional)]
    help_text: Option<&'static str>,
    /// Maximum items to show in dropdown
    #[prop(default = 15)]
    max_results: usize,
    /// Whether to group items by category in dropdown
    #[prop(default = true)]
    group_by_category: bool,
    /// Optional category filter - only show items from these categories
    #[prop(optional)]
    category_filter: Option<Vec<ArtifactCategory>>,
) -> impl IntoView {
    let (search_query, set_search_query) = signal(String::new());
    let (show_dropdown, set_show_dropdown) = signal(false);
    let (focused_index, set_focused_index) = signal(Option::<usize>::None);

    // Filter items based on search query and category filter
    let cat_filter_for_filtered = category_filter.clone();
    let filtered_items = Signal::derive(move || {
        let query = search_query.get().to_lowercase();
        let all_items = items.get();
        let selected_ids = selected.get();
        let cat_filter = cat_filter_for_filtered.clone();

        let filtered: Vec<_> = all_items
            .into_iter()
            .filter(|item| {
                // Apply category filter if specified
                if let Some(ref cats) = cat_filter {
                    if !cats.contains(&item.category) {
                        return false;
                    }
                }
                // Exclude already selected items
                if selected_ids.contains(&item.id) {
                    return false;
                }
                // Apply search filter
                if query.is_empty() {
                    return true;
                }
                item.name.to_lowercase().contains(&query)
                    || item.description.to_lowercase().contains(&query)
                    || item.category.label().to_lowercase().contains(&query)
                    || item.server.as_ref().map(|s| s.to_lowercase().contains(&query)).unwrap_or(false)
            })
            .collect();

        if group_by_category {
            // Sort by category then by name
            let mut sorted = filtered;
            sorted.sort_by(|a, b| {
                match a.category.label().cmp(b.category.label()) {
                    std::cmp::Ordering::Equal => a.name.cmp(&b.name),
                    other => other,
                }
            });
            sorted.into_iter().take(max_results).collect()
        } else {
            filtered.into_iter().take(max_results).collect()
        }
    });

    // Get selected items for display
    let selected_items = move || {
        let all_items = items.get();
        let selected_ids = selected.get();
        all_items
            .into_iter()
            .filter(|item| selected_ids.contains(&item.id))
            .collect::<Vec<_>>()
    };

    // Add item to selection - wrapped in Callback for use in multiple closures
    let add_item = Callback::new(move |item_id: String| {
        let mut current = selected.get();
        if mode == SelectionMode::Single {
            current = vec![item_id];
        } else if !current.contains(&item_id) {
            current.push(item_id);
        }
        on_change.run(current);
        set_search_query.set(String::new());
        set_focused_index.set(None);
        if mode == SelectionMode::Single {
            set_show_dropdown.set(false);
        }
    });

    // Remove item from selection - wrapped in Callback for use in multiple closures
    let remove_item = Callback::new(move |item_id: String| {
        let mut current = selected.get();
        current.retain(|id| id != &item_id);
        on_change.run(current);
    });

    // Clear all selections
    let clear_all = move |_| {
        on_change.run(vec![]);
    };

    // Handle keyboard navigation
    let on_keydown = move |ev: KeyboardEvent| {
        let key = ev.key();
        let filtered: Vec<ArtifactItem> = filtered_items.get();
        let filtered_len = filtered.len();

        match key.as_str() {
            "ArrowDown" => {
                ev.prevent_default();
                set_show_dropdown.set(true);
                set_focused_index.update(|idx| {
                    *idx = Some(match idx {
                        Some(i) if *i < filtered_len.saturating_sub(1) => *i + 1,
                        Some(_) => 0,
                        None => 0,
                    });
                });
            }
            "ArrowUp" => {
                ev.prevent_default();
                set_focused_index.update(|idx| {
                    *idx = Some(match idx {
                        Some(i) if *i > 0 => *i - 1,
                        Some(_) => filtered_len.saturating_sub(1),
                        None => filtered_len.saturating_sub(1),
                    });
                });
            }
            "Enter" => {
                ev.prevent_default();
                if let Some(idx) = focused_index.get() {
                    if let Some(item) = filtered.get(idx) {
                        add_item.run(item.id.clone());
                    }
                }
            }
            "Escape" => {
                set_show_dropdown.set(false);
                set_focused_index.set(None);
            }
            "Backspace" => {
                if search_query.get().is_empty() && mode == SelectionMode::Multi {
                    // Remove last selected item
                    let mut current = selected.get();
                    if !current.is_empty() {
                        current.pop();
                        on_change.run(current);
                    }
                }
            }
            _ => {}
        }
    };

    let cat_filter_for_total = category_filter.clone();
    let total_count = move || {
        let cat_filter = cat_filter_for_total.clone();
        items.get().into_iter().filter(|item| {
            if let Some(ref cats) = cat_filter {
                cats.contains(&item.category)
            } else {
                true
            }
        }).count()
    };
    let selected_count = move || selected.get().len();

    view! {
        <div class="space-y-2">
            // Label with count
            {label.map(|l| view! {
                <div class="flex items-center justify-between">
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                        {l}
                        <span class="ml-2 text-gray-400 font-normal text-xs">
                            "("{selected_count}" / "{total_count}")"
                        </span>
                    </label>
                    {move || {
                        if mode == SelectionMode::Multi && selected_count() > 0 {
                            view! {
                                <button
                                    type="button"
                                    class="text-xs text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200"
                                    on:click=clear_all
                                >
                                    "Clear all"
                                </button>
                            }.into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }
                    }}
                </div>
            })}

            // Selected items as chips (for multi-select), grouped by category
            {move || {
                let items = selected_items();
                if items.is_empty() || mode == SelectionMode::Single {
                    view! { <span></span> }.into_any()
                } else {
                    // Group items by category
                    let mut groups: std::collections::HashMap<ArtifactCategory, Vec<ArtifactItem>> = std::collections::HashMap::new();
                    for item in items {
                        groups.entry(item.category).or_default().push(item);
                    }

                    // Define category order
                    let category_order = [
                        ArtifactCategory::Tool,
                        ArtifactCategory::McpTool,
                        ArtifactCategory::Workflow,
                        ArtifactCategory::Agent,
                        ArtifactCategory::Resource,
                        ArtifactCategory::ResourceTemplate,
                    ];

                    view! {
                        <div class="space-y-3 mb-4">
                            {category_order.iter().filter_map(|cat| {
                                groups.get(cat).map(|items| {
                                    let category = *cat;
                                    view! {
                                        <div class=format!("rounded-lg p-2 {}", category.badge_bg())>
                                            <div class=format!("flex items-center gap-1.5 mb-2 text-xs font-semibold {}", category.badge_text())>
                                                <CategoryIcon category=category class="w-3.5 h-3.5" />
                                                {category.label()}"s"
                                                <span class="ml-1 opacity-70">"("{items.len()}")"</span>
                                            </div>
                                            <div class="flex flex-wrap gap-1.5">
                                                {items.iter().map(|item| {
                                                    let item_id = item.id.clone();
                                                    let item_id_for_remove = item_id.clone();
                                                    view! {
                                                        <span class=format!(
                                                            "inline-flex items-center gap-1.5 pl-2 pr-1 py-0.5 rounded-md text-xs font-medium border bg-white/50 dark:bg-gray-800/50 {}",
                                                            item.category.chip_class()
                                                        )>
                                                            <span class="truncate max-w-[150px]">{item.name.clone()}</span>
                                                            {item.server.clone().map(|s| view! {
                                                                <span class="text-[10px] opacity-60">{format!("@{}", s)}</span>
                                                            })}
                                                            <button
                                                                type="button"
                                                                class=format!("ml-0.5 p-0.5 rounded hover:bg-black/10 dark:hover:bg-white/10 {}", item.category.chip_remove_class())
                                                                on:click=move |_| remove_item.run(item_id_for_remove.clone())
                                                            >
                                                                <svg class="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                                                                    <path fill-rule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clip-rule="evenodd"/>
                                                                </svg>
                                                            </button>
                                                        </span>
                                                    }
                                                }).collect::<Vec<_>>()}
                                            </div>
                                        </div>
                                    }
                                })
                            }).collect::<Vec<_>>()}
                        </div>
                    }.into_any()
                }
            }}

            // Search input with dropdown
            <div class="relative">
                // For single-select, show selected value in input area
                {move || {
                    if mode == SelectionMode::Single {
                        let items = selected_items();
                        if let Some(item) = items.first() {
                            return view! {
                                <div class="flex items-center gap-2 w-full px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-md shadow-sm bg-white dark:bg-gray-700">
                                    <CategoryBadge category=item.category server=item.server.clone() />
                                    <span class="flex-1 text-gray-900 dark:text-white truncate">{item.name.clone()}</span>
                                    <button
                                        type="button"
                                        class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-200"
                                        on:click=move |_| {
                                            on_change.run(vec![]);
                                            set_show_dropdown.set(true);
                                        }
                                    >
                                        <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
                                            <path fill-rule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clip-rule="evenodd"/>
                                        </svg>
                                    </button>
                                </div>
                            }.into_any();
                        }
                    }
                    view! { <span></span> }.into_any()
                }}

                // Search input - conditionally hidden for single-select with value
                <div
                    class="relative"
                    style:display=move || {
                        if mode == SelectionMode::Single && !selected.get().is_empty() {
                            "none"
                        } else {
                            "block"
                        }
                    }
                >
                    <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                        <svg class="h-4 w-4 text-gray-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                        </svg>
                    </div>
                    <input
                        type="text"
                        class="w-full pl-10 pr-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:ring-2 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white"
                        placeholder=placeholder
                        prop:value=move || search_query.get()
                        on:input=move |ev| {
                            set_search_query.set(event_target_value(&ev));
                            set_show_dropdown.set(true);
                            set_focused_index.set(None);
                        }
                        on:focus=move |_| set_show_dropdown.set(true)
                        on:blur=move |_| {
                            // Delay hiding to allow click on dropdown items
                            set_timeout(move || set_show_dropdown.set(false), std::time::Duration::from_millis(200));
                        }
                        on:keydown=on_keydown
                    />
                </div>

                // Dropdown with filtered results
                {
                    let category_filter_for_dropdown = category_filter.clone();
                    move || {
                    // Don't show dropdown for single-select with value unless explicitly opened
                    if !show_dropdown.get() {
                        return view! { <span></span> }.into_any();
                    }

                    let filtered = filtered_items.get();
                    if filtered.is_empty() {
                        let query = search_query.get();
                        return view! {
                            <div class="absolute z-50 w-full mt-1 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-md shadow-lg">
                                <div class="px-4 py-3 text-sm text-gray-500 dark:text-gray-400 flex items-center gap-2">
                                    <svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.172 16.172a4 4 0 015.656 0M9 10h.01M15 10h.01M12 12h.01M12 12h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                                    </svg>
                                    {if query.is_empty() {
                                        "All items selected or no items available"
                                    } else {
                                        "No matching items found"
                                    }}
                                </div>
                            </div>
                        }.into_any();
                    }

                    let focused_idx = focused_index.get();

                    // Group items by category if enabled
                    let mut current_category: Option<ArtifactCategory> = None;
                    let items_with_headers: Vec<(Option<ArtifactCategory>, Option<ArtifactItem>, usize)> = if group_by_category {
                        let mut result = Vec::new();
                        for (idx, item) in filtered.iter().enumerate() {
                            if current_category != Some(item.category) {
                                current_category = Some(item.category);
                                result.push((Some(item.category), None, idx));
                            }
                            result.push((None, Some(item.clone()), idx));
                        }
                        result
                    } else {
                        filtered.iter().enumerate().map(|(idx, item)| (None, Some(item.clone()), idx)).collect()
                    };

                    view! {
                        <div class="absolute z-50 w-full mt-1 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-md shadow-lg max-h-72 overflow-y-auto">
                            {items_with_headers.into_iter().map(|(header_cat, item_opt, idx)| {
                                if let Some(cat) = header_cat {
                                    // Category header
                                    view! {
                                        <div class=format!(
                                            "sticky top-0 px-3 py-1.5 text-xs font-semibold {} {} border-b border-gray-100 dark:border-gray-700 flex items-center gap-1.5",
                                            cat.badge_bg(), cat.badge_text()
                                        )>
                                            <CategoryIcon category=cat class="w-3.5 h-3.5" />
                                            {cat.label()}"s"
                                        </div>
                                    }.into_any()
                                } else if let Some(item) = item_opt {
                                    let item_id = item.id.clone();
                                    let item_id_for_click = item_id.clone();
                                    let is_focused = focused_idx == Some(idx);
                                    let focus_class = if is_focused {
                                        "bg-gray-100 dark:bg-gray-700"
                                    } else {
                                        ""
                                    };

                                    view! {
                                        <div
                                            class=format!(
                                                "px-3 py-2 cursor-pointer {} {} transition-colors duration-75",
                                                item.category.hover_bg(), focus_class
                                            )
                                            on:mousedown=move |_| add_item.run(item_id_for_click.clone())
                                        >
                                            <div class="flex items-start gap-2">
                                                <span class=format!("mt-0.5 {}", item.category.badge_text())>
                                                    <CategoryIcon category=item.category class="w-4 h-4" />
                                                </span>
                                                <div class="flex-1 min-w-0">
                                                    <div class="flex items-center gap-2">
                                                        <span class="text-sm font-medium text-gray-900 dark:text-white truncate">
                                                            {item.name}
                                                        </span>
                                                        {item.server.map(|s| view! {
                                                            <span class="text-[10px] text-gray-500 dark:text-gray-400 bg-gray-100 dark:bg-gray-700 px-1.5 py-0.5 rounded">
                                                                {format!("@{}", s)}
                                                            </span>
                                                        })}
                                                    </div>
                                                    <div class="text-xs text-gray-500 dark:text-gray-400 mt-0.5 line-clamp-2">
                                                        {item.description}
                                                    </div>
                                                </div>
                                            </div>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! { <span></span> }.into_any()
                                }
                            }).collect::<Vec<_>>()}

                            // Show remaining count
                            {
                                let cat_filter_inner = category_filter_for_dropdown.clone();
                                move || {
                                let total = items.get().into_iter().filter(|item| {
                                    if let Some(ref cats) = cat_filter_inner {
                                        cats.contains(&item.category)
                                    } else {
                                        true
                                    }
                                }).count();
                                let shown = filtered_items.get().len();
                                let remaining = total.saturating_sub(selected.get().len()).saturating_sub(shown);
                                if remaining > 0 {
                                    view! {
                                        <div class="px-3 py-2 text-xs text-gray-400 dark:text-gray-500 border-t border-gray-100 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50">
                                            <span class="font-medium">{remaining}</span>" more available - type to search"
                                        </div>
                                    }.into_any()
                                } else {
                                    view! { <span></span> }.into_any()
                                }
                            }}
                        </div>
                    }.into_any()
                }}
            </div>

            // Help text
            {help_text.map(|text| view! {
                <p class="text-xs text-gray-500 dark:text-gray-400">{text}</p>
            })}
        </div>
    }
}
