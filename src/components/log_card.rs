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

#[component]
pub fn LogLine(
    #[prop(into)] time: String,
    #[prop(into)] message: String,
    #[prop(optional, into)] variant: String, // "ok", "warn", "err", or empty
) -> impl IntoView {
    let line_class = if variant.is_empty() {
        "line".to_string()
    } else {
        format!("line {}", variant)
    };

    view! {
        <div class=line_class>
            <span class="t">{time}</span>
            <span class="m">{message}</span>
        </div>
    }
}
