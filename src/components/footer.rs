use crate::router::{Route, route_href};
use leptos::prelude::*;

#[component]
pub fn Footer() -> impl IntoView {
    view! {
        <footer>
            <div class="ft">
                <div class="ft-brand">
                    <a class="brand" href=route_href(Route::Home) style="font-size:18px">
                        <span class="logo"></span>"Face Crop Forge"
                    </a>
                    <p>"A browser-native image pipeline. Built for designers, ML engineers, and operators who'd rather not upload a face to a stranger's server."</p>
                </div>
                <div>
                    <h6>"Workflows"</h6>
                    <a href=route_href(Route::Batch)>"Batch"</a>
                    <a href=route_href(Route::Single)>"Single image"</a>
                    <a href=route_href(Route::Csv)>"CSV-driven"</a>
                </div>
                <div>
                    <h6>"Resources"</h6>
                    <a href=route_href(Route::Docs)>"Documentation"</a>
                    <a href="https://github.com/gregorycarnegie/face-crop-forge">"Source"</a>
                    <a href="https://github.com/gregorycarnegie/face-crop-forge/issues">"Issues"</a>
                </div>
                <div>
                    <h6>"Project"</h6>
                    <a href="https://github.com/gregorycarnegie/face-crop-forge">"github / face-crop-forge"</a>
                    <a href="https://github.com/gregorycarnegie/face-crop-forge/blob/main/LICENSE">"AGPL-3.0 license"</a>
                    <h6 style="margin-top:20px">"Desktop app"</h6>
                    <a href="https://facecropstudio.com/" target="_blank" rel="noopener">"facecropstudio.com"</a>
                    <a href="https://github.com/gregorycarnegie/face-crop-studio" target="_blank" rel="noopener">"github / face-crop-studio"</a>
                </div>
            </div>
            <div class="copyrow">
                <span>"(c) 2026 "<b>"Face Crop Forge"</b>" - local browser processing - AGPL-3.0"</span>
                <span>"Rust + Leptos + MediaPipe fallback"</span>
            </div>
        </footer>
    }
}
