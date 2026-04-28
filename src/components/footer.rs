use leptos::prelude::*;

#[component]
pub fn Footer() -> impl IntoView {
    view! {
        <footer>
            <div class="ft">
                <div class="ft-brand">
                    <a class="brand" href="/" style="font-size:18px">
                        <span class="logo"></span>"Face Crop Forge"
                    </a>
                    <p>"A browser-native image pipeline. Built for designers, ML engineers, and operators who'd rather not upload a face to a stranger's server."</p>
                </div>
                <div>
                    <h6>"Workflows"</h6>
                    <a href="/batch">"Batch"</a>
                    <a href="/single">"Single image"</a>
                    <a href="/csv">"CSV-driven"</a>
                </div>
                <div>
                    <h6>"Resources"</h6>
                    <a href="/docs">"Documentation"</a>
                    <a href="https://github.com/gregorycarnegie/face-crop-forge">"Source"</a>
                    <a href="https://github.com/gregorycarnegie/face-crop-forge/issues">"Issues"</a>
                </div>
                <div>
                    <h6>"Project"</h6>
                    <a href="https://github.com/gregorycarnegie/face-crop-forge">"github / face-crop-forge"</a>
                    <a href="https://github.com/gregorycarnegie/face-crop-forge/blob/main/LICENSE">"AGPL-3.0 license"</a>
                </div>
            </div>
            <div class="copyrow">
                <span>"(c) 2026 "<b>"Face Crop Forge"</b>" - local browser processing - AGPL-3.0"</span>
                <span>"Rust + Leptos + MediaPipe fallback"</span>
            </div>
        </footer>
    }
}
