use leptos::prelude::*;

#[component]
pub fn LogCard(
    #[prop(into)] title: String,
    #[prop(into)] accent: String, // "peach", "cyan", "lime"
    #[prop(into)] meta_left: String,
    #[prop(into)] meta_right: String,
    children: Children,
) -> impl IntoView {
    let b_class = accent.clone();
    view! {
        <div class="log-card">
            <div class="log-head">
                <h3>{title} " " <b class=b_class>"log"</b></h3>
                <div class="meta">
                    <span>{meta_left}</span>
                    {if !meta_right.is_empty() {
                        Some(view! { <span>{meta_right}</span> })
                    } else {
                        None
                    }}
                </div>
            </div>
            <div class="log-body">
                {children()}
            </div>
        </div>
    }
}
