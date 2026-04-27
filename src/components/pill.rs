use leptos::prelude::*;

#[component]
pub fn Pill(
    #[prop(optional, into)] variant: String, // "ok", "warn", "run", "peach", empty
    #[prop(optional)] has_dot: bool,
    children: Children,
) -> impl IntoView {
    let class_str = if variant.is_empty() {
        "pill".to_string()
    } else {
        format!("pill {}", variant)
    };

    view! {
        <span class=class_str>
            {if has_dot {
                match variant.as_str() {
                    "peach" => Some(view! { <span class="dot"></span> }), // old style
                    _ => Some(view! { <span class="d"></span> }),         // new style
                }
            } else {
                None
            }}
            {children()}
        </span>
    }
}

#[component]
pub fn Badge(
    #[prop(into)] variant: String, // "ok", "run", "miss", "queued", "fail"
    children: Children,
) -> impl IntoView {
    let class_str = format!("badge {}", variant);
    view! {
        <span class=class_str>
            {children()}
        </span>
    }
}
