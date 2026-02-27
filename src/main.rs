mod app;
mod base_core;
mod base_runtime;
mod base_ui;
mod batch_core;
mod batch_export;
mod crop_math;
mod csv_core;
mod domain_types;
mod export_runtime;
#[cfg(test)]
mod flow_tests;
mod history;
mod mediapipe;
mod panels;
#[cfg(test)]
mod perf_snapshot;
mod preprocessing;
mod quality_filters;
mod single_core;
mod state;
mod worker_bridge;

use app::App;
use leptos::mount::mount_to_body;
use leptos::prelude::*;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App /> });
}
