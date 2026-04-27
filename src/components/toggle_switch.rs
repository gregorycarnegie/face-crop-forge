use leptos::prelude::*;

#[component]
pub fn ToggleSwitch(
    #[prop(into)] label: String,
    #[prop(optional, into)] desc: Option<String>,
    #[prop(default = false)] initially_on: bool,
    #[prop(optional, into)] accent: String, // "cyan", "lime", default is peach
) -> impl IntoView {
    let (on, set_on) = signal(initially_on);

    let switch_class = move || {
        let mut classes = vec!["switch"];
        if on.get() {
            classes.push("on");
            if !accent.is_empty() {
                classes.push(&accent);
            }
        }
        classes.join(" ")
    };

    view! {
        <div class="toggle-row">
            <div>
                <div>{label}</div>
                {desc.map(|d| view! { <div class="desc">{d}</div> })}
            </div>
            <span class=switch_class on:click=move |_| set_on.update(|o| *o = !*o)></span>
        </div>
    }
}
