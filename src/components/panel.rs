use leptos::prelude::*;

#[component]
pub fn Panel(
    #[prop(into)] title: String,
    #[prop(into)] num: String,
    #[prop(optional, into)] accent: String, // "peach", "cyan", "lime", or empty
    #[prop(default = true)] initially_open: bool,
    #[prop(optional, into)] badge: Option<String>,
    children: Children,
) -> impl IntoView {
    let (open, set_open) = signal(initially_open);

    let panel_class = move || {
        let mut classes = vec!["panel"];
        if open.get() {
            classes.push("open");
        }
        if !accent.is_empty() {
            classes.push(&accent);
        }
        classes.join(" ")
    };

    view! {
        <div class=panel_class>
            <div class="p-head" on:click=move |_| set_open.update(|o| *o = !*o)>
                <h3>
                    <span class="num">{num}</span>
                    {title}
                    {badge.map(|b| view! { <span class="chip" style="font-size:10px;padding:2px 6px">{b}</span> })}
                </h3>
                <span class="chev">"▶"</span>
            </div>
            <div class="p-body">
                {children()}
            </div>
        </div>
    }
}
