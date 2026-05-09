use crate::router::{Route, navigate, route_href};
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
            <a class="brand" href=route_href(Route::Home) on:click=move |ev| handle_nav(ev, Route::Home)>
                <span class="logo"></span>"Face Crop Forge"
            </a>
            <nav class="nav" aria-label="primary">
                <a href=route_href(Route::Home) class=move || active_class(Route::Home) on:click=move |ev| handle_nav(ev, Route::Home)>"Home"</a>
                <a href=route_href(Route::Batch) class=move || active_class(Route::Batch) on:click=move |ev| handle_nav(ev, Route::Batch)>"Batch"</a>
                <a href=route_href(Route::Single) class=move || active_class(Route::Single) on:click=move |ev| handle_nav(ev, Route::Single)>"Single"</a>
                <a href=route_href(Route::Csv) class=move || active_class(Route::Csv) on:click=move |ev| handle_nav(ev, Route::Csv)>"CSV"</a>
                <a href=route_href(Route::Docs) class=move || active_class(Route::Docs) on:click=move |ev| handle_nav(ev, Route::Docs)>"Docs"</a>
                <a href=route_href(Route::Donate) class=move || active_class(Route::Donate) on:click=move |ev| handle_nav(ev, Route::Donate)>"Donate"</a>
            </nav>
            <div class="top-r">
                <a class="ghost" href="https://github.com/gregorycarnegie/face-crop-forge">"GitHub ->"</a>
                {move || {
                    let (label, r) = match route {
                        Route::Home | Route::Csv => ("Open Batch", Route::Batch),
                        Route::Batch => ("Single image", Route::Single),
                        Route::Single => ("Open Batch", Route::Batch),
                        Route::Docs | Route::Donate => ("Open Batch", Route::Batch),
                    };
                    view! {
                        <a class="pri" href=route_href(r) on:click=move |ev| handle_nav(ev, r)>
                            {label}
                        </a>
                    }
                }}
            </div>
        </header>
    }
}
