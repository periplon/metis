use leptos::prelude::*;
use leptos_router::components::{Router, Route, Routes, A};
use leptos_router::path;

mod api;
mod types;
mod components;

use components::dashboard::Dashboard;
use components::config::Config;
use components::logs::Logs;
use components::resources::{Resources, ResourceForm};
use components::tools::{Tools, ToolForm};
use components::prompts::{Prompts, PromptForm};
use components::workflows::{Workflows, WorkflowForm};

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <div class="flex h-screen bg-gray-100">
                // Sidebar
                <div class="w-64 bg-gray-800 text-white p-4 flex flex-col">
                    <h1 class="text-2xl font-bold mb-8">"Metis"</h1>
                    <nav class="space-y-1 flex-1">
                        <NavLink href="/" label="Dashboard" />
                        <NavLink href="/resources" label="Resources" />
                        <NavLink href="/tools" label="Tools" />
                        <NavLink href="/prompts" label="Prompts" />
                        <NavLink href="/workflows" label="Workflows" />
                        <div class="border-t border-gray-700 my-4"></div>
                        <NavLink href="/config" label="Configuration" />
                        <NavLink href="/logs" label="Logs" />
                    </nav>
                    <div class="text-xs text-gray-500 mt-4">
                        "Metis MCP Mock Server"
                    </div>
                </div>

                // Main Content
                <div class="flex-1 overflow-y-auto">
                    <Routes fallback=|| "Not found.">
                        <Route path=path!("/") view=Dashboard/>
                        <Route path=path!("/resources/new") view=ResourceForm/>
                        <Route path=path!("/resources") view=Resources/>
                        <Route path=path!("/tools/new") view=ToolForm/>
                        <Route path=path!("/tools") view=Tools/>
                        <Route path=path!("/prompts/new") view=PromptForm/>
                        <Route path=path!("/prompts") view=Prompts/>
                        <Route path=path!("/workflows/new") view=WorkflowForm/>
                        <Route path=path!("/workflows") view=Workflows/>
                        <Route path=path!("/config") view=Config/>
                        <Route path=path!("/logs") view=Logs/>
                    </Routes>
                </div>
            </div>
        </Router>
    }
}

#[component]
fn NavLink(href: &'static str, label: &'static str) -> impl IntoView {
    view! {
        <A href=href attr:class="block p-2 hover:bg-gray-700 rounded transition-colors">
            {label}
        </A>
    }
}

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}
