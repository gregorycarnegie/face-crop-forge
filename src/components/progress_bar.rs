use leptos::prelude::*;

#[component]
pub fn ProgressBar(#[prop(into)] progress: RwSignal<u32>, // percentage 0-100
) -> impl IntoView {
    view! {
        <div class="ptrack">
            <i style=move || format!("width: {}%", progress.get())></i>
        </div>
    }
}
