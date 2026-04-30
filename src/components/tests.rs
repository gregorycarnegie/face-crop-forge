#![cfg(target_arch = "wasm32")]

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

use super::log_card::LogCard;
use super::panel::Panel;
use super::pill::{Badge, Pill};
use super::progress_bar::ProgressBar;
use super::stat_card::StatCard;

wasm_bindgen_test_configure!(run_in_browser);

fn render<F, V>(f: F) -> web_sys::Element
where
    F: FnOnce() -> V + 'static,
    V: IntoView,
{
    let doc = web_sys::window().unwrap().document().unwrap();
    let container = doc.create_element("div").unwrap();
    doc.body().unwrap().append_child(&container).unwrap();
    leptos::mount::mount_to(
        container.clone().unchecked_into::<web_sys::HtmlElement>(),
        f,
    )
    .forget();
    container
}

fn query(root: &web_sys::Element, selector: &str) -> web_sys::Element {
    root.query_selector(selector)
        .expect("querySelector threw")
        .unwrap_or_else(|| panic!("no element matched {selector:?}"))
}

fn has_class(el: &web_sys::Element, cls: &str) -> bool {
    el.get_attribute("class")
        .map(|c| c.split_whitespace().any(|s| s == cls))
        .unwrap_or(false)
}

// ── Pill ────────────────────────────────────────────────────────────────────

#[wasm_bindgen_test]
fn pill_no_variant_has_base_class_only() {
    let c = render(|| view! { <Pill>"text"</Pill> });
    assert_eq!(
        query(&c, ".pill").get_attribute("class").as_deref(),
        Some("pill"),
    );
}

#[wasm_bindgen_test]
fn pill_variant_appended_to_class() {
    let c = render(|| view! { <Pill variant="ok">"text"</Pill> });
    assert!(has_class(&query(&c, ".pill"), "ok"));
}

#[wasm_bindgen_test]
fn pill_no_dot_element_by_default() {
    let c = render(|| view! { <Pill variant="ok">"text"</Pill> });
    let span = query(&c, ".pill");
    assert!(span.query_selector(".d").unwrap().is_none());
    assert!(span.query_selector(".dot").unwrap().is_none());
}

#[wasm_bindgen_test]
fn pill_new_style_dot_for_non_peach() {
    let c = render(|| view! { <Pill variant="ok" has_dot=true>"text"</Pill> });
    let _ = query(&c, ".pill .d");
}

#[wasm_bindgen_test]
fn pill_peach_dot_uses_dot_class() {
    let c = render(|| view! { <Pill variant="peach" has_dot=true>"text"</Pill> });
    let _ = query(&c, ".pill .dot");
}

#[wasm_bindgen_test]
fn pill_renders_children() {
    let c = render(|| view! { <Pill>"hello"</Pill> });
    assert!(query(&c, ".pill").inner_html().contains("hello"));
}

// ── Badge ───────────────────────────────────────────────────────────────────

#[wasm_bindgen_test]
fn badge_applies_variant_class() {
    let c = render(|| view! { <Badge variant="fail">"text"</Badge> });
    assert!(has_class(&query(&c, ".badge"), "fail"));
}

#[wasm_bindgen_test]
fn badge_renders_children() {
    let c = render(|| view! { <Badge variant="ok">"pass"</Badge> });
    assert!(query(&c, ".badge").inner_html().contains("pass"));
}

// ── StatCard ─────────────────────────────────────────────────────────────────

#[wasm_bindgen_test]
fn stat_card_renders_key_and_value() {
    let c = render(|| view! { <StatCard key="Size" value="42" /> });
    assert!(query(&c, ".k").inner_html().contains("Size"));
    assert!(query(&c, ".v em").inner_html().contains("42"));
}

#[wasm_bindgen_test]
fn stat_card_suffix_shown_when_provided() {
    let c = render(|| view! { <StatCard key="k" value="v" suffix=" MB" /> });
    assert!(query(&c, ".v").inner_html().contains("MB"));
}

#[wasm_bindgen_test]
fn stat_card_no_suffix_span_when_empty() {
    let c = render(|| view! { <StatCard key="k" value="v" /> });
    assert!(query(&c, ".v").query_selector("span").unwrap().is_none());
}

#[wasm_bindgen_test]
fn stat_card_sub_shown_when_provided() {
    let c = render(|| view! { <StatCard key="k" value="v" sub="detail" /> });
    assert!(query(&c, ".sub").inner_html().contains("detail"));
}

#[wasm_bindgen_test]
fn stat_card_no_sub_when_empty() {
    let c = render(|| view! { <StatCard key="k" value="v" /> });
    assert!(query(&c, ".s").query_selector(".sub").unwrap().is_none());
}

#[wasm_bindgen_test]
fn stat_card_accent_applied_to_root() {
    let c = render(|| view! { <StatCard key="k" value="v" accent="cyan" /> });
    assert!(has_class(&query(&c, ".s"), "cyan"));
}

// ── ProgressBar ──────────────────────────────────────────────────────────────

#[wasm_bindgen_test]
fn progress_bar_width_matches_initial_signal() {
    let progress = RwSignal::new(65_u32);
    let c = render(move || view! { <ProgressBar progress=progress /> });
    assert_eq!(
        query(&c, ".ptrack i").get_attribute("style").as_deref(),
        Some("width: 65%"),
    );
}

#[wasm_bindgen_test]
fn progress_bar_zero_renders_zero_width() {
    let progress = RwSignal::new(0_u32);
    let c = render(move || view! { <ProgressBar progress=progress /> });
    assert_eq!(
        query(&c, ".ptrack i").get_attribute("style").as_deref(),
        Some("width: 0%"),
    );
}

// ── LogCard ──────────────────────────────────────────────────────────────────

#[wasm_bindgen_test]
fn log_card_title_in_heading() {
    let c = render(|| {
        view! {
            <LogCard title="Events" accent="cyan" meta_left="l" meta_right="">"body"</LogCard>
        }
    });
    assert!(query(&c, ".log-card h3").inner_html().contains("Events"));
}

#[wasm_bindgen_test]
fn log_card_accent_class_on_b_element() {
    let c = render(|| {
        view! {
            <LogCard title="t" accent="lime" meta_left="l" meta_right="">"body"</LogCard>
        }
    });
    assert!(has_class(&query(&c, ".log-head h3 b"), "lime"));
}

#[wasm_bindgen_test]
fn log_card_meta_right_shown_when_provided() {
    let c = render(|| {
        view! {
            <LogCard title="t" accent="peach" meta_left="left" meta_right="sentinel-value">"body"</LogCard>
        }
    });
    assert!(query(&c, ".meta").inner_html().contains("sentinel-value"));
}

#[wasm_bindgen_test]
fn log_card_meta_right_hidden_when_empty() {
    let c = render(|| {
        view! {
            <LogCard title="t" accent="peach" meta_left="left" meta_right="">"body"</LogCard>
        }
    });
    assert!(
        query(&c, ".meta")
            .query_selector("span + span")
            .unwrap()
            .is_none(),
        "no second span should render when meta_right is empty",
    );
}

// ── Panel ─────────────────────────────────────────────────────────────────────

#[wasm_bindgen_test]
fn panel_open_by_default() {
    let c = render(|| view! { <Panel title="Settings" num="1">"content"</Panel> });
    assert!(has_class(&query(&c, ".panel"), "open"));
}

#[wasm_bindgen_test]
fn panel_closed_when_initially_closed() {
    let c = render(|| {
        view! {
            <Panel title="Settings" num="1" initially_open=false>"content"</Panel>
        }
    });
    assert!(!has_class(&query(&c, ".panel"), "open"));
}

#[wasm_bindgen_test]
fn panel_accent_class_applied() {
    let c = render(|| view! { <Panel title="t" num="1" accent="peach">"content"</Panel> });
    assert!(has_class(&query(&c, ".panel"), "peach"));
}

#[wasm_bindgen_test]
fn panel_badge_shown_when_provided() {
    let c = render(|| {
        view! {
            <Panel title="t" num="1" badge="Beta">"content"</Panel>
        }
    });
    let _ = query(&c, ".chip");
}

#[wasm_bindgen_test]
fn panel_no_badge_when_none() {
    let c = render(|| view! { <Panel title="t" num="1">"content"</Panel> });
    assert!(
        query(&c, ".panel")
            .query_selector(".chip")
            .unwrap()
            .is_none()
    );
}

// ── Crop + export pipeline (E2E in real browser) ──────────────────────────────

fn make_canvas(width: u32, height: u32, fill: &str) -> web_sys::HtmlCanvasElement {
    use wasm_bindgen::JsCast;
    use wasm_bindgen::JsValue;

    let doc = web_sys::window().unwrap().document().unwrap();
    let canvas = doc
        .create_element("canvas")
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .unwrap();
    canvas.set_width(width);
    canvas.set_height(height);
    let ctx = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();
    ctx.set_fill_style(&JsValue::from_str(fill));
    ctx.fill_rect(0.0, 0.0, f64::from(width), f64::from(height));
    canvas
}

#[wasm_bindgen_test]
async fn crop_face_bytes_returns_valid_png() {
    let canvas = make_canvas(200, 200, "rgb(200, 100, 50)");
    let data_url = canvas.to_data_url().expect("to_data_url failed");
    assert!(data_url.starts_with("data:image/png"));

    let face = crate::worker_bridge::DetectedFace {
        id: "face_1".to_string(),
        x: 50.0,
        y: 50.0,
        width: 100.0,
        height: 100.0,
        confidence: 1.0,
    };
    let settings = crate::state::ProcessingSettings {
        output_width: 64,
        output_height: 64,
        ..crate::state::ProcessingSettings::default()
    };

    let bytes =
        crate::runtime::crop_face_bytes_from_source(&data_url, &face, &settings, "image/png")
            .await
            .expect("crop_face_bytes_from_source failed");

    assert!(!bytes.is_empty(), "crop output must not be empty");
    // PNG magic bytes: 0x89 P N G
    assert_eq!(&bytes[..4], b"\x89PNG", "output must be a valid PNG");
}

#[wasm_bindgen_test]
async fn crop_face_bytes_returns_valid_jpeg() {
    let canvas = make_canvas(200, 200, "rgb(50, 150, 200)");
    let data_url = canvas.to_data_url().expect("to_data_url failed");

    let face = crate::worker_bridge::DetectedFace {
        id: "face_1".to_string(),
        x: 25.0,
        y: 25.0,
        width: 150.0,
        height: 150.0,
        confidence: 0.95,
    };
    let settings = crate::state::ProcessingSettings {
        output_width: 128,
        output_height: 128,
        jpeg_quality: 0.9,
        ..crate::state::ProcessingSettings::default()
    };

    let bytes =
        crate::runtime::crop_face_bytes_from_source(&data_url, &face, &settings, "image/jpeg")
            .await
            .expect("crop_face_bytes_from_source failed for JPEG");

    assert!(!bytes.is_empty(), "JPEG crop output must not be empty");
    // JPEG magic bytes: FF D8
    assert_eq!(&bytes[..2], b"\xFF\xD8", "output must be a valid JPEG");
}

#[wasm_bindgen_test]
async fn crop_then_zip_produces_valid_archive() {
    let canvas = make_canvas(120, 120, "blue");
    let data_url = canvas.to_data_url().expect("to_data_url failed");

    let face = crate::worker_bridge::DetectedFace {
        id: "face_1".to_string(),
        x: 10.0,
        y: 10.0,
        width: 100.0,
        height: 100.0,
        confidence: 0.9,
    };
    let settings = crate::state::ProcessingSettings::default();

    let crop_bytes =
        crate::runtime::crop_face_bytes_from_source(&data_url, &face, &settings, "image/png")
            .await
            .expect("crop failed");

    let zip_bytes =
        crate::export_runtime::build_zip_bytes(&[("face_1.png".to_string(), crop_bytes)])
            .expect("build_zip_bytes failed");

    // ZIP local file header magic: PK 0x03 0x04
    assert_eq!(&zip_bytes[..4], b"PK\x03\x04", "output must be a valid ZIP");
}

#[wasm_bindgen_test]
async fn confidence_filter_excludes_low_confidence_face_from_export() {
    let canvas = make_canvas(200, 200, "green");
    let data_url = canvas.to_data_url().expect("to_data_url failed");

    let faces = vec![
        crate::worker_bridge::DetectedFace {
            id: "face_1".to_string(),
            x: 10.0,
            y: 10.0,
            width: 80.0,
            height: 80.0,
            confidence: 0.9,
        },
        crate::worker_bridge::DetectedFace {
            id: "face_2".to_string(),
            x: 110.0,
            y: 110.0,
            width: 80.0,
            height: 80.0,
            confidence: 0.2,
        },
    ];
    let settings = crate::state::ProcessingSettings {
        min_confidence: 0.5,
        output_width: 64,
        output_height: 64,
        ..crate::state::ProcessingSettings::default()
    };

    let passing = crate::runtime::apply_detection_quality_filters(faces, &settings);
    assert_eq!(
        passing.len(),
        1,
        "only the high-confidence face should pass"
    );

    let bytes =
        crate::runtime::crop_face_bytes_from_source(&data_url, &passing[0], &settings, "image/png")
            .await
            .expect("crop of passing face failed");
    assert_eq!(&bytes[..4], b"\x89PNG");
}
