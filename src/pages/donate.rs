use crate::components::footer::Footer;
use crate::components::topbar::Topbar;
use crate::router::Route;
use leptos::prelude::*;
use qrcodegen::{QrCode, QrCodeEcc};

#[derive(Clone, Copy)]
struct DonateTier {
    name: &'static str,
    description: &'static str,
    href: &'static str,
    aria_label: &'static str,
}

#[derive(Clone, Copy)]
struct CryptoWallet {
    symbol: &'static str,
    network: &'static str,
    address: &'static str,
    explorer_base_url: &'static str,
}

const DONATE_TIERS: [DonateTier; 3] = [
    DonateTier {
        name: "Ko-fi",
        description: "Fast one-time support. Keeps the compiler warm.",
        href: "https://ko-fi.com/gregory_carnegie",
        aria_label: "Donate via Ko-fi",
    },
    DonateTier {
        name: "GitHub Sponsors",
        description: "Funds roadmap work and ongoing maintenance. Most popular.",
        href: "https://github.com/sponsors/gregorycarnegie",
        aria_label: "Sponsor on GitHub Sponsors",
    },
    DonateTier {
        name: "Buy Me a Coffee",
        description: "Simple support option for one-time or recurring donations.",
        href: "https://buymeacoffee.com/gregory_carnegie",
        aria_label: "Donate via Buy Me a Coffee",
    },
];

const CRYPTO_WALLETS: [CryptoWallet; 3] = [
    CryptoWallet {
        symbol: "BTC",
        network: "Bitcoin",
        address: "bc1qkuj79evqn87fagxygpfmgt4qmnqg4mm6qdejmg",
        explorer_base_url: "https://www.blockchain.com/explorer/addresses/btc/",
    },
    CryptoWallet {
        symbol: "ETH",
        network: "Ethereum",
        address: "0x32Fa5C823D4fa3A4745AAc7C56325ED849340F28",
        explorer_base_url: "https://etherscan.io/address/",
    },
    CryptoWallet {
        symbol: "SOL",
        network: "Solana",
        address: "8z1BWsshYfkdgJm38RHRuhhVkYiPFSZLXbaSMTNch8e1",
        explorer_base_url: "https://solscan.io/account/",
    },
];

fn wallet_qr_data_uri(address: &str) -> String {
    use base64::Engine as _;

    let Some(qr) = QrCode::encode_text(address, QrCodeEcc::Medium).ok() else {
        return "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 180 180'%3E%3Crect width='100%25' height='100%25' fill='%23f5f5f5'/%3E%3C/svg%3E".to_string();
    };

    const BORDER: i32 = 2;
    const MODULE_SIZE: i32 = 6;

    let size = qr.size();
    let canvas = (size + (BORDER * 2)) * MODULE_SIZE;

    let mut svg = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 {0} {0}' shape-rendering='crispEdges'><rect width='100%' height='100%' fill='white'/>",
        canvas
    );

    for y in 0..size {
        for x in 0..size {
            if qr.get_module(x, y) {
                let x_pos = (x + BORDER) * MODULE_SIZE;
                let y_pos = (y + BORDER) * MODULE_SIZE;
                svg.push_str(&format!(
                    "<rect x='{x_pos}' y='{y_pos}' width='{MODULE_SIZE}' height='{MODULE_SIZE}' fill='black'/>"
                ));
            }
        }
    }

    svg.push_str("</svg>");

    format!(
        "data:image/svg+xml;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(svg)
    )
}

#[component]
pub fn Donate(route: Route, set_route: WriteSignal<Route>) -> impl IntoView {
    view! {
        <Topbar route set_route />

        <div class="donate-page-wrap">
            <section class="donate-hero">
                <div class="donate-hero-text">
                    <div class="crumb">
                        <span>"Support"</span>
                        <span class="dot"></span>
                        <span class="now">"Donate"</span>
                    </div>
                    <h1>"Help make it "<span class="grad">"better."</span></h1>
                    <p>"Face Crop Forge is free and open-source. Every donation goes directly into development time — better GPU pipelines, new preset types, Linux and macOS builds, and UI improvements."</p>
                    <p style="margin-top:12px">"Even a small contribution keeps the project moving forward. No subscription, no paywalls, no strings attached."</p>
                    <div class="hero-cta" style="margin-top:28px">
                        <a class="btn-primary" href="https://github.com/sponsors/gregorycarnegie" target="_blank" rel="noopener noreferrer">
                            "GitHub Sponsors "<span class="arr">"→"</span>
                        </a>
                        <a class="btn-ghost" href="https://ko-fi.com/gregory_carnegie" target="_blank" rel="noopener noreferrer">
                            "Ko-fi"
                        </a>
                    </div>
                    <p class="donate-quote">"\"Good tools should be accessible to everyone.\""</p>
                </div>
                <div class="donate-meta">
                    <div class="donate-meta-item">
                        <span>"License"</span><b>"AGPL-3.0"</b>
                    </div>
                    <div class="donate-meta-item">
                        <span>"Cost"</span><b>"Free forever"</b>
                    </div>
                    <div class="donate-meta-item">
                        <span>"Platforms"</span><b>"3 options"</b>
                    </div>
                    <div class="donate-meta-item">
                        <span>"Crypto"</span><b>"BTC, ETH, SOL"</b>
                    </div>
                </div>
            </section>

            <section class="donate-tiers-section">
                <div class="caps-h">
                    <div class="left">
                        <div class="ix">"// support tiers"</div>
                        <h2>"Choose how to "<span class="grad">"contribute"</span></h2>
                    </div>
                    <p>"One-time or recurring. Pick whichever fits your style."</p>
                </div>
                <div class="donate-tiers">
                    <For
                        each=move || DONATE_TIERS.into_iter()
                        key=|tier| tier.name
                        children=move |tier| {
                            view! {
                                <a
                                    href=tier.href
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    class="tier"
                                    aria-label=tier.aria_label
                                >
                                    <div class="tier-top">
                                        <span class="tier-name">{tier.name}</span>
                                    </div>
                                    <p class="tier-desc">{tier.description}</p>
                                    <span class="tier-cta">"Open donation page"</span>
                                </a>
                            }
                        }
                    />
                </div>
            </section>

            <section class="crypto-section">
                <div class="caps-h">
                    <div class="left">
                        <div class="ix">"// crypto"</div>
                        <h2>"Donate with "<span class="grad">"crypto"</span></h2>
                    </div>
                    <p>"Prefer crypto? Use one of the wallets below."</p>
                </div>
                <div class="crypto-wallets" aria-label="Crypto donation wallets">
                    <For
                        each=move || CRYPTO_WALLETS.into_iter()
                        key=|wallet| wallet.symbol
                        children=move |wallet| {
                            let (show_qr, set_show_qr) = signal(false);
                            let explorer_url = format!("{}{}", wallet.explorer_base_url, wallet.address);
                            let qr_src = wallet_qr_data_uri(wallet.address);
                            view! {
                                <div class="crypto-wallet-item">
                                    <div class="crypto-wallet-row">
                                        <div class="crypto-wallet-meta">
                                            <span class="crypto-wallet-symbol">{wallet.symbol}</span>
                                            <span class="crypto-wallet-network">{wallet.network}</span>
                                        </div>
                                        <code class="crypto-wallet-address">{wallet.address}</code>
                                        <div class="crypto-wallet-actions">
                                            <button
                                                type="button"
                                                class="crypto-wallet-qr-btn"
                                                aria-expanded=move || if show_qr.get() { "true" } else { "false" }
                                                on:click=move |_| set_show_qr.update(|v| *v = !*v)
                                            >
                                                {move || if show_qr.get() { "Hide QR" } else { "Show QR" }}
                                            </button>
                                            <a
                                                href=explorer_url
                                                target="_blank"
                                                rel="noopener noreferrer"
                                                class="crypto-wallet-link"
                                                aria-label=format!("Open {} wallet address in explorer", wallet.symbol)
                                            >
                                                "View"
                                            </a>
                                        </div>
                                    </div>
                                    <img
                                        class="crypto-wallet-qr"
                                        src=qr_src
                                        alt=format!("QR code for {} wallet address", wallet.symbol)
                                        loading="lazy"
                                        style:display=move || if show_qr.get() { "block" } else { "none" }
                                    />
                                </div>
                            }
                        }
                    />
                </div>
            </section>

            <p class="donate-note">"// All platforms accept one-time or recurring donations"</p>
        </div>

        <Footer />
    }
}
