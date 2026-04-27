use leptos::prelude::*;

#[component]
pub fn SliderField(
    #[prop(into)] label: String,
    #[prop(into)] prefix: String,
    #[prop(into)] suffix: String,
    #[prop(default = 0)] min: i32,
    #[prop(default = 100)] max: i32,
    #[prop(default = 50)] initial: i32,
) -> impl IntoView {
    let (value, set_value) = signal(initial);

    // Provide a random unique id for the slider mapping just not to collision (ideally use passed ID but here string is fine)

    view! {
        <div class="field">
            <label>{label} " · " {move || value.get().to_string()} {suffix.clone()}</label>
            <div class="slider-row">
                <input
                    type="range"
                    class="slider"
                    min=min
                    max=max
                    prop:value=move || value.get().to_string()
                    on:input=move |ev| {
                        if let Ok(v) = event_target_value(&ev).parse::<i32>() {
                            set_value.set(v);
                        }
                    }
                />
                <span class="num">{prefix}{move || value.get().to_string()}{suffix.clone()}</span>
            </div>
        </div>
    }
}
