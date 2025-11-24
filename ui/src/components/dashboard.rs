use leptos::prelude::*;

#[component]
pub fn Dashboard() -> impl IntoView {
    view! {
        <div class="p-4">
            <h2 class="text-xl font-bold mb-4">"Dashboard"</h2>
            <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
                <div class="bg-white p-4 rounded shadow">
                    <h3 class="font-bold text-gray-500">"Requests / sec"</h3>
                    <p class="text-2xl">"0"</p>
                </div>
                <div class="bg-white p-4 rounded shadow">
                    <h3 class="font-bold text-gray-500">"Active Connections"</h3>
                    <p class="text-2xl">"0"</p>
                </div>
                <div class="bg-white p-4 rounded shadow">
                    <h3 class="font-bold text-gray-500">"Uptime"</h3>
                    <p class="text-2xl">"0s"</p>
                </div>
            </div>
        </div>
    }
}
