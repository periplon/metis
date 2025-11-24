use leptos::prelude::*;

#[component]
pub fn Logs() -> impl IntoView {
    view! {
        <div class="p-4">
            <h2 class="text-xl font-bold mb-4">"Logs"</h2>
            <div class="bg-black text-green-400 p-4 rounded shadow h-64 overflow-y-auto font-mono">
                <p>"[INFO] Server started..."</p>
            </div>
        </div>
    }
}
