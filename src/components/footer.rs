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
                    <a href="/api">"Headless"</a>
                </div>
                <div>
                    <h6>"Resources"</h6>
                    <a href="/docs">"Documentation"</a>
                    <a href="/changelog">"Changelog"</a>
                    <a href="/benchmarks">"Benchmarks"</a>
                    <a href="/source">"Source"</a>
                </div>
                <div>
                    <h6>"Contact"</h6>
                    <a href="#">"github / facecropforge"</a>
                    <a href="#">"hello@facecropforge"</a>
                    <a href="#">"issues"</a>
                </div>
            </div>
            <div class="copyrow">
                <span>"© 2026 "<b>"Face Crop Forge"</b>" · Zero data retention · MIT licensed"</span>
                <span>"Build "<b>"a3f9c"</b>" · WebGPU + WASM SIMD"</span>
            </div>
        </footer>
    }
}
