use leptos::*;
use leptos_router::*;

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
                        <A href="/" class="block p-2 hover:bg-gray-700 rounded">"Dashboard"</A>
                        <A href="/config" class="block p-2 hover:bg-gray-700 rounded">"Configuration"</A>
                        <A href="/logs" class="block p-2 hover:bg-gray-700 rounded">"Logs"</A>
                    </nav>
                </div>

                // Main Content
                <div class="flex-1 overflow-y-auto">
                    <Routes>
                        <Route path="/" view=Dashboard/>
                        <Route path="/config" view=Config/>
                        <Route path="/logs" view=Logs/>
                    </Routes>
                </div>
            </div>
        </Router>
    }
}

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App/> })
}
