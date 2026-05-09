use leptos::prelude::*;
use wasm_bindgen::JsValue;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Route {
    Home,
    Batch,
    Single,
    Csv,
    Docs,
    Donate,
}

impl Route {
    pub const fn path(self) -> &'static str {
        match self {
            Route::Home => "/",
            Route::Batch => "/batch",
            Route::Single => "/single",
            Route::Csv => "/csv",
            Route::Docs => "/docs",
            Route::Donate => "/donate",
        }
    }
}

pub const GITHUB_PAGES_BASE_PATH: &str = "/face-crop-forge";

#[cfg(target_arch = "wasm32")]
pub fn app_base_path() -> String {
    let Some(window) = web_sys::window() else {
        return String::new();
    };
    let location = window.location();
    let hostname = location.hostname().unwrap_or_default();
    let pathname = location.pathname().unwrap_or_default();

    if hostname.ends_with("github.io") || pathname.starts_with(GITHUB_PAGES_BASE_PATH) {
        GITHUB_PAGES_BASE_PATH.to_string()
    } else {
        String::new()
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn app_base_path() -> String {
    String::new()
}

pub fn app_url(path: &str) -> String {
    let base = app_base_path();
    let path = if path == "/" { "" } else { path };
    if base.is_empty() {
        if path.is_empty() {
            "/".to_string()
        } else {
            path.to_string()
        }
    } else if path.is_empty() {
        format!("{base}/")
    } else {
        format!("{base}{path}")
    }
}

pub fn route_href(route: Route) -> String {
    app_url(route.path())
}

fn strip_app_base(path: &str) -> &str {
    path.strip_prefix(GITHUB_PAGES_BASE_PATH).unwrap_or(path)
}

pub fn route_from_path(path: &str) -> Route {
    match strip_app_base(path).trim_matches('/') {
        "batch" => Route::Batch,
        "single" => Route::Single,
        "csv" => Route::Csv,
        "docs" => Route::Docs,
        "donate" => Route::Donate,
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
    if let Some(window) = web_sys::window()
        && let Ok(history) = window.history()
    {
        let _ = history.push_state_with_url(&JsValue::NULL, "", Some(&route_href(route)));
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
