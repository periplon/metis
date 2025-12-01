//! Version History UI Component
//!
//! Provides a UI for browsing commits, tags, and rolling back to previous versions.

use leptos::prelude::*;
use leptos::web_sys;
use wasm_bindgen::JsCast;
use crate::api;
use crate::types::{Commit, Tag};

#[component]
pub fn VersionHistory() -> impl IntoView {
    let (refresh_trigger, set_refresh_trigger) = signal(0u32);
    let (selected_commit, set_selected_commit) = signal(Option::<String>::None);
    let (show_create_tag_modal, set_show_create_tag_modal) = signal(false);
    let (tag_commit_hash, set_tag_commit_hash) = signal(String::new());
    let (message, set_message) = signal(Option::<(String, bool)>::None);

    // Fetch commits
    let commits = LocalResource::new(move || {
        let _ = refresh_trigger.get();
        async move { api::list_commits(50, 0).await.ok() }
    });

    // Fetch tags
    let tags = LocalResource::new(move || {
        let _ = refresh_trigger.get();
        async move { api::list_tags().await.ok() }
    });

    // Fetch database status
    let db_status = LocalResource::new(move || {
        let _ = refresh_trigger.get();
        async move { api::get_database_status().await.ok() }
    });

    view! {
        <div class="p-6">
            <div class="flex justify-between items-center mb-6">
                <h2 class="text-2xl font-bold">"Version History"</h2>
                <button
                    class="px-4 py-2 bg-gray-100 text-gray-700 rounded hover:bg-gray-200"
                    on:click=move |_| set_refresh_trigger.update(|n| *n += 1)
                >
                    "Refresh"
                </button>
            </div>

            // Message banner
            {move || message.get().map(|(msg, success)| {
                let color = if success { "bg-green-100 border-green-400 text-green-700" } else { "bg-red-100 border-red-400 text-red-700" };
                view! {
                    <div class=format!("p-3 mb-4 border rounded {}", color)>
                        <div class="flex justify-between items-center">
                            <span>{msg}</span>
                            <button
                                class="text-gray-500 hover:text-gray-700"
                                on:click=move |_| set_message.set(None)
                            >
                                "×"
                            </button>
                        </div>
                    </div>
                }
            })}

            <Suspense fallback=move || view! { <div class="text-gray-500">"Loading..."</div> }>
                {move || {
                    let db = db_status.get().flatten();
                    match db {
                        Some(status) if status.enabled && status.healthy => {
                            let commits_data = commits.get().flatten().unwrap_or_default();
                            let tags_data = tags.get().flatten().unwrap_or_default();
                            let head_hash = status.head.as_ref().map(|h| h.commit_hash.clone());

                            view! {
                                <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
                                    // Left: Commits list
                                    <div class="lg:col-span-2">
                                        <CommitsList
                                            commits=commits_data
                                            tags=tags_data.clone()
                                            head_hash=head_hash
                                            set_selected_commit=set_selected_commit
                                            set_message=set_message
                                            set_refresh_trigger=set_refresh_trigger
                                            set_show_create_tag_modal=set_show_create_tag_modal
                                            set_tag_commit_hash=set_tag_commit_hash
                                        />
                                    </div>

                                    // Right: Tags panel
                                    <div>
                                        <TagsPanel
                                            tags=tags_data
                                            set_message=set_message
                                            set_refresh_trigger=set_refresh_trigger
                                            set_selected_commit=set_selected_commit
                                        />
                                    </div>
                                </div>

                                // Commit details modal
                                {move || selected_commit.get().map(|hash| {
                                    view! {
                                        <CommitDetailsModal
                                            commit_hash=hash
                                            on_close=move || set_selected_commit.set(None)
                                        />
                                    }
                                })}

                                // Create tag modal
                                {move || {
                                    if show_create_tag_modal.get() {
                                        Some(view! {
                                            <CreateTagModal
                                                commit_hash=tag_commit_hash.get()
                                                on_close=move || set_show_create_tag_modal.set(false)
                                                on_created=move |_| {
                                                    set_show_create_tag_modal.set(false);
                                                    set_refresh_trigger.update(|n| *n += 1);
                                                    set_message.set(Some(("Tag created successfully".to_string(), true)));
                                                }
                                            />
                                        })
                                    } else {
                                        None
                                    }
                                }}
                            }.into_any()
                        }
                        Some(status) if !status.enabled => {
                            view! {
                                <div class="bg-yellow-50 border border-yellow-200 rounded-lg p-6 text-center">
                                    <svg class="w-12 h-12 mx-auto text-yellow-400 mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/>
                                    </svg>
                                    <h3 class="text-lg font-semibold text-yellow-800 mb-2">"Database Not Configured"</h3>
                                    <p class="text-yellow-700 mb-4">
                                        "Version history requires database persistence to be enabled."
                                    </p>
                                    <a href="/config" class="inline-block px-4 py-2 bg-yellow-600 text-white rounded hover:bg-yellow-700">
                                        "Configure Database"
                                    </a>
                                </div>
                            }.into_any()
                        }
                        _ => {
                            view! {
                                <div class="bg-red-50 border border-red-200 rounded-lg p-6 text-center">
                                    <h3 class="text-lg font-semibold text-red-800 mb-2">"Database Unavailable"</h3>
                                    <p class="text-red-700">"Unable to connect to the database. Check your configuration."</p>
                                </div>
                            }.into_any()
                        }
                    }
                }}
            </Suspense>
        </div>
    }
}

#[component]
fn CommitsList(
    commits: Vec<Commit>,
    tags: Vec<Tag>,
    head_hash: Option<String>,
    set_selected_commit: WriteSignal<Option<String>>,
    set_message: WriteSignal<Option<(String, bool)>>,
    set_refresh_trigger: WriteSignal<u32>,
    set_show_create_tag_modal: WriteSignal<bool>,
    set_tag_commit_hash: WriteSignal<String>,
) -> impl IntoView {
    view! {
        <div class="bg-white rounded-lg shadow">
            <div class="px-4 py-3 border-b border-gray-200 bg-gray-50">
                <h3 class="text-lg font-semibold text-gray-800">"Commits"</h3>
            </div>

            {if commits.is_empty() {
                view! {
                    <div class="p-8 text-center text-gray-500">
                        <svg class="w-12 h-12 mx-auto text-gray-300 mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
                        </svg>
                        <p>"No commits yet"</p>
                        <p class="text-sm text-gray-400 mt-1">"Make changes to create your first commit"</p>
                    </div>
                }.into_any()
            } else {
                let head = head_hash.clone();
                view! {
                    <div class="divide-y divide-gray-100">
                        {commits.into_iter().map(|commit| {
                            let hash = commit.commit_hash.clone();
                            let short_hash = if hash.len() >= 8 { hash[..8].to_string() } else { hash.clone() };
                            let is_head = head.as_ref().map(|h| h == &hash).unwrap_or(false);
                            let commit_tag = tags.iter().find(|t| t.commit_hash == hash).map(|t| t.name.clone());
                            let hash_for_details = hash.clone();
                            let hash_for_rollback = hash.clone();
                            let hash_for_tag = hash.clone();

                            view! {
                                <div class="p-4 hover:bg-gray-50">
                                    <div class="flex items-start justify-between">
                                        <div class="flex-1 min-w-0">
                                            <div class="flex items-center gap-2 mb-1">
                                                <code class="text-sm font-mono text-blue-600 bg-blue-50 px-2 py-0.5 rounded">
                                                    {short_hash}
                                                </code>
                                                {is_head.then(|| view! {
                                                    <span class="px-2 py-0.5 text-xs font-medium bg-green-100 text-green-700 rounded">
                                                        "HEAD"
                                                    </span>
                                                })}
                                                {commit.is_snapshot.then(|| view! {
                                                    <span class="px-2 py-0.5 text-xs font-medium bg-purple-100 text-purple-700 rounded">
                                                        "Snapshot"
                                                    </span>
                                                })}
                                                {commit_tag.map(|tag| view! {
                                                    <span class="px-2 py-0.5 text-xs font-medium bg-yellow-100 text-yellow-700 rounded flex items-center gap-1">
                                                        <svg class="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                                                            <path fill-rule="evenodd" d="M17.707 9.293a1 1 0 010 1.414l-7 7a1 1 0 01-1.414 0l-7-7A.997.997 0 012 10V5a3 3 0 013-3h5c.256 0 .512.098.707.293l7 7zM5 6a1 1 0 100-2 1 1 0 000 2z" clip-rule="evenodd"/>
                                                        </svg>
                                                        {tag}
                                                    </span>
                                                })}
                                            </div>
                                            <p class="text-gray-800 font-medium truncate">{commit.message.clone()}</p>
                                            <div class="flex items-center gap-4 mt-1 text-sm text-gray-500">
                                                {commit.author.clone().map(|a| view! { <span>{a}</span> })}
                                                <span>{commit.committed_at.clone()}</span>
                                                <span>{format!("{} changes", commit.changes_count)}</span>
                                            </div>
                                        </div>
                                        <div class="flex items-center gap-2 ml-4">
                                            <button
                                                class="px-3 py-1 text-sm text-gray-600 hover:text-gray-800 hover:bg-gray-100 rounded"
                                                on:click=move |_| set_selected_commit.set(Some(hash_for_details.clone()))
                                            >
                                                "Details"
                                            </button>
                                            {(!is_head).then(|| {
                                                let hash_rb = hash_for_rollback.clone();
                                                view! {
                                                    <button
                                                        class="px-3 py-1 text-sm text-orange-600 hover:text-orange-800 hover:bg-orange-50 rounded"
                                                        on:click=move |_| {
                                                            let h = hash_rb.clone();
                                                            wasm_bindgen_futures::spawn_local(async move {
                                                                match api::rollback_to_commit(&h).await {
                                                                    Ok(_) => {
                                                                        set_message.set(Some((format!("Rolled back to {}", &h[..8]), true)));
                                                                        set_refresh_trigger.update(|n| *n += 1);
                                                                    }
                                                                    Err(e) => {
                                                                        set_message.set(Some((format!("Rollback failed: {}", e), false)));
                                                                    }
                                                                }
                                                            });
                                                        }
                                                    >
                                                        "Checkout"
                                                    </button>
                                                }
                                            })}
                                            <button
                                                class="px-3 py-1 text-sm text-yellow-600 hover:text-yellow-800 hover:bg-yellow-50 rounded"
                                                on:click=move |_| {
                                                    set_tag_commit_hash.set(hash_for_tag.clone());
                                                    set_show_create_tag_modal.set(true);
                                                }
                                            >
                                                "Tag"
                                            </button>
                                        </div>
                                    </div>
                                </div>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                }.into_any()
            }}
        </div>
    }
}

#[component]
fn TagsPanel(
    tags: Vec<Tag>,
    set_message: WriteSignal<Option<(String, bool)>>,
    set_refresh_trigger: WriteSignal<u32>,
    set_selected_commit: WriteSignal<Option<String>>,
) -> impl IntoView {
    view! {
        <div class="bg-white rounded-lg shadow">
            <div class="px-4 py-3 border-b border-gray-200 bg-gray-50">
                <h3 class="text-lg font-semibold text-gray-800">"Tags"</h3>
            </div>

            {if tags.is_empty() {
                view! {
                    <div class="p-6 text-center text-gray-500">
                        <svg class="w-10 h-10 mx-auto text-gray-300 mb-3" fill="currentColor" viewBox="0 0 20 20">
                            <path fill-rule="evenodd" d="M17.707 9.293a1 1 0 010 1.414l-7 7a1 1 0 01-1.414 0l-7-7A.997.997 0 012 10V5a3 3 0 013-3h5c.256 0 .512.098.707.293l7 7zM5 6a1 1 0 100-2 1 1 0 000 2z" clip-rule="evenodd"/>
                        </svg>
                        <p class="text-sm">"No tags yet"</p>
                    </div>
                }.into_any()
            } else {
                view! {
                    <div class="divide-y divide-gray-100">
                        {tags.into_iter().map(|tag| {
                            let tag_name = tag.name.clone();
                            let tag_name_for_delete = tag.name.clone();
                            let commit_hash = tag.commit_hash.clone();
                            let short_hash = if commit_hash.len() >= 8 { commit_hash[..8].to_string() } else { commit_hash.clone() };
                            let hash_for_checkout = commit_hash.clone();
                            let hash_for_view = commit_hash.clone();

                            view! {
                                <div class="p-3 hover:bg-gray-50">
                                    <div class="flex items-center justify-between">
                                        <div>
                                            <div class="flex items-center gap-2">
                                                <span class="font-medium text-yellow-700">{tag_name}</span>
                                                <code class="text-xs text-gray-500">{short_hash}</code>
                                            </div>
                                            {tag.message.map(|m| view! {
                                                <p class="text-sm text-gray-500 mt-1">{m}</p>
                                            })}
                                        </div>
                                        <div class="flex items-center gap-1">
                                            <button
                                                class="p-1 text-gray-400 hover:text-blue-600"
                                                title="View commit"
                                                on:click=move |_| set_selected_commit.set(Some(hash_for_view.clone()))
                                            >
                                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"/>
                                                </svg>
                                            </button>
                                            <button
                                                class="p-1 text-gray-400 hover:text-orange-600"
                                                title="Checkout this tag"
                                                on:click=move |_| {
                                                    let h = hash_for_checkout.clone();
                                                    wasm_bindgen_futures::spawn_local(async move {
                                                        match api::rollback_to_commit(&h).await {
                                                            Ok(_) => {
                                                                set_message.set(Some(("Checked out tag successfully".to_string(), true)));
                                                                set_refresh_trigger.update(|n| *n += 1);
                                                            }
                                                            Err(e) => {
                                                                set_message.set(Some((format!("Checkout failed: {}", e), false)));
                                                            }
                                                        }
                                                    });
                                                }
                                            >
                                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/>
                                                </svg>
                                            </button>
                                            <button
                                                class="p-1 text-gray-400 hover:text-red-600"
                                                title="Delete tag"
                                                on:click=move |_| {
                                                    let name = tag_name_for_delete.clone();
                                                    wasm_bindgen_futures::spawn_local(async move {
                                                        match api::delete_tag(&name).await {
                                                            Ok(_) => {
                                                                set_message.set(Some(("Tag deleted".to_string(), true)));
                                                                set_refresh_trigger.update(|n| *n += 1);
                                                            }
                                                            Err(e) => {
                                                                set_message.set(Some((format!("Delete failed: {}", e), false)));
                                                            }
                                                        }
                                                    });
                                                }
                                            >
                                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/>
                                                </svg>
                                            </button>
                                        </div>
                                    </div>
                                </div>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                }.into_any()
            }}
        </div>
    }
}

#[component]
fn CommitDetailsModal(
    commit_hash: String,
    on_close: impl Fn() + 'static,
) -> impl IntoView {
    let hash = commit_hash.clone();
    let commit = LocalResource::new(move || {
        let h = hash.clone();
        async move { api::get_commit(&h).await.ok() }
    });

    let hash_for_changesets = commit_hash.clone();
    let changesets = LocalResource::new(move || {
        let h = hash_for_changesets.clone();
        async move { api::get_commit_changesets(&h).await.ok() }
    });

    view! {
        <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
            <div class="bg-white rounded-lg shadow-xl max-w-3xl w-full mx-4 max-h-[80vh] overflow-hidden flex flex-col">
                <div class="px-6 py-4 border-b border-gray-200 flex items-center justify-between bg-gray-50">
                    <h3 class="text-lg font-semibold text-gray-800">"Commit Details"</h3>
                    <button
                        class="text-gray-400 hover:text-gray-600"
                        on:click=move |_| on_close()
                    >
                        <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                        </svg>
                    </button>
                </div>

                <div class="p-6 overflow-y-auto flex-1">
                    <Suspense fallback=move || view! { <div class="text-gray-500">"Loading..."</div> }>
                        {move || {
                            match commit.get().flatten() {
                                Some(c) => {
                                    let changes = changesets.get().flatten().unwrap_or_default();
                                    view! {
                                        <div class="space-y-4">
                                            <div class="grid grid-cols-2 gap-4 text-sm">
                                                <div>
                                                    <span class="text-gray-500">"Hash: "</span>
                                                    <code class="font-mono text-blue-600">{c.commit_hash.clone()}</code>
                                                </div>
                                                {c.parent_hash.map(|p| view! {
                                                    <div>
                                                        <span class="text-gray-500">"Parent: "</span>
                                                        <code class="font-mono text-gray-600">{p}</code>
                                                    </div>
                                                })}
                                                <div>
                                                    <span class="text-gray-500">"Date: "</span>
                                                    <span>{c.committed_at.clone()}</span>
                                                </div>
                                                {c.author.clone().map(|a| view! {
                                                    <div>
                                                        <span class="text-gray-500">"Author: "</span>
                                                        <span>{a}</span>
                                                    </div>
                                                })}
                                            </div>

                                            <div class="bg-gray-50 p-3 rounded">
                                                <p class="font-medium">{c.message.clone()}</p>
                                            </div>

                                            <div>
                                                <h4 class="font-semibold text-gray-700 mb-2">
                                                    {format!("Changes ({})", changes.len())}
                                                </h4>
                                                <div class="border rounded divide-y">
                                                    {changes.into_iter().map(|cs| {
                                                        let (op_color, op_icon) = match cs.operation.as_str() {
                                                            "create" => ("text-green-600 bg-green-50", "+"),
                                                            "update" => ("text-blue-600 bg-blue-50", "~"),
                                                            "delete" => ("text-red-600 bg-red-50", "-"),
                                                            _ => ("text-gray-600 bg-gray-50", "?"),
                                                        };
                                                        view! {
                                                            <div class="p-3 flex items-center gap-3">
                                                                <span class=format!("w-6 h-6 rounded flex items-center justify-center font-mono font-bold {}", op_color)>
                                                                    {op_icon}
                                                                </span>
                                                                <div>
                                                                    <span class="text-gray-500 text-sm">{cs.archetype_type.clone()}</span>
                                                                    <span class="mx-1 text-gray-400">"/"</span>
                                                                    <span class="font-medium">{cs.archetype_name.clone()}</span>
                                                                </div>
                                                            </div>
                                                        }
                                                    }).collect::<Vec<_>>()}
                                                </div>
                                            </div>
                                        </div>
                                    }.into_any()
                                }
                                None => view! {
                                    <div class="text-gray-500">"Loading commit details..."</div>
                                }.into_any(),
                            }
                        }}
                    </Suspense>
                </div>
            </div>
        </div>
    }
}

#[component]
fn CreateTagModal(
    commit_hash: String,
    on_close: impl Fn() + 'static,
    on_created: impl Fn(Tag) + 'static + Clone,
) -> impl IntoView {
    let (tag_name, set_tag_name) = signal(String::new());
    let (tag_message, set_tag_message) = signal(String::new());
    let (saving, set_saving) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);

    let short_hash = if commit_hash.len() >= 8 { commit_hash[..8].to_string() } else { commit_hash.clone() };
    let hash_for_create = commit_hash.clone();

    view! {
        <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
            <div class="bg-white rounded-lg shadow-xl max-w-md w-full mx-4">
                <div class="px-6 py-4 border-b border-gray-200 bg-gray-50">
                    <h3 class="text-lg font-semibold text-gray-800">"Create Tag"</h3>
                    <p class="text-sm text-gray-500 mt-1">
                        "For commit " <code class="font-mono text-blue-600">{short_hash}</code>
                    </p>
                </div>

                <form on:submit={
                    let on_created = on_created.clone();
                    move |ev: web_sys::SubmitEvent| {
                        ev.prevent_default();
                        let name = tag_name.get();
                        if name.is_empty() {
                            set_error.set(Some("Tag name is required".to_string()));
                            return;
                        }

                        set_saving.set(true);
                        set_error.set(None);

                        let h = hash_for_create.clone();
                        let msg = tag_message.get();
                        let msg_opt: Option<String> = if msg.is_empty() { None } else { Some(msg) };
                        let on_created = on_created.clone();

                        wasm_bindgen_futures::spawn_local(async move {
                            let msg_ref = msg_opt.as_deref();
                            match api::create_tag(&h, &name, msg_ref).await {
                                Ok(tag) => {
                                    on_created(tag);
                                }
                                Err(e) => {
                                    set_error.set(Some(e));
                                    set_saving.set(false);
                                }
                            }
                        });
                    }
                }>
                    <div class="p-6 space-y-4">
                        {move || error.get().map(|e| view! {
                            <div class="p-3 bg-red-50 border border-red-200 rounded text-red-700 text-sm">
                                {e}
                            </div>
                        })}

                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"Tag Name"</label>
                            <input
                                type="text"
                                class="w-full px-3 py-2 border border-gray-300 rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
                                placeholder="v1.0.0"
                                prop:value=move || tag_name.get()
                                on:input=move |ev| {
                                    let target = ev.target().unwrap();
                                    let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                    set_tag_name.set(input.value());
                                }
                            />
                        </div>

                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"Message (optional)"</label>
                            <textarea
                                rows=2
                                class="w-full px-3 py-2 border border-gray-300 rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
                                placeholder="Release notes..."
                                prop:value=move || tag_message.get()
                                on:input=move |ev| {
                                    let target = ev.target().unwrap();
                                    let textarea: web_sys::HtmlTextAreaElement = target.dyn_into().unwrap();
                                    set_tag_message.set(textarea.value());
                                }
                            />
                        </div>
                    </div>

                    <div class="px-6 py-4 border-t border-gray-200 bg-gray-50 flex justify-end gap-3">
                        <button
                            type="button"
                            class="px-4 py-2 text-gray-600 hover:text-gray-800"
                            on:click=move |_| on_close()
                        >
                            "Cancel"
                        </button>
                        <button
                            type="submit"
                            class="px-4 py-2 bg-yellow-500 text-white rounded hover:bg-yellow-600 disabled:opacity-50"
                            disabled=move || saving.get()
                        >
                            {move || if saving.get() { "Creating..." } else { "Create Tag" }}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    }
}
