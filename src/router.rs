use leptos::prelude::*;
use wasm_bindgen::JsValue;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Route {
    Home,
    Batch,
    Single,
    Csv,
}

impl Route {
    pub fn path(self) -> &'static str {
        match self {
            Route::Home => "/",
            Route::Batch => "/batch",
            Route::Single => "/single",
            Route::Csv => "/csv",
        }
    }
}

pub fn route_from_path(path: &str) -> Route {
    match path.trim_matches('/') {
        "batch" => Route::Batch,
        "single" => Route::Single,
        "csv" => Route::Csv,
        _ => Route::Home,
    }
}

pub fn current_route() -> Route {
    web_sys::window()
        .and_then(|window| window.location().pathname().ok())
        .map(|path| route_from_path(&path))
        .unwrap_or(Route::Home)
}

pub fn push_route(route: Route) {
    if let Some(window) = web_sys::window() {
        if let Ok(history) = window.history() {
            let _ = history.push_state_with_url(&JsValue::NULL, "", Some(route.path()));
        }
    }
}

pub fn navigate(route: Route, set_route: WriteSignal<Route>) {
    set_route.set(route);
    push_route(route);

    // Optional: Scroll to top on navigation if needed
    if let Some(window) = web_sys::window() {
        window.scroll_to_with_x_and_y(0.0, 0.0);
    }
}

pub fn setup_popstate_listener(set_route: WriteSignal<Route>) {
    if let Some(window) = web_sys::window() {
        use wasm_bindgen::JsCast;
        use wasm_bindgen::closure::Closure;

        let handler = move |_: web_sys::Event| {
            set_route.set(current_route());
        };

        let closure = Closure::wrap(Box::new(handler) as Box<dyn FnMut(_)>);
        let _ =
            window.add_event_listener_with_callback("popstate", closure.as_ref().unchecked_ref());

        // Prevent JS from garbage collecting the closure
        closure.forget();
    }
}
