use super::{
    AppShell, ClassAttribute, CropSettingsPanel, CustomAttribute, ElementChild, IntoView,
    OnAttribute, OutputSettingsBatchPanel, OutputSettingsCsvPanel, PreprocessingSettingsPanel,
    StyleAttribute, component, view,
};

#[component]
pub(crate) fn PanelsGalleryPage() -> impl IntoView {
    view! {
        <AppShell title="Leptos Panels Gallery">
            <div class="app-shell" style="padding-top: 24px;">
                <div class="app-body">
                    <aside class="control-panel">
                        <div class="control-scroll">
                            <CropSettingsPanel />
                            <PreprocessingSettingsPanel />
                            <OutputSettingsBatchPanel />
                            <OutputSettingsCsvPanel />
                        </div>
                    </aside>
                </div>
            </div>
        </AppShell>
    }
}

#[component]
pub(crate) fn LandingPage() -> impl IntoView {
    view! {
        <AppShell title="Face Crop Forge">
            <div class="home-container">
                <div class="background-blobs">
                    <div class="blob blob-1"></div>
                    <div class="blob blob-2"></div>
                </div>

                <header class="hero-section">
                    <div class="hero-badge">"v1.7.0 • 100% Client-Side Privacy"</div>
                    <h1 class="hero-title">
                        "Face Crop "
                        <span class="gradient-text">"Forge"</span>
                    </h1>
                    <p class="hero-subtitle">
                        "The ultimate tool for batch face detection and cropping. Process thousands of images securely in your browser without uploading a single file."
                    </p>
                    <div class="hero-stats">
                        <div class="stat">
                            <span class="stat-number">"0s"</span>
                            <span class="stat-label">"Upload Time"</span>
                        </div>
                        <div class="stat-divider"></div>
                        <div class="stat">
                            <span class="stat-number">"100%"</span>
                            <span class="stat-label">"Private"</span>
                        </div>
                        <div class="stat-divider"></div>
                        <div class="stat">
                            <span class="stat-number">"Free"</span>
                            <span class="stat-label">"Open Source"</span>
                        </div>
                    </div>
                </header>

                <main class="modes-grid">
                    <ModeCard
                        href="batch"
                        title="Batch Processing"
                        description="The powerhouse mode. Drag and drop folders of images and process them all at once."
                        action="Launch Batch Mode"
                        featured=true
                        icon_path="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"
                        feature_1="Bulk processing"
                        feature_2="ZIP export"
                        feature_3="Smart centering"
                    />
                    <ModeCard
                        href="single"
                        title="Single Image"
                        description="Perfect for fine-tuning. Adjust crops, rotate, and inspect detections one by one."
                        action="Launch Single Mode"
                        featured=false
                        icon_path="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z"
                        feature_1="Real-time preview"
                        feature_2="Manual adjustments"
                        feature_3="Webcam support"
                    />
                    <ModeCard
                        href="csv"
                        title="CSV Workflow"
                        description="Enterprise workflow. Map CSV data to filenames for automated organization."
                        action="Launch CSV Mode"
                        featured=false
                        icon_path="M9 17v-2m3 2v-4m3 4v-6m2 10H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
                        feature_1="Data mapping"
                        feature_2="Custom naming"
                        feature_3="Bulk organization"
                    />
                </main>

                <footer class="footer">
                    <div class="footer-content">
                        <p>
                            "Built with "
                            <a href="https://ai.google.dev/edge/mediapipe/solutions/guide" target="_blank" class="footer-link">
                                "MediaPipe Vision"
                            </a>
                        </p>
                        <div class="footer-links">
                            <a href="https://github.com/gregorycarnegie/face-crop-forge" target="_blank" rel="noopener">
                                "GitHub"
                            </a>
                        </div>
                    </div>
                </footer>
            </div>
        </AppShell>
    }
}

#[component]
pub(crate) fn ModeCard(
    href: &'static str,
    title: &'static str,
    description: &'static str,
    action: &'static str,
    featured: bool,
    icon_path: &'static str,
    feature_1: &'static str,
    feature_2: &'static str,
    feature_3: &'static str,
) -> impl IntoView {
    let class_name = if featured {
        "mode-card featured"
    } else {
        "mode-card"
    };
    let click_title = title;

    view! {
        <a
            href=href
            class=class_name
            on:click=move |_| {
                leptos::logging::log!("User selected: {} mode", click_title);
            }
        >
            <div class="card-glow"></div>
            <div class="mode-icon-wrapper">
                <svg class="mode-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d=icon_path></path>
                </svg>
            </div>
            <h2 class="mode-title">{title}</h2>
            <p class="mode-description">{description}</p>
            <ul class="mode-features">
                <li>{feature_1}</li>
                <li>{feature_2}</li>
                <li>{feature_3}</li>
            </ul>
            <span class="mode-action">
                {action}
                <span class="arrow">"->"</span>
            </span>
        </a>
    }
}
