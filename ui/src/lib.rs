use leptos::prelude::*;
use leptos_router::components::{Router, Route, Routes, A};
use leptos_router::path;

mod components;
use components::dashboard::Dashboard;
use components::config::Config;
use components::logs::Logs;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <div class="flex h-screen bg-gray-100">
                // Sidebar
                <div class="w-64 bg-gray-800 text-white p-4">
                    <h1 class="text-2xl font-bold mb-8">"Metis"</h1>
                    <nav class="space-y-2">
                        <A href="/" attr:class="block p-2 hover:bg-gray-700 rounded">"Dashboard"</A>
                        <A href="/config" attr:class="block p-2 hover:bg-gray-700 rounded">"Configuration"</A>
                        <A href="/logs" attr:class="block p-2 hover:bg-gray-700 rounded">"Logs"</A>
                    </nav>
                </div>

                // Main Content
                <div class="flex-1 overflow-y-auto">
                    <Routes fallback=|| "Not found.">
                        <Route path=path!("/") view=Dashboard/>
                        <Route path=path!("/config") view=Config/>
                        <Route path=path!("/logs") view=Logs/>
                    </Routes>
                </div>
            </div>
        </Router>
    }
}

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}
