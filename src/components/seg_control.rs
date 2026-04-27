use leptos::prelude::*;

#[component]
pub fn SegControl(
    options: Vec<String>,
    #[prop(default = 0)] initial_index: usize,
    #[prop(optional, into)] accent: String, // "cyan", "lime", default is peach
) -> impl IntoView {
    let (selected, set_selected) = signal(initial_index);

    let cols_class = match options.len() {
        3 => "seg cols-3",
        4 => "seg cols-4",
        _ => "seg",
    };

    view! {
        <div class=cols_class>
            {options.into_iter().enumerate().map(|(i, opt)| {
                let accent_class = accent.clone();
                let class_name = move || {
                    if selected.get() == i {
                        if accent_class.is_empty() {
                            "on".to_string()
                        } else {
                            format!("on {}", accent_class)
                        }
                    } else {
                        "".to_string()
                    }
                };
                view! {
                    <button class=class_name on:click=move |_| set_selected.set(i)>
                        {opt}
                    </button>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
}
