use crate::components::footer::Footer;
use crate::components::pill::Pill;
use crate::components::stat_card::StatCard;
use crate::components::topbar::Topbar;
use crate::router::{Route, navigate};
use leptos::prelude::*;

#[component]
pub fn Home(route: Route, set_route: WriteSignal<Route>) -> impl IntoView {
    view! {
        <Topbar route set_route />

        <section class="hero">
            <div>
                <div class="pill-row rise">
                    <Pill variant="peach" has_dot=true>"v0.9.2 · Now with WebGPU"</Pill>
                    <Pill has_dot=true>"0 bytes uploaded"</Pill>
                </div>

                <h1 class="hero-h rise d1">
                    "Crop faces"<br/>
                    "from "<span class="grad">"thousands"</span><span class="ic"></span>"of images "<br/>
                    <span class="squig">"without"</span>" uploading."
                </h1>

                <p class="lede rise d2">
                    "Face Crop Forge is a "<b>"browser-native image pipeline"</b>" for dataset prep,
                    profile photos, and ML training sets. Drop in a folder, watch the bounding
                    boxes settle, and ship a "<span class="hl">"deterministic ZIP"</span>" — all on your
                    machine, all on your GPU, nothing ever leaves the tab."
                </p>

                <div class="hero-cta rise d3">
                    <a class="btn-primary" href="/batch" on:click=move |ev| { ev.prevent_default(); navigate(Route::Batch, set_route); }>
                        "Start with Batch "<span class="arr">"→"</span>
                    </a>
                    <a class="btn-ghost" href="/single" on:click=move |ev| { ev.prevent_default(); navigate(Route::Single, set_route); }>
                        "Try a single image"
                    </a>
                </div>

                <div class="micro rise d3">
                    <span><b>"WebGPU + WASM"</b>" runtime"</span>
                    <span class="sep"></span>
                    <span><b>"YuNet ONNX"</b>" model"</span>
                    <span class="sep"></span>
                    <span><b>"~28 img/sec"</b>" throughput"</span>
                    <span class="sep"></span>
                    <span><b>"MIT"</b>" licensed"</span>
                </div>
            </div>


            <div class="preview rise d2">
                <div class="floats">
                    <div class="float f1"><div class="k">"Backend"</div><div class="v"><em>"WebGPU"</em></div></div>
                    <div class="float f2"><div class="k">"Throughput"</div><div class="v"><em>"28.4"</em><span style="font-size:14px;color:var(--ink-3)">" img/s"</span></div></div>
                    <div class="float f3"><div class="k">"Detected"</div><div class="v"><em>"112"</em><span style="font-size:14px;color:var(--ink-3)">" / 117"</span></div></div>
                </div>

                <div class="app">
                    <div class="app-bar">
                        <div class="traffic"><span></span><span></span><span></span></div>
                        <div class="urlbar">"facecropforge.app/"<b>"batch"</b></div>
                        <span class="runtag">"● running"</span>
                    </div>

                    <div class="app-body">
                        <aside class="app-sidebar">
                            <div class="sb-h">"workflows"</div>
                            <div class="sb-item active"><span class="ico"></span>"Batch"<span class="count">"204"</span></div>
                            <div class="sb-item"><span class="ico"></span>"Single image"</div>
                            <div class="sb-item"><span class="ico"></span>"CSV-driven"<span class="count">"3"</span></div>
                            <div class="sb-h" style="margin-top:8px">"recent runs"</div>
                            <div class="sb-item"><span class="ico"></span>"portraits-2026"</div>
                            <div class="sb-item"><span class="ico"></span>"headshots-q2"</div>
                            <div class="sb-item"><span class="ico"></span>"archive-may"</div>
                        </aside>

                        <div class="app-main">
                            <div class="am-head">
                                <div>
                                    <h2><b>"portraits-2026"</b>" · running"</h2>
                                    <div style="font-size:12.5px;color:var(--ink-3);margin-top:4px;font-family:var(--mono)">"batch.run · 14:02:17 · 117 of 204"</div>
                                </div>
                                <div class="controls">
                                    <span class="ctl">"pad "<b>"18%"</b></span>
                                    <span class="ctl">"aspect "<b>"1:1"</b></span>
                                    <span class="ctl peach">"conf "<b>"0.62"</b></span>
                                </div>
                            </div>

                            <div class="vp-grid">
                                <div class="vc"><svg viewBox="0 0 100 100"><defs><linearGradient id="ag1" x1="0" x2="0" y1="0" y2="1"><stop offset="0" stop-color="#3a2e22"/><stop offset="1" stop-color="#161217"/></linearGradient></defs><rect width="100" height="100" fill="url(#ag1)"/><circle cx="50" cy="42" r="15" fill="#d8b88a"/><ellipse cx="50" cy="84" rx="26" ry="18" fill="#d8b88a"/></svg><span class="frm" style="left:24%;top:22%;width:54%;height:56%" data-c="0.97"></span><span class="name">"001"</span></div>
                                <div class="vc"><svg viewBox="0 0 100 100"><defs><linearGradient id="ag2" x1="0" x2="1" y1="0" y2="1"><stop offset="0" stop-color="#1f2a32"/><stop offset="1" stop-color="#0e1418"/></linearGradient></defs><rect width="100" height="100" fill="url(#ag2)"/><circle cx="46" cy="44" r="14" fill="#e6c8a3"/><ellipse cx="46" cy="84" rx="24" ry="17" fill="#e6c8a3"/></svg><span class="frm" style="left:20%;top:24%;width:54%;height:54%" data-c="0.94"></span><span class="name">"002"</span></div>
                                <div class="vc"><svg viewBox="0 0 100 100"><defs><linearGradient id="ag3" x1="0" x2="0" y1="0" y2="1"><stop offset="0" stop-color="#2c1f15"/><stop offset="1" stop-color="#11100c"/></linearGradient></defs><rect width="100" height="100" fill="url(#ag3)"/><circle cx="54" cy="46" r="15" fill="#c9a17a"/><ellipse cx="54" cy="86" rx="26" ry="18" fill="#c9a17a"/></svg><span class="frm" style="left:28%;top:26%;width:52%;height:52%" data-c="0.99"></span><span class="name">"003"</span></div>
                                <div class="vc miss"><svg viewBox="0 0 100 100"><defs><linearGradient id="ag4" x1="0" x2="1" y1="0" y2="1"><stop offset="0" stop-color="#231a18"/><stop offset="1" stop-color="#0e0a0c"/></linearGradient></defs><rect width="100" height="100" fill="url(#ag4)"/><rect x="22" y="32" width="56" height="40" fill="#3a2e34" opacity=".55" rx="3"/></svg><span class="frm" style="left:18%;top:24%;width:64%;height:56%" data-c="—"></span><span class="name">"skip"</span></div>
                                <div class="vc"><svg viewBox="0 0 100 100"><defs><linearGradient id="ag5" x1="0" x2="0" y1="0" y2="1"><stop offset="0" stop-color="#16292a"/><stop offset="1" stop-color="#0a1413"/></linearGradient></defs><rect width="100" height="100" fill="url(#ag5)"/><circle cx="48" cy="42" r="15" fill="#b89a7a"/><ellipse cx="48" cy="82" rx="24" ry="17" fill="#b89a7a"/></svg><span class="frm" style="left:22%;top:22%;width:54%;height:54%" data-c="0.96"></span><span class="name">"005"</span></div>
                                <div class="vc"><svg viewBox="0 0 100 100"><defs><linearGradient id="ag6" x1="0" x2="1" y1="0" y2="1"><stop offset="0" stop-color="#2c1a22"/><stop offset="1" stop-color="#120c10"/></linearGradient></defs><rect width="100" height="100" fill="url(#ag6)"/><circle cx="52" cy="44" r="14" fill="#dbb892"/><ellipse cx="52" cy="84" rx="24" ry="17" fill="#dbb892"/></svg><span class="frm" style="left:26%;top:24%;width:52%;height:52%" data-c="0.92"></span><span class="name">"006"</span></div>
                                <div class="vc"><svg viewBox="0 0 100 100"><defs><linearGradient id="ag7" x1="0" x2="0" y1="0" y2="1"><stop offset="0" stop-color="#23202a"/><stop offset="1" stop-color="#0e0c12"/></linearGradient></defs><rect width="100" height="100" fill="url(#ag7)"/><circle cx="50" cy="40" r="13" fill="#caa888"/><ellipse cx="50" cy="80" rx="22" ry="15" fill="#caa888"/></svg><span class="frm" style="left:28%;top:20%;width:48%;height:54%" data-c="0.98"></span><span class="name">"007"</span></div>
                                <div class="vc"><svg viewBox="0 0 100 100"><defs><linearGradient id="ag8" x1="0" x2="1" y1="0" y2="1"><stop offset="0" stop-color="#1d2b1f"/><stop offset="1" stop-color="#0c130d"/></linearGradient></defs><rect width="100" height="100" fill="url(#ag8)"/><circle cx="50" cy="44" r="15" fill="#cfa67e"/><ellipse cx="50" cy="84" rx="25" ry="18" fill="#cfa67e"/></svg><span class="frm" style="left:24%;top:24%;width:52%;height:52%" data-c="0.95"></span><span class="name">"008"</span></div>
                            </div>

                            <div class="am-foot">
                                <div>
                                    <div class="pmeta"><span><b>"Batch"</b>" · img 117 / 204"</span><span class="peach"><b class="peach">"57%"</b></span></div>
                                    <div class="ptrack"><i></i></div>
                                </div>
                                <div class="actions">
                                    <span class="ctl">"Pause"</span>
                                    <a class="pri" href="/export">"Export ZIP"</a>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </section>


        <section class="workflows">
            <a class="wcard batch" href="/batch" on:click=move |ev| { ev.prevent_default(); navigate(Route::Batch, set_route); }>
                <span class="badge">"Workflow 01 · Most popular"</span>
                <h3>"Process by the"<br/>"folder."</h3>
                <p>"Drag in thousands. The forge runs YuNet on your GPU and ships a clean ZIP with a manifest."</p>
                <div class="ill">
                    <div class="chip"><svg viewBox="0 0 30 30"><rect width="30" height="30" fill="#2a1d14"/><circle cx="15" cy="13" r="5" fill="#caa888"/><ellipse cx="15" cy="26" rx="9" ry="6" fill="#caa888"/></svg><i></i></div>
                    <div class="chip"><svg viewBox="0 0 30 30"><rect width="30" height="30" fill="#1c2a32"/><circle cx="15" cy="13" r="5" fill="#dbb892"/><ellipse cx="15" cy="26" rx="9" ry="6" fill="#dbb892"/></svg><i></i></div>
                    <div class="chip"><svg viewBox="0 0 30 30"><rect width="30" height="30" fill="#2a1f15"/><circle cx="15" cy="13" r="5" fill="#cfa67e"/><ellipse cx="15" cy="26" rx="9" ry="6" fill="#cfa67e"/></svg><i></i></div>
                </div>
                <span class="arrow">"→"</span>
            </a>

            <a class="wcard single" href="/single" on:click=move |ev| { ev.prevent_default(); navigate(Route::Single, set_route); }>
                <span class="badge">"Workflow 02 · Precision"</span>
                <h3>"Tune one shot,"<br/>"just right."</h3>
                <p>"Manual padding, aspect lock, retina export. Built for the picture you'll actually publish."</p>
                <div class="ill">
                    <div class="chip" style="width:56px"><svg viewBox="0 0 56 30"><rect width="56" height="30" fill="#1c2a32"/><circle cx="28" cy="13" r="6" fill="#dbb892"/><ellipse cx="28" cy="26" rx="11" ry="6" fill="#dbb892"/></svg><i style="inset:14% 28%"></i></div>
                </div>
                <span class="arrow">"→"</span>
            </a>

            <a class="wcard csv" href="/csv" on:click=move |ev| { ev.prevent_default(); navigate(Route::Csv, set_route); }>
                <span class="badge">"Workflow 03 · Pipeline"</span>
                <h3>"Driven by your"<br/>"CSV."</h3>
                <p>"Pair filenames with IDs. Outputs are named exactly the way your training pipeline expects."</p>
                <div class="ill" style="bottom:20px;font-family:var(--mono);font-size:10px;color:var(--ink-3)">
                    <div class="chip" style="width:auto;padding:4px 8px;color:var(--ink-2)"><span style="position:relative;z-index:1">"id_001.jpg"</span></div>
                    <div class="chip" style="width:auto;padding:4px 8px;color:var(--ink-2)"><span style="position:relative;z-index:1">"id_002.jpg"</span></div>
                    <div class="chip" style="width:auto;padding:4px 8px;color:var(--ink-2)"><span style="position:relative;z-index:1">"id_003.jpg"</span></div>
                </div>
                <span class="arrow">"→"</span>
            </a>
        </section>


        <section class="statement">
            <div class="statement-grid">
                <div>
                    <h2>"An image pipeline"<br/>"that "<span class="grad">"stays on your machine"</span>"."</h2>
                    <p>"The forge does the unglamorous middle step of every dataset workflow — finding faces, framing them, keeping them organized — "<b>"without uploading a single byte"</b>". Audit the source. Run it offline. Trust it because you can read it."</p>
                </div>
                <div class="stat-grid">
                    <StatCard key="throughput" value="28" suffix="/s" sub="images per second on M-class GPU" accent="peach" />
                    <StatCard key="accuracy" value="99.4" suffix="%" sub="precision on FDDB benchmark" accent="cyan" />
                    <StatCard key="privacy" value="0" sub="bytes leave your device" accent="lime" />
                    <StatCard key="formats" value="JPG · PNG · WEBP" sub="EXIF preserved through pipeline" accent="rose" custom_value_size=true />
                </div>
            </div>
        </section>


        <section class="caps">
            <div class="caps-h">
                <div class="left">
                    <div class="ix">"What it does well"</div>
                    <h2>"Built for people who "<span class="grad">"ship image data"</span>"."</h2>
                </div>
                <p>"Dataset preparation, headshot pipelines, registry photos, and any workflow where you'd otherwise babysit a Python script for an afternoon."</p>
            </div>
            <div class="caps-grid">
                <div class="cap">
                    <div class="icon"><svg viewBox="0 0 18 18" fill="none" stroke="currentColor" stroke-width="1.6"><rect x="3" y="3" width="12" height="12" rx="2"/><path d="M3 7h12M7 3v12"/></svg></div>
                    <h4>"Stays on your device"</h4>
                    <p>"Images never leave the tab. No upload, no telemetry, no third-party server."</p>
                </div>
                <div class="cap cyan">
                    <div class="icon"><svg viewBox="0 0 18 18" fill="none" stroke="currentColor" stroke-width="1.6"><path d="M3 9h12M9 3v12M3 3l12 12M15 3L3 15"/></svg></div>
                    <h4>"Built for batches"</h4>
                    <p>"Tens of thousands of images in one session. Resume on crash, skip duplicates."</p>
                </div>
                <div class="cap lime">
                    <div class="icon"><svg viewBox="0 0 18 18" fill="none" stroke="currentColor" stroke-width="1.6"><rect x="2" y="4" width="14" height="11" rx="1.5"/><path d="M2 8h14M6 4v11"/></svg></div>
                    <h4>"CSV-driven runs"</h4>
                    <p>"Pair filenames with IDs. Outputs named exactly the way your pipeline expects."</p>
                </div>
                <div class="cap rose">
                    <div class="icon"><svg viewBox="0 0 18 18" fill="none" stroke="currentColor" stroke-width="1.6"><path d="M9 2v14M2 9h14" stroke-linecap="round"/><circle cx="9" cy="9" r="6"/></svg></div>
                    <h4>"Reproducible by design"</h4>
                    <p>"Same model weights, same parameters, same crops. Every run produces a manifest."</p>
                </div>
            </div>
        </section>


        <section class="pipe">
            <div class="pipe-h">
                <h3>"The "<b>"5-stage"</b>" pipeline"</h3>
                <span class="meta"><b>"portraits-2026"</b>" · stage 02 running · 14:02:17"</span>
            </div>
            <div class="pipe-grid">
                <div class="pcol">
                    <div class="n">"stage 01 · ingest"</div>
                    <h4>"Drop, paste, or pair"</h4>
                    <p>"FileSystemAccess reads up to 100k files. No upload, no temp directory."</p>
                </div>
                <div class="pcol run">
                    <div class="n">"stage 02 · running"</div>
                    <h4>"Detect with YuNet"</h4>
                    <p>"WebGPU runs the ONNX model. Falls back to WASM SIMD on older hardware."</p>
                </div>
                <div class="pcol">
                    <div class="n">"stage 03 · frame"</div>
                    <h4>"Pad & lock aspect"</h4>
                    <p>"Optionally aligns by eye landmarks. Rejects below confidence threshold."</p>
                </div>
                <div class="pcol">
                    <div class="n">"stage 04 · render"</div>
                    <h4>"Resample on GPU"</h4>
                    <p>"Lanczos · bicubic · nearest. JPG, PNG, or WEBP output, EXIF preserved."</p>
                </div>
                <div class="pcol">
                    <div class="n">"stage 05 · export"</div>
                    <h4>"Deterministic ZIP"</h4>
                    <p>"Writes manifest.csv alongside crops. Same params, same bytes, every run."</p>
                </div>
            </div>
        </section>

        <Footer />
    }
}
