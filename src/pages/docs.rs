use crate::components::footer::Footer;
use crate::components::topbar::Topbar;
use crate::router::{Route, navigate};
use leptos::prelude::*;

#[component]
pub fn Docs(route: Route, set_route: WriteSignal<Route>) -> impl IntoView {
    view! {
        <Topbar route set_route />

        <div class="docs-page-wrap">
            <section class="docs-hero">
                <div>
                    <div class="crumb"><span>"Resources"</span><span class="dot"></span><span class="now">"Documentation"</span></div>
                    <h1>"Use Face Crop Forge without uploading images."</h1>
                    <p>"Face Crop Forge runs in the browser. It detects faces locally, crops selected images with your settings, and exports individual files or ZIP archives."</p>
                    <div class="hero-cta">
                        <a class="btn-primary" href="/batch" on:click=move |ev| { ev.prevent_default(); navigate(Route::Batch, set_route); }>
                            "Open Batch"
                        </a>
                        <a class="btn-ghost" href="/single" on:click=move |ev| { ev.prevent_default(); navigate(Route::Single, set_route); }>
                            "Open Single"
                        </a>
                    </div>
                </div>
                <div class="docs-summary">
                    <div><span>"Runtime"</span><b>"Browser + WASM"</b></div>
                    <div><span>"Detection"</span><b>"FaceDetector, then MediaPipe"</b></div>
                    <div><span>"Exports"</span><b>"JPG, PNG, WEBP, ZIP"</b></div>
                    <div><span>"License"</span><b>"AGPL-3.0"</b></div>
                </div>
            </section>

            <section class="docs-grid">
                <article class="doc-card">
                    <span class="doc-k">"Workflow 01"</span>
                    <h2>"Batch processing"</h2>
                    <p>"Use Batch when you have a set of image files and want one crop per selected image."</p>
                    <ul>
                        <li>"Drop files or a folder, where supported by the browser."</li>
                        <li>"Select the images to process from the queue."</li>
                        <li>"Set crop aspect ratio, padding, confidence threshold, dimensions, and output format."</li>
                        <li>"Process locally, preview generated crops, then download a ZIP."</li>
                    </ul>
                </article>

                <article class="doc-card">
                    <span class="doc-k">"Workflow 02"</span>
                    <h2>"Single image"</h2>
                    <p>"Use Single when you need to inspect one source image and choose which detected faces to export."</p>
                    <ul>
                        <li>"Load an image or capture a frame from the webcam."</li>
                        <li>"Click detected face boxes to include or exclude them."</li>
                        <li>"Adjust padding, output dimensions, aspect ratio, and format."</li>
                        <li>"Save selected face crops as individual files."</li>
                    </ul>
                </article>

                <article class="doc-card">
                    <span class="doc-k">"Workflow 03"</span>
                    <h2>"CSV-driven output"</h2>
                    <p>"Use CSV when output filenames must come from a spreadsheet or manifest."</p>
                    <ul>
                        <li>"Upload a CSV with headers."</li>
                        <li>"Map one column to source filenames and one column to output names."</li>
                        <li>"Upload matching image files."</li>
                        <li>"Process matched rows and download a ZIP named from the mapped values."</li>
                    </ul>
                </article>

                <article class="doc-card">
                    <span class="doc-k">"Runtime"</span>
                    <h2>"Detection behavior"</h2>
                    <p>"The app tries the browser's native FaceDetector API first. If that is unavailable or fails, it loads the bundled MediaPipe Tasks assets from the local models directory."</p>
                    <ul>
                        <li>"All image input, previews, crop rendering, and downloads happen in the browser."</li>
                        <li>"The Single page shows the active detection backend in its status pills."</li>
                        <li>"Browsers without supported detection APIs may report a detection failure."</li>
                    </ul>
                </article>

                <article class="doc-card">
                    <span class="doc-k">"Output"</span>
                    <h2>"Files and naming"</h2>
                    <p>"Crop rendering uses browser canvas export and normalizes filenames to match the selected MIME type."</p>
                    <ul>
                        <li>"Supported crop formats are JPG, PNG, and WEBP."</li>
                        <li>"Batch and CSV workflows export ZIP archives."</li>
                        <li>"CSV templates can use {csv_name}, {original}, {index}, {width}, {height}, and {timestamp}."</li>
                    </ul>
                </article>

                <article class="doc-card">
                    <span class="doc-k">"Development"</span>
                    <h2>"Run locally"</h2>
                    <p>"The project is a Rust + Leptos CSR app built with Trunk. No Node toolchain is required."</p>
                    <div class="codeblock">
                        <code>"rustup target add wasm32-unknown-unknown"</code>
                        <code>"cargo install trunk"</code>
                        <code>"trunk serve"</code>
                    </div>
                </article>
            </section>
        </div>

        <Footer />
    }
}
