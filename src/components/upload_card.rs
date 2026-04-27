use leptos::prelude::*;

#[component]
pub fn UploadCard(
    #[prop(optional, into)] variant: String, // "single", "batch", "csv-up", "img-up"
    #[prop(into)] icon_svg: String,
    #[prop(into)] title: String,
    #[prop(into)] desc: String,
    #[prop(into)] actions: Vec<String>,
    #[prop(optional)] file_tag: Option<String>,
    #[prop(optional)] children: Option<Children>, // for stat-row or breakdown
) -> impl IntoView {
    let class_str = if variant.is_empty() {
        "upload-card".to_string()
    } else {
        format!("upload-card {}", variant)
    };

    view! {
        <div class=class_str>
            <div class="icon" inner_html=icon_svg></div>
            <h4>{title}</h4>
            <p inner_html=desc></p>
            <div class="actions">
                {actions.into_iter().map(|action| {
                    view! { <span class="chip">{action}</span> }
                }).collect::<Vec<_>>()}
            </div>
            {if let Some(tag) = file_tag {
                Some(view! { <div class="file-tag" inner_html=tag></div> })
            } else {
                None
            }}
            {children.map(|c| c())}
        </div>
    }
}
