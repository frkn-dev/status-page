use crate::i18n::{detect_lang, save_lang, t, Lang};
use crate::network::{
    check_frkn_nodes, detect_vpn, fetch_frkn_nodes, fetch_ip_info, is_frkn_ip, measure_speed,
    FrknAggregateStatus, FrknNodeStatus, IpInfo,
};
use crate::services::ServiceStatus;
use leptos::*;
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn App() -> impl IntoView {
    let (ip_info, set_ip_info) = create_signal::<Option<IpInfo>>(None);
    let (is_vpn, set_is_vpn) = create_signal(false);
    let (speed_mbps, set_speed_mbps) = create_signal::<Option<f64>>(None);
    let (is_speed_testing, set_is_speed_testing) = create_signal(false);
    let (loading, set_loading) = create_signal(true);
    let (error, set_error) = create_signal::<Option<String>>(None);
    let (is_frkn_vpn, set_is_frkn_vpn) = create_signal(false);
    let (frkn_aggregate, set_frkn_aggregate) = create_signal(FrknAggregateStatus::AllOffline);
    let (frkn_nodes, set_frkn_nodes) = create_signal(Vec::<FrknNodeStatus>::new());
    let (frkn_loading, set_frkn_loading) = create_signal(true);
    let (expanded, set_expanded) = create_signal(false);
    let (lang, set_lang) = create_signal(detect_lang());

    // Заменить на реальные ссылки после публикации в сторах.
    const CHROME_STORE_URL: &str = "https://chromewebstore.google.com/detail/frkn-service-checker/elngedoofkkabmcnnldncempkklofmkh";
    const FIREFOX_STORE_URL: &str = "https://addons.mozilla.org/addon/frkn-service-checker/";

    let run_checks = move || {
        set_loading.set(true);
        set_frkn_loading.set(true);
        set_error.set(None);

        spawn_local(async move {
            // Запрашиваем IP и список FRKN-нод параллельно.
            let (ip_result, nodes_result) = futures::join!(fetch_ip_info(), fetch_frkn_nodes());

            let mut vpn_detected = false;

            // IP + VPN
            match ip_result {
                Ok(info) => {
                    vpn_detected = detect_vpn(&info);
                    set_ip_info.set(Some(info));
                }
                Err(e) => {
                    web_sys::console::error_1(&format!("IP fetch error: {e}").into());
                    set_error.set(Some(format!("Не удалось определить IP: {e}")));
                }
            }

            // FRKN nodes + VPN matching
            match nodes_result {
                Ok(nodes) => {
                    if let Some(ref info) = ip_info.get_untracked() {
                        let frkn_match = is_frkn_ip(&info.ip, &nodes);
                        if frkn_match {
                            vpn_detected = true;
                        }
                        set_is_frkn_vpn.set(frkn_match);
                    }
                    let (aggregate, statuses) = check_frkn_nodes(&nodes).await;
                    set_frkn_aggregate.set(aggregate);
                    set_frkn_nodes.set(statuses);
                }
                Err(e) => {
                    web_sys::console::error_1(&format!("FRKN nodes fetch error: {e}").into());
                    set_frkn_aggregate.set(FrknAggregateStatus::Error);
                }
            }

            set_is_vpn.set(vpn_detected);
            set_frkn_loading.set(false);
            set_loading.set(false);
        });
    };

    run_checks();

    let run_speed_test = move |_| {
        set_is_speed_testing.set(true);
        set_speed_mbps.set(None);
        spawn_local(async move {
            match measure_speed().await {
                Ok(mbps) => set_speed_mbps.set(Some(mbps)),
                Err(e) => {
                    web_sys::console::error_1(&format!("Speed test error: {e}").into());
                    set_speed_mbps.set(Some(0.0));
                }
            }
            set_is_speed_testing.set(false);
        });
    };

    view! {
        <div class="min-h-screen p-4 md:p-8 max-w-6xl mx-auto">
            // Header
            <header class="mb-8 text-center md:text-left">
                <div class="flex items-center justify-between gap-3 mb-3">
                    <div class="flex items-center justify-center md:justify-start gap-3">
                        <img src="logo.png" alt="FRKN" class="w-10 h-10 rounded-lg" />
                        <h1 class="text-3xl md:text-4xl font-extrabold tracking-tight">
                            "Status Page"
                        </h1>
                    </div>
                    <div class="inline-flex rounded-xl border border-frkn-border bg-frkn-card overflow-hidden">
                        <button
                            on:click=move |_| { set_lang.set(Lang::Ru); save_lang(Lang::Ru); }
                            class={move || format!("px-3 py-1.5 text-sm font-semibold transition {}", if lang.get() == Lang::Ru { "bg-frkn-accent text-white" } else { "text-frkn-muted hover:text-frkn-text" })}
                        >
                            {move || t(lang.get(), "lang_ru")}
                        </button>
                        <button
                            on:click=move |_| { set_lang.set(Lang::En); save_lang(Lang::En); }
                            class={move || format!("px-3 py-1.5 text-sm font-semibold transition {}", if lang.get() == Lang::En { "bg-frkn-accent text-white" } else { "text-frkn-muted hover:text-frkn-text" })}
                        >
                            {move || t(lang.get(), "lang_en")}
                        </button>
                    </div>
                </div>
                <p class="text-frkn-muted mt-2">
                    {move || t(lang.get(), "header_subtitle")}
                </p>
                <a
                    href="how.html"
                    class="inline-flex items-center gap-1 mt-3 text-sm text-frkn-accent hover:text-frkn-accent2 transition"
                >
                    {move || t(lang.get(), "how_we_measure")} <span>" ->"</span>
                </a>
            </header>

            // VPN / IP banner
            <section class="mb-8">
                <div class="frkn-card frkn-gradient-border rounded-2xl p-5 flex flex-col md:flex-row md:items-center justify-between gap-4">
                    <div class="flex items-center gap-4">
                        <div class="w-12 h-12 rounded-xl bg-frkn-accent/10 border border-frkn-accent/25 flex items-center justify-center text-xl">
                            {move || if is_vpn.get() { "🔒" } else { "⚠️" }}
                        </div>
                        <div>
                            <h2 class="text-lg font-bold">
                                {move || if is_vpn.get() { t(lang.get(), "vpn_detected") } else { t(lang.get(), "vpn_not_detected") }}
                            </h2>
                            <p class="text-frkn-muted text-sm">
                                {move || ip_info.get().map(|i| i.summary()).unwrap_or_else(|| t(lang.get(), "determining_ip").to_string())}
                            </p>
                        </div>
                    </div>
                    <div class="flex items-center gap-3">
                        <button
                            on:click=move |_| run_checks()
                            disabled=loading
                            class="px-5 py-2.5 rounded-xl bg-frkn-card border border-frkn-border hover:border-frkn-accent/40 disabled:opacity-50 disabled:cursor-not-allowed text-frkn-text font-semibold transition"
                        >
                            {move || if loading.get() { t(lang.get(), "checking").to_string() } else { t(lang.get(), "refresh").to_string() }}
                        </button>
                        {move || {
                            let frkn = is_frkn_vpn.get();
                            let vpn = is_vpn.get();
                            if frkn {
                                view! {
                                    <div class="flex items-center gap-2">
                                        <span class="inline-flex items-center px-3 py-2 rounded-full text-sm font-bold bg-frkn-accent/20 text-frkn-accent border border-frkn-accent/40 shadow-[0_0_15px_rgba(91,124,250,0.3)]">{move || t(lang.get(), "frkn_badge")}</span>
                                        <span class="inline-flex items-center px-3 py-2 rounded-full text-sm font-semibold bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">{move || t(lang.get(), "protected")}</span>
                                    </div>
                                }
                            } else if vpn {
                                view! {
                                    <div class="contents">
                                        <span class="inline-flex items-center px-3 py-2 rounded-full text-sm font-semibold bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">{move || t(lang.get(), "protected")}</span>
                                    </div>
                                }
                            } else {
                                view! {
                                    <div class="contents">
                                        <span class="inline-flex items-center px-3 py-2 rounded-full text-sm font-semibold bg-rose-500/10 text-rose-400 border border-rose-500/20">{move || t(lang.get(), "enable_vpn")}</span>
                                    </div>
                                }
                            }
                        }}
                    </div>
                </div>
            </section>

            // Speed test
            <section class="mb-10">
                <div class="frkn-card frkn-gradient-border rounded-2xl p-5">
                    <div class="flex flex-col md:flex-row md:items-center justify-between gap-4 mb-4">
                        <div>
                            <h3 class="text-xl font-bold">{move || t(lang.get(), "speed_title")}</h3>
                            <p class="text-frkn-muted text-sm">{move || t(lang.get(), "speed_subtitle")}</p>
                        </div>
                        <button
                            on:click=run_speed_test
                            disabled=is_speed_testing
                            class="frkn-btn px-5 py-2.5 rounded-xl text-white font-semibold disabled:opacity-50 disabled:cursor-not-allowed transition"
                        >
                            {move || if is_speed_testing.get() { t(lang.get(), "measuring").to_string() } else { t(lang.get(), "measure_speed").to_string() }}
                        </button>
                    </div>
                    {move || {
                        if is_speed_testing.get() {
                            view! { <div class="text-frkn-muted">{t(lang.get(), "measuring")}</div> }
                        } else if let Some(mbps) = speed_mbps.get() {
                            if mbps <= 0.0 {
                                view! { <div class="text-rose-400">{t(lang.get(), "speed_failed")}</div> }
                            } else {
                                view! {
                                    <div class="flex items-baseline gap-2">
                                        <span class="text-4xl font-extrabold frkn-gradient-text">{format!("{:.1}", mbps)}</span>
                                        <span class="text-frkn-muted font-medium">{t(lang.get(), "mbps")}</span>
                                    </div>
                                }
                            }
                        } else {
                            view! { <div class="text-frkn-muted">{t(lang.get(), "speed_prompt")}</div> }
                        }
                    }}
                </div>
            </section>
            // FRKN servers card
            <section class="mb-8">
                <div class={move || format!("frkn-card frkn-gradient-border rounded-2xl p-5 {}", frkn_aggregate.get().bg_class())}>
                    <div class="flex items-start justify-between gap-4 mb-4">
                        <div class="flex items-center gap-4">
                            <div class="w-12 h-12 rounded-xl bg-frkn-accent/10 border border-frkn-accent/25 flex items-center justify-center text-lg font-bold text-frkn-accent">
                                "F"
                            </div>
                            <div>
                                <h3 class="text-lg font-bold">{move || t(lang.get(), "frkn_servers_title")}</h3>
                                <p class="text-xs text-frkn-muted mt-0.5">
                                    {move || t(lang.get(), "frkn_servers_note")}
                                </p>
                                <div class="flex items-center gap-2 mt-1">
                                    <span class={move || format!("text-sm font-semibold {}", frkn_aggregate.get().color_class())}>
                                        {move || {
                                            if frkn_loading.get() {
                                                t(lang.get(), "checking").to_string()
                                            } else {
                                                frkn_aggregate.get().label_for(lang.get()).to_string()
                                            }
                                        }}
                                    </span>
                                    {move || {
                                        let total = frkn_nodes.get().len();
                                        let online = frkn_nodes.get().iter().filter(|n| n.browser_status == ServiceStatus::Online).count();
                                        if total > 0 && !frkn_loading.get() {
                                            Some(view! {
                                                <span class="text-sm text-frkn-muted">
                                                    {t(lang.get(), "of_available").replace("{online}", &online.to_string()).replace("{total}", &total.to_string())}
                                                </span>
                                            })
                                        } else {
                                            None
                                        }
                                    }}
                                </div>
                            </div>
                        </div>
                        <button
                            on:click=move |_| set_expanded.set(!expanded.get())
                            disabled=frkn_loading
                            class="shrink-0 inline-flex items-center gap-1 px-3 py-2 rounded-xl bg-frkn-card border border-frkn-border hover:border-frkn-accent/40 text-sm font-semibold transition disabled:opacity-50 disabled:cursor-not-allowed"
                        >
                            {move || {
                                let total = frkn_nodes.get().len();
                                if expanded.get() {
                                    t(lang.get(), "hide").to_string()
                                } else {
                                    t(lang.get(), "show_servers").replace("{total}", &total.to_string())
                                }
                            }}
                            <svg
                                xmlns="http://www.w3.org/2000/svg"
                                viewBox="0 0 20 20"
                                fill="currentColor"
                                class={move || format!("w-5 h-5 transition-transform {}", if expanded.get() { "rotate-180" } else { "" })}
                            >
                                <path fill-rule="evenodd" d="M5.23 7.21a.75.75 0 011.06.02L10 10.94l3.71-3.71a.75.75 0 111.06 1.06l-4.24 4.24a.75.75 0 01-1.06 0L5.21 8.29a.75.75 0 01.02-1.08z" clip-rule="evenodd" />
                            </svg>
                        </button>
                    </div>

                    // API row always visible
                    <div class="flex items-center justify-between p-3 rounded-xl bg-white/[0.03] border border-frkn-border">
                        <div class="flex items-center gap-2">
                            <span class="font-medium text-sm">{move || t(lang.get(), "api_label")}</span>
                            <span class="text-xs text-frkn-muted">{t(Lang::Ru, "api_hostname")}</span>
                        </div>
                        {move || {
                            let api = frkn_nodes.get()
                                .into_iter()
                                .find(|n| n.hostname == "api.frkn.org")
                                .map(|n| n.browser_status)
                                .unwrap_or(ServiceStatus::Checking);
                            let api2 = api.clone();
                            let lang_api = lang.get();
                            view! {
                                <span class={move || format!("text-sm font-semibold {}", api.color_class())}>
                                    {move || api2.label_for(lang_api)}
                                </span>
                            }
                        }}
                    </div>

                    // Expandable server list
                    {move || {
                        if expanded.get() {
                            Some(view! {
                                <div class="mt-4 space-y-3">
                                    {frkn_nodes.get()
                                        .into_iter()
                                        .filter(|n| n.hostname != "api.frkn.org")
                                        .map(|node| {
                                            let status_class = node.browser_status.color_class();
                                            let node_lang = lang.get();
                                            view! {
                                                <div class="p-3 rounded-xl bg-white/[0.03] border border-frkn-border">
                                                    <div class="flex items-center justify-between gap-2">
                                                        <div class="flex items-center gap-2 min-w-0">
                                                            <span class="text-sm font-medium truncate">{node.label}</span>
                                                            <span class="text-xs text-frkn-muted shrink-0">{node.country}</span>
                                                        </div>
                                                        <span class={format!("text-xs font-semibold shrink-0 {}", status_class)}>
                                                            {node.browser_status.label_for(node_lang)}
                                                        </span>
                                                    </div>
                                                    <div class="mt-1 flex items-center flex-wrap gap-2">
                                                        <span class="text-xs text-frkn-muted">{node.address}</span>
                                                        <span class="text-xs px-1.5 py-0.5 rounded bg-white/5 border border-frkn-border">
                                                            {format!("API: {}", node.api_status)}
                                                        </span>
                                                    </div>
                                                    {if !node.inbounds.is_empty() {
                                                        Some(view! {
                                                            <div class="mt-2 flex flex-wrap gap-2">
                                                                {node.inbounds.into_iter().map(|inb| {
                                                                    let dot_class = match inb.browser_status {
                                                                        ServiceStatus::Online => "bg-emerald-400",
                                                                        _ => "bg-rose-400",
                                                                    };
                                                                    view! {
                                                                        <span class="inline-flex items-center gap-1.5 px-2 py-1 rounded-md text-xs bg-black/20 border border-frkn-border">
                                                                            <span>{format!("{}:{}", inb.tag, inb.port)}</span>
                                                                            <span class={format!("w-2 h-2 rounded-full {}", dot_class)}></span>
                                                                        </span>
                                                                    }
                                                                }).collect_view()}
                                                            </div>
                                                        })
                                                    } else {
                                                        None
                                                    }}
                                                </div>
                                            }
                                        })
                                        .collect_view()
                                    }
                                </div>
                            })
                        } else {
                            None
                        }
                    }}
                </div>
            </section>

            // Error
            {move || error.get().map(|e| view! {
                <div class="mb-6 p-4 rounded-2xl bg-orange-500/10 border border-orange-500/30 text-orange-300">
                    {e}
                </div>
            })}


            // Store badges
            <section class="mb-8">
                <div class="frkn-card frkn-gradient-border rounded-2xl p-5">
                    <h3 class="text-lg font-bold mb-2">{move || t(lang.get(), "browser_extension_title")}</h3>
                    <p class="text-frkn-muted text-sm mb-4">
                        {move || t(lang.get(), "browser_extension_text")}
                    </p>
                    <div class="flex flex-col sm:flex-row flex-wrap gap-3">
                        <a
                            href=CHROME_STORE_URL
                            target="_blank"
                            rel="noopener noreferrer"
                            class="inline-flex items-center justify-center gap-2 px-4 py-2.5 rounded-xl bg-white text-slate-900 hover:bg-slate-100 font-semibold transition"
                        >
                            <img src="icons/googlechrome.svg" alt="Chrome" class="w-5 h-5" />
                            {move || t(lang.get(), "store_chrome")}
                        </a>
                        <a
                            href=FIREFOX_STORE_URL
                            target="_blank"
                            rel="noopener noreferrer"
                            class="inline-flex items-center justify-center gap-2 px-4 py-2.5 rounded-xl bg-white text-slate-900 hover:bg-slate-100 font-semibold transition"
                        >
                            <img src="icons/firefox.svg" alt="Firefox" class="w-5 h-5" />
                            {move || t(lang.get(), "store_firefox")}
                        </a>
                    </div>
                    <p class="text-xs text-frkn-muted mt-4">
                        {move || t(lang.get(), "zip_fallback")}
                    </p>
                    <div class="flex flex-col sm:flex-row gap-3 mt-3">
                        <a
                            href="frkn-service-checker-chrome.zip"
                            class="inline-flex items-center justify-center px-4 py-2 rounded-xl bg-frkn-card border border-frkn-border hover:border-frkn-accent/40 text-frkn-text text-sm font-semibold transition"
                        >
                            {move || t(lang.get(), "download_zip_chrome")}
                        </a>
                        <a
                            href="frkn-service-checker-firefox.zip"
                            class="inline-flex items-center justify-center px-4 py-2 rounded-xl bg-frkn-card border border-frkn-border hover:border-frkn-accent/40 text-frkn-text text-sm font-semibold transition"
                        >
                            {move || t(lang.get(), "download_zip_firefox")}
                        </a>
                    </div>
                </div>
            </section>

            // Footer
            <footer class="text-center text-frkn-muted text-sm pt-6 pb-8 border-t border-frkn-border">
                <div class="flex flex-col sm:flex-row items-center justify-center gap-4 mb-4">
                    <a
                        href="https://frkn.org"
                        target="_blank"
                        rel="noopener noreferrer"
                        class="inline-flex items-center gap-2 hover:text-frkn-accent transition"
                    >
                        {move || t(lang.get(), "footer_powered")} <span class="font-bold text-frkn-text">"FRKN"</span>
                    </a>
                    <span class="hidden sm:inline">"•"</span>
                    <a
                        href="https://github.com/frkn-dev/status-page"
                        target="_blank"
                        rel="noopener noreferrer"
                        class="inline-flex items-center gap-2 hover:text-frkn-accent transition"
                    >
                        <img src="icons/github.svg" alt="GitHub" class="w-4 h-4" />
                        "GitHub"
                    </a>
                    <span class="hidden sm:inline">"•"</span>
                    <a href="privacy.html" class="hover:text-frkn-accent transition">{move || t(lang.get(), "privacy_ru")}</a>
                    <a href="privacy-en.html" class="hover:text-frkn-accent transition">{move || t(lang.get(), "privacy_en")}</a>
                </div>
            </footer>
        </div>
    }
}
