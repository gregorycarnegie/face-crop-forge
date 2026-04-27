use leptos::prelude::*;

#[component]
pub fn StatCard(
    #[prop(into)] key: String,
    #[prop(into)] value: String,
    #[prop(optional, into)] suffix: String,
    #[prop(optional, into)] sub: String,
    #[prop(optional, into)] accent: String, // "peach", "cyan", "lime", "rose"
    #[prop(default = false)] custom_value_size: bool, // e.g. 18px or so, handled explicitly
) -> impl IntoView {
    let s_class = if accent.is_empty() {
        "s".to_string()
    } else {
        format!("s {}", accent)
    };

    view! {
        <div class=s_class>
            <div class="k">{key}</div>
            <div class="v" style=move || if custom_value_size { "font-size:18px" } else { "" }>
                <em>{value}</em>
                {if !suffix.is_empty() {
                    Some(view! { <span style="font-size:14px;color:var(--ink-3)">{suffix.clone()}</span> })
                } else {
                    None
                }}
            </div>
            {if !sub.is_empty() {
                Some(view! { <div class="sub">{sub.clone()}</div> })
            } else {
                None
            }}
        </div>
    }
}
