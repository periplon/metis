use leptos::prelude::*;
use crate::api;
use crate::types::Resource;

#[component]
pub fn Resources() -> impl IntoView {
    let resources = LocalResource::new(|| async move {
        api::list_resources().await.ok()
    });

    view! {
        <div class="p-6">
            <div class="flex justify-between items-center mb-6">
                <h2 class="text-2xl font-bold">"Resources"</h2>
                <a href="/resources/new" class="bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded">
                    "+ New Resource"
                </a>
            </div>

            <Suspense fallback=move || view! { <div class="text-gray-500">"Loading..."</div> }>
                {move || {
                    resources.get().map(|data| {
                        match data {
                            Some(list) if !list.is_empty() => view! {
                                <div class="bg-white rounded-lg shadow overflow-hidden">
                                    <table class="min-w-full divide-y divide-gray-200">
                                        <thead class="bg-gray-50">
                                            <tr>
                                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Name"</th>
                                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"URI"</th>
                                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Type"</th>
                                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Strategy"</th>
                                                <th class="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">"Actions"</th>
                                            </tr>
                                        </thead>
                                        <tbody class="bg-white divide-y divide-gray-200">
                                            {list.into_iter().map(|resource| {
                                                view! { <ResourceRow resource=resource /> }
                                            }).collect::<Vec<_>>()}
                                        </tbody>
                                    </table>
                                </div>
                            }.into_any(),
                            Some(_) => view! {
                                <div class="text-center py-12 bg-white rounded-lg shadow">
                                    <p class="text-gray-500 mb-4">"No resources configured"</p>
                                    <a href="/resources/new" class="text-blue-500 hover:underline">"Create your first resource"</a>
                                </div>
                            }.into_any(),
                            None => view! {
                                <div class="text-red-500">"Failed to load resources"</div>
                            }.into_any(),
                        }
                    })
                }}
            </Suspense>
        </div>
    }
}

#[component]
fn ResourceRow(resource: Resource) -> impl IntoView {
    let strategy = resource.mock.as_ref()
        .map(|m| format!("{:?}", m.strategy))
        .unwrap_or_else(|| if resource.content.is_some() { "Static".to_string() } else { "-".to_string() });

    let mime = resource.mime_type.clone().unwrap_or_else(|| "-".to_string());

    view! {
        <tr class="hover:bg-gray-50">
            <td class="px-6 py-4 whitespace-nowrap">
                <div class="font-medium text-gray-900">{resource.name.clone()}</div>
                <div class="text-sm text-gray-500">{resource.description.clone().unwrap_or_default()}</div>
            </td>
            <td class="px-6 py-4 whitespace-nowrap text-sm font-mono text-gray-600">
                {resource.uri.clone()}
            </td>
            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                {mime}
            </td>
            <td class="px-6 py-4 whitespace-nowrap">
                <span class="px-2 py-1 text-xs font-semibold rounded-full bg-blue-100 text-blue-800">
                    {strategy}
                </span>
            </td>
            <td class="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
                <button class="text-blue-600 hover:text-blue-900 mr-3">"Edit"</button>
                <button class="text-red-600 hover:text-red-900">"Delete"</button>
            </td>
        </tr>
    }
}
