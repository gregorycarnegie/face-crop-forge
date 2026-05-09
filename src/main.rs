mod base_runtime;
mod base_ui;
mod batch_core;
mod batch_export;
mod components;
mod csv_core;
mod export_runtime;
mod mediapipe;
mod pages;
mod router;
mod runtime;
mod single_core;
mod state;
mod worker_bridge;

#[cfg(test)]
mod e2e_tests;

use leptos::prelude::*;
use pages::batch::Batch;
use pages::csv::Csv;
use pages::docs::Docs;
use pages::donate::Donate;
use pages::home::Home;
use pages::single::Single;
use router::{Route, current_route, setup_popstate_listener};
use state::AppState;

#[component]
fn App() -> impl IntoView {
    provide_context(AppState::new());

    // Top-level routing state
    let (route, set_route) = signal(current_route());

    // Listen to browser back/forward buttons
    setup_popstate_listener(set_route);

    // Apply the body class based on the current route
    Effect::new(move |_| {
        if let Some(window) = web_sys::window()
            && let Some(document) = window.document()
            && let Some(body) = document.body()
        {
            let path_class = match route.get() {
                Route::Home => "home-page",
                Route::Batch => "batch-page",
                Route::Single => "single-page",
                Route::Csv => "csv-page",
                Route::Docs => "docs-page",
                Route::Donate => "donate-page",
            };
            body.set_class_name(path_class);
        }
    });

    view! {
        <div class="shell">
            {move || match route.get() {
                Route::Home => view! { <Home route=route.get() set_route=set_route /> }.into_any(),
                Route::Batch => view! { <Batch route=route.get() set_route=set_route /> }.into_any(),
                Route::Single => view! { <Single route=route.get() set_route=set_route /> }.into_any(),
                Route::Csv => view! { <Csv route=route.get() set_route=set_route /> }.into_any(),
                Route::Docs => view! { <Docs route=route.get() set_route=set_route /> }.into_any(),
                Route::Donate => view! { <Donate route=route.get() set_route=set_route /> }.into_any(),
            }}
        </div>
    }
}

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}
