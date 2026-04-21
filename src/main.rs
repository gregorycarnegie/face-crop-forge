#![warn(clippy::pedantic)]

mod app;
mod base_runtime;
mod base_ui;
mod batch_core;
mod batch_export;
#[cfg(test)]
mod crop_math;
mod csv_core;
mod export_runtime;
#[cfg(test)]
mod flow_tests;
mod mediapipe;
mod panels;
#[cfg(test)]
mod perf_snapshot;
#[cfg(test)]
mod preprocessing;
#[cfg(test)]
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
