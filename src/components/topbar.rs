use crate::router::{Route, navigate};
use leptos::prelude::*;
use web_sys::MouseEvent;

#[component]
pub fn Topbar(route: Route, set_route: WriteSignal<Route>) -> impl IntoView {
    let handle_nav = move |ev: MouseEvent, target_route: Route| {
        ev.prevent_default();
        navigate(target_route, set_route);
    };

    let active_class = move |target: Route| {
        if route == target { "active" } else { "" }
    };

    view! {
        <header class="topbar">
            <a class="brand" href="/" on:click=move |ev| handle_nav(ev, Route::Home)>
                <span class="logo"></span>"Face Crop Forge"
            </a>
            <nav class="nav" aria-label="primary">
                <a href="/" class=move || active_class(Route::Home) on:click=move |ev| handle_nav(ev, Route::Home)>"Home"</a>
                <a href="/batch" class=move || active_class(Route::Batch) on:click=move |ev| handle_nav(ev, Route::Batch)>"Batch"</a>
                <a href="/single" class=move || active_class(Route::Single) on:click=move |ev| handle_nav(ev, Route::Single)>"Single"</a>
                <a href="/csv" class=move || active_class(Route::Csv) on:click=move |ev| handle_nav(ev, Route::Csv)>"CSV"</a>
                <a href="/docs">"Docs"</a>
            </nav>
            <div class="top-r">
                <a class="ghost" href="/source">"GitHub →"</a>
                {move || {
                    let (href, label, r) = match route {
                        Route::Home | Route::Csv => ("/batch", "Open Batch", Route::Batch),
                        Route::Batch => ("/single", "Single image", Route::Single),
                        Route::Single => ("/batch", "Open Batch", Route::Batch),
                    };
                    view! {
                        <a class="pri" href=href on:click=move |ev| handle_nav(ev, r)>
                            {label}
                        </a>
                    }
                }}
            </div>
        </header>
    }
}
