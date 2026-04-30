#![cfg(target_arch = "wasm32")]

use crate::pages::single::Single;
use crate::router::Route;
use crate::state::AppState;
use js_sys::{Array, Function, Promise, Reflect};
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::{JsValue, closure::Closure};
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::wasm_bindgen_test;

fn render_single_page() -> web_sys::Element {
    let document = web_sys::window().unwrap().document().unwrap();
    let container = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&container).unwrap();

    leptos::mount::mount_to(
        container.clone().unchecked_into::<web_sys::HtmlElement>(),
        || {
            provide_context(AppState::new());
            let (route, set_route) = signal(Route::Single);
            view! { <Single route=route.get() set_route /> }
        },
    )
    .forget();

    container
}

fn query(root: &web_sys::Element, selector: &str) -> web_sys::Element {
    root.query_selector(selector)
        .expect("querySelector threw")
        .unwrap_or_else(|| panic!("no element matched {selector:?}"))
}

fn install_browser_hooks() {
    let installer = Function::new_no_args(
        r#"
        window.__faceCropForgeDownloads = [];
        globalThis.FaceDetector = class {
          constructor() {}
          detect() {
            return Promise.resolve([
              {
                boundingBox: { x: 36, y: 30, width: 58, height: 66 },
                confidence: [0.92]
              }
            ]);
          }
        };

        if (!HTMLAnchorElement.prototype.__faceCropForgeDownloadHook) {
          Object.defineProperty(HTMLAnchorElement.prototype, "__faceCropForgeDownloadHook", {
            value: true
          });
          HTMLAnchorElement.prototype.click = function() {
            window.__faceCropForgeDownloads.push({
              download: this.download || "",
              href: this.href || ""
            });
          };
        }
        "#,
    );
    installer.call0(&JsValue::NULL).unwrap();
}

async fn synthetic_png_file() -> web_sys::File {
    let generator = Function::new_no_args(
        r##"
        return new Promise((resolve, reject) => {
          const canvas = document.createElement("canvas");
          canvas.width = 160;
          canvas.height = 160;
          const ctx = canvas.getContext("2d");
          ctx.fillStyle = "#f7d6c3";
          ctx.fillRect(0, 0, canvas.width, canvas.height);
          ctx.fillStyle = "#2f2f37";
          ctx.beginPath();
          ctx.arc(80, 68, 30, 0, Math.PI * 2);
          ctx.fill();
          ctx.fillStyle = "#475467";
          ctx.beginPath();
          ctx.ellipse(80, 142, 42, 34, 0, 0, Math.PI * 2);
          ctx.fill();
          canvas.toBlob((blob) => {
            if (!blob) {
              reject(new Error("canvas did not produce a blob"));
              return;
            }
            resolve(new File([blob], "fixture-face.png", { type: "image/png" }));
          }, "image/png");
        });
        "##,
    );

    let value = JsFuture::from(Promise::from(generator.call0(&JsValue::NULL).unwrap()))
        .await
        .unwrap();
    value.dyn_into::<web_sys::File>().unwrap()
}

fn set_file_input(input: &web_sys::HtmlInputElement, file: &web_sys::File) {
    let setter = Function::new_with_args(
        "input, file",
        r#"
        const transfer = new DataTransfer();
        transfer.items.add(file);
        input.files = transfer.files;
        input.dispatchEvent(new Event("change", { bubbles: true }));
        "#,
    );
    setter
        .call2(&JsValue::NULL, input.as_ref(), file.as_ref())
        .unwrap();
}

async fn delay_ms(timeout_ms: i32) {
    let promise = Promise::new(&mut |resolve, _reject| {
        let callback = Closure::once_into_js(move || {
            let _ = resolve.call0(&JsValue::NULL);
        });
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                callback.unchecked_ref(),
                timeout_ms,
            )
            .unwrap();
    });
    let _ = JsFuture::from(promise).await.unwrap();
}

async fn wait_for(mut condition: impl FnMut() -> bool, message: &str) {
    for _ in 0..100 {
        if condition() {
            return;
        }
        delay_ms(25).await;
    }
    panic!("{message}");
}

fn downloads() -> Array {
    Reflect::get(
        web_sys::window().unwrap().as_ref(),
        &JsValue::from_str("__faceCropForgeDownloads"),
    )
    .map(|value| Array::from(&value))
    .unwrap_or_else(|_| Array::new())
}

#[wasm_bindgen_test]
async fn single_image_detect_crop_export_flow_runs_in_browser() {
    install_browser_hooks();
    let root = render_single_page();
    let file = synthetic_png_file().await;
    let input = query(&root, "#singleImageInput")
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap();

    set_file_input(&input, &file);

    wait_for(
        || {
            root.query_selector(".face-chip")
                .ok()
                .flatten()
                .map(|chip| chip.text_content().unwrap_or_default().contains("face_1"))
                .unwrap_or(false)
        },
        "face chip did not render after detection",
    )
    .await;

    assert!(
        query(&root, ".face-chip")
            .text_content()
            .unwrap()
            .contains("0.92")
    );

    query(&root, ".out-card-single .dl")
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();

    wait_for(
        || downloads().length() == 1,
        "export did not trigger a download",
    )
    .await;
    wait_for(
        || {
            root.query_selector(".out-card-single .sub")
                .ok()
                .flatten()
                .map(|meta| {
                    meta.text_content()
                        .unwrap_or_default()
                        .contains("image/png")
                })
                .unwrap_or(false)
        },
        "export preview metadata did not update",
    )
    .await;

    let first = downloads().get(0);
    let name = Reflect::get(&first, &JsValue::from_str("download"))
        .unwrap()
        .as_string()
        .unwrap();
    assert_eq!(name, "face_fixture-face_1.png");
    assert!(
        query(&root, ".out-card-single .nm")
            .text_content()
            .unwrap()
            .contains(&name)
    );
}
