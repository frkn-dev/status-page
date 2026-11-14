use crate::i18n::{t, Lang};
use crate::services::ServiceStatus;
use futures::future::{join_all, select, Either};
use gloo_net::http::Request;
use gloo_timers::future::TimeoutFuture;
use js_sys::Promise;
use serde::{Deserialize, Serialize};
use std::f64;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use wasm_bindgen_futures::JsFuture;
use web_sys::Performance;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct IpInfo {
    pub ip: String,
    pub city: String,
    pub region: String,
    pub country_name: String,
    pub country_code: String,
    pub org: String,
    pub asn: String,
    /// Прямой флаг VPN/proxy/hosting/datacenter от IP-сервиса (если есть).
    pub vpn_flag: bool,
}

impl IpInfo {
    pub fn summary(&self) -> String {
        let mut parts = vec![self.ip.clone()];
        if !self.city.is_empty() {
            parts.push(self.city.clone());
        }
        if !self.country_name.is_empty() {
            parts.push(self.country_name.clone());
        }
        if !self.org.is_empty() {
            parts.push(self.org.clone());
        }
        parts.join(" • ")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FrknNode {
    pub hostname: String,
    pub address: String,
    pub label: String,
    pub country: String,
    pub api_status: String,
    pub node_type: String,
    pub ports: Vec<u16>,
    /// Pool identifier straight from the node config (`[node] pool`), verbatim.
    pub pool: Option<String>,
    /// Localized pool titles (`[node] pool_translation`), lang -> title.
    pub pool_translation: Option<BTreeMap<String, String>>,
    /// Bench base URL straight from the api (`speedtest_url`, 0.6.134+).
    /// The api owns the name; the page never derives it. `None` = old api or a
    /// node without a bench.
    pub speedtest_url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FrknPortStatus {
    pub port: u16,
    pub browser_status: ServiceStatus,
    pub latency_ms: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FrknNodeStatus {
    pub label: String,
    pub hostname: String,
    pub address: String,
    pub country: String,
    pub api_status: String,
    pub node_type: String,
    pub browser_status: ServiceStatus,
    pub latency_ms: Option<u32>,
    pub ports: Vec<FrknPortStatus>,
    pub pool: Option<String>,
    pub pool_translation: Option<BTreeMap<String, String>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FrknAggregateStatus {
    AllOnline,
    Partial,
    AllOffline,
    Error,
}

impl FrknAggregateStatus {
    pub fn label_for(&self, lang: Lang) -> &'static str {
        let key = match self {
            FrknAggregateStatus::AllOnline => "aggregate_all_online",
            FrknAggregateStatus::Partial => "aggregate_partial",
            FrknAggregateStatus::AllOffline => "aggregate_all_offline",
            FrknAggregateStatus::Error => "aggregate_error",
        };
        t(lang, key)
    }

    pub fn color_class(&self) -> &'static str {
        match self {
            FrknAggregateStatus::AllOnline => "text-emerald-400",
            FrknAggregateStatus::Partial => "text-yellow-400",
            FrknAggregateStatus::AllOffline => "text-rose-400",
            FrknAggregateStatus::Error => "text-orange-400",
        }
    }

    pub fn bg_class(&self) -> &'static str {
        match self {
            FrknAggregateStatus::AllOnline => "bg-emerald-500/[0.08] border-emerald-500/30",
            FrknAggregateStatus::Partial => "bg-yellow-500/[0.08] border-yellow-500/30",
            FrknAggregateStatus::AllOffline => "bg-rose-500/[0.08] border-rose-500/30",
            FrknAggregateStatus::Error => "bg-orange-500/[0.08] border-orange-500/30",
        }
    }
}

pub async fn fetch_frkn_nodes() -> Result<Vec<FrknNode>, String> {
    let resp = Request::get("https://api.frkn.org/status")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let nodes: Vec<FrknNode> = json["response"]
        .as_array()
        .ok_or("invalid response format")?
        .iter()
        .filter_map(|n| {
            Some(FrknNode {
                hostname: n["hostname"].as_str()?.to_string(),
                address: n["address"].as_str()?.to_string(),
                label: n["label"].as_str().unwrap_or("").to_string(),
                country: n["country"].as_str().unwrap_or("").to_string(),
                api_status: n["status"].as_str().unwrap_or("Unknown").to_string(),
                node_type: n["type"].as_str().unwrap_or("").to_string(),
                ports: n["ports"]
                    .as_array()
                    .map(|arr| arr.iter().filter_map(|p| p.as_u64().map(|v| v as u16)).collect())
                    .unwrap_or_default(),
                speedtest_url: n["speedtest_url"].as_str().map(str::to_string),
                pool: n["pool"].as_str().map(str::to_string),
                pool_translation: n["pool_translation"].as_object().map(|m| {
                    m.iter()
                        .filter_map(|(k, v)| v.as_str().map(|t| (k.clone(), t.to_string())))
                        .collect()
                }),
            })
        })
        .collect();

    // Remember which node carries which bench URL: measure_* work from a
    // hostname, and the URL must come from the api, never from the name.
    BENCH_URLS.with(|m| {
        let mut m = m.borrow_mut();
        m.clear();
        for n in &nodes {
            if let Some(url) = &n.speedtest_url {
                m.insert(n.hostname.clone(), url.clone());
            }
        }
    });

    Ok(nodes)
}

pub fn is_frkn_ip(ip: &str, nodes: &[FrknNode]) -> bool {
    nodes.iter().any(|n| n.address == ip)
}

pub async fn check_frkn_nodes(nodes: &[FrknNode]) -> (FrknAggregateStatus, Vec<FrknNodeStatus>) {
    // Проверяем все ноды параллельно, чтобы не ждать по очереди.
    // Премиум-ноды исключаем из публичной карточки статуса.
    let checkable_nodes: Vec<&FrknNode> = nodes
        .iter()
        .filter(|n| n.node_type != "PremiumNode")
        .collect();

    // One port per node and 8 nodes at a time: with ~100 sockets in flight the
    // browser queues connections and the deadline fires on untouched sockets,
    // which showed a different set of nodes as "not answering" on every run.
    let mut statuses: Vec<FrknNodeStatus> = Vec::new();
    for chunk in checkable_nodes.chunks(16) {
        let futures = chunk.iter().map(|node| async move {
            let url = format!("https://{}/favicon.ico", node.hostname);
            let (favicon_status, favicon_latency) = check_frkn_node(&url).await;
            let ports: Vec<FrknPortStatus> = match node.ports.first() {
                Some(&port) => {
                    let (p_status, p_latency) = check_port(&node.hostname, port).await;
                    vec![FrknPortStatus { port, browser_status: p_status, latency_ms: p_latency }]
                }
                None => Vec::new(),
            };

            let answered = ports
                .iter()
                .any(|p| p.browser_status == ServiceStatus::Online);
            let (browser_status, latency_ms) = if answered {
                let p_latency = ports.iter().filter_map(|p| p.latency_ms).min();
                (ServiceStatus::Online, p_latency.or(favicon_latency))
            } else if favicon_status == ServiceStatus::Online {
                (ServiceStatus::Online, favicon_latency)
            } else {
                // Nothing answered — but api may still report Online; the UI shows
                // both, so this is "silent from here", not "down".
                (ServiceStatus::Offline, None)
            };

            FrknNodeStatus {
                label: node.label.clone(),
                hostname: node.hostname.clone(),
                address: node.address.clone(),
                country: node.country.clone(),
                api_status: node.api_status.clone(),
                node_type: node.node_type.clone(),
                browser_status,
                latency_ms,
                ports,
                pool: node.pool.clone(),
                pool_translation: node.pool_translation.clone(),
            }
        });
        statuses.extend(join_all(futures).await);
    }


    // Проверяем api.frkn.org отдельно через публичный /healthcheck
    let api_url = "https://api.frkn.org/healthcheck";
    let (api_status, api_latency) = check_frkn_node(api_url).await;
    statuses.push(FrknNodeStatus {
        label: "API".to_string(),
        hostname: "api.frkn.org".to_string(),
        address: "api.frkn.org".to_string(),
        country: String::new(),
        api_status: "API".to_string(),
        node_type: "API".to_string(),
        browser_status: api_status,
        latency_ms: api_latency,
        ports: Vec::new(),
        pool: None,
        pool_translation: None,
    });

    let online_count = statuses
        .iter()
        .filter(|s| s.browser_status == ServiceStatus::Online)
        .count();
    let total = statuses.len();
    let aggregate = if total == 0 {
        FrknAggregateStatus::AllOffline
    } else if online_count == total {
        FrknAggregateStatus::AllOnline
    } else if online_count == 0 {
        FrknAggregateStatus::AllOffline
    } else {
        FrknAggregateStatus::Partial
    };

    (aggregate, statuses)
}

async fn check_frkn_node(url: &str) -> (ServiceStatus, Option<u32>) {
    let cache_buster = format!("{}?t={}", url, now_ms() as u64);
    match check_with_fetch(&cache_buster, 4_000).await {
        Ok(latency) => (ServiceStatus::Online, Some(latency)),
        Err(_) => (ServiceStatus::Offline, None),
    }
}

/// Проверяет доступность TCP-порта ноды из браузера.
/// Из обычной веб-страницы нельзя открыть raw TCP/UDP, поэтому
/// используем WebSocket handshake (`wss://hostname:port/`). Если порт
/// открыт и принимает TLS, handshake завершится событием open/error
/// до таймаута — это означает, что порт достижим. Если порт закрыт
/// или блокируется, браузер дождётся таймаута.
async fn check_port(hostname: &str, port: u16) -> (ServiceStatus, Option<u32>) {
    // A browser cannot tell "closed" from "filtered": both can end in onerror,
    // and a queued socket can simply miss a short deadline. So the result is
    // "answered within the deadline" or "silent" — never "unavailable".
    let url = format!("wss://{}:{}/", hostname, port);
    let start = now_ms();
    let ws = match web_sys::WebSocket::new(&url) {
        Ok(ws) => ws,
        Err(_) => return (ServiceStatus::Error, None),
    };

    let (tx, rx) = futures::channel::oneshot::channel::<bool>();
    let tx = Rc::new(RefCell::new(Some(tx)));

    let tx_event = tx.clone();
    let on_event = Closure::once(Box::new(move || {
        if let Some(t) = tx_event.borrow_mut().take() {
            let _ = t.send(true);
        }
    }));
    ws.set_onopen(Some(on_event.as_ref().unchecked_ref()));
    ws.set_onerror(Some(on_event.as_ref().unchecked_ref()));
    on_event.forget();

    let tx_timeout = tx.clone();
    spawn_local(async move {
        TimeoutFuture::new(6_000).await;
        if let Some(t) = tx_timeout.borrow_mut().take() {
            let _ = t.send(false);
        }
    });

    let reachable = match rx.await {
        Ok(v) => v,
        Err(_) => false,
    };
    let elapsed = ((now_ms() - start).max(0.0)) as u32;
    let _ = ws.close();

    if reachable {
        (ServiceStatus::Online, Some(elapsed.max(1)))
    } else {
        (ServiceStatus::Offline, None)
    }
}

pub async fn fetch_ip_info() -> Result<IpInfo, String> {
    // Запрашиваем все сервисы параллельно, чтобы не упустить VPN-флаги
    // из одного сервиса, если другой ответил быстрее без них.
    let (ipapi, ipwho, ipinfo) = futures::join!(
        fetch_ip_ipapi(),
        fetch_ip_ipwhois(),
        fetch_ip_ipinfo()
    );

    let mut results: Vec<IpInfo> = Vec::new();
    if let Ok(info) = ipapi {
        results.push(info);
    } else if let Err(e) = ipapi {
        web_sys::console::warn_1(&format!("ipapi failed: {e}").into());
    }
    if let Ok(info) = ipwho {
        results.push(info);
    } else if let Err(e) = ipwho {
        web_sys::console::warn_1(&format!("ipwho.is failed: {e}").into());
    }
    if let Ok(info) = ipinfo {
        results.push(info);
    } else if let Err(e) = ipinfo {
        web_sys::console::warn_1(&format!("ipinfo.io failed: {e}").into());
    }

    if results.is_empty() {
        return Err("Не удалось определить IP ни на одном из сервисов".to_string());
    }

    // Для отображения используем первый успешный (приоритет ipapi → ipwho → ipinfo).
    let mut display = results[0].clone();

    // VPN считаем обнаруженным, если хотя бы один сервис выставил флаг
    // или хотя бы у одного провайдер/ASN похож на VPN/дата-центр.
    let any_vpn_flag = results.iter().any(|i| i.vpn_flag);
    let any_keyword = results.iter().any(|i| detect_vpn(i));
    display.vpn_flag = display.vpn_flag || any_vpn_flag || any_keyword;

    Ok(display)
}

async fn fetch_ip_ipapi() -> Result<IpInfo, String> {
    let resp = Request::get("https://ipapi.co/json/")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    let threat = &json["threat"];
    Ok(IpInfo {
        ip: json["ip"].as_str().unwrap_or("").to_string(),
        city: json["city"].as_str().unwrap_or("").to_string(),
        region: json["region"].as_str().unwrap_or("").to_string(),
        country_name: json["country_name"].as_str().unwrap_or("").to_string(),
        country_code: json["country_code"].as_str().unwrap_or("").to_string(),
        org: json["org"].as_str().unwrap_or("").to_string(),
        asn: json["asn"].as_str().unwrap_or("").to_string(),
        vpn_flag: threat["is_anonymous"].as_bool() == Some(true)
            || threat["is_proxy"].as_bool() == Some(true)
            || threat["is_vpn"].as_bool() == Some(true),
    })
}

async fn fetch_ip_ipwhois() -> Result<IpInfo, String> {
    let resp = Request::get("https://ipwho.is/")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    let security = &json["security"];
    Ok(IpInfo {
        ip: json["ip"].as_str().unwrap_or("").to_string(),
        city: json["city"].as_str().unwrap_or("").to_string(),
        region: json["region"].as_str().unwrap_or("").to_string(),
        country_name: json["country"].as_str().unwrap_or("").to_string(),
        country_code: json["country_code"].as_str().unwrap_or("").to_string(),
        org: json["connection"]["org"]
            .as_str()
            .or_else(|| json["org"].as_str())
            .unwrap_or("")
            .to_string(),
        asn: json["connection"]["asn"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        vpn_flag: security["vpn"].as_bool() == Some(true)
            || security["proxy"].as_bool() == Some(true)
            || security["datacenter"].as_bool() == Some(true)
            || security["tor"].as_bool() == Some(true),
    })
}

async fn fetch_ip_ipinfo() -> Result<IpInfo, String> {
    let resp = Request::get("https://ipinfo.io/json")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    let privacy = &json["privacy"];
    Ok(IpInfo {
        ip: json["ip"].as_str().unwrap_or("").to_string(),
        city: json["city"].as_str().unwrap_or("").to_string(),
        region: json["region"].as_str().unwrap_or("").to_string(),
        country_name: json["country"].as_str().unwrap_or("").to_string(),
        country_code: json["country"].as_str().unwrap_or("").to_string(),
        org: json["org"].as_str().unwrap_or("").to_string(),
        asn: json["asn"]
            .as_str()
            .or_else(|| json["org"].as_str())
            .unwrap_or("")
            .to_string(),
        vpn_flag: privacy["vpn"].as_bool() == Some(true)
            || privacy["proxy"].as_bool() == Some(true)
            || privacy["hosting"].as_bool() == Some(true)
            || privacy["tor"].as_bool() == Some(true),
    })
}

pub fn detect_vpn(info: &IpInfo) -> bool {
    // 1. Прямые флаги от IP-сервисов (vpn/proxy/hosting/datacenter/tor).
    if info.vpn_flag {
        return true;
    }

    // 2. Эвристика по названию провайдера/ASN.
    let org_lower = info.org.to_lowercase();
    let asn_lower = info.asn.to_lowercase();
    [
        "vpn",
        "proxy",
        "hosting",
        "datacenter",
        "cloud",
        "server",
        "vps",
        "dedicated",
        "teleport",
        "outline",
        "wireguard",
        "openvpn",
        "m247",
        "ovh",
        "hetzner",
        "digitalocean",
        "linode",
        "amazon",
        "aws",
        // Популярные VPN/прокси и антивирусы с VPN
        "mullvad",
        "nord",
        "nordvpn",
        "expressvpn",
        "surfshark",
        "proton",
        "protonvpn",
        "windscribe",
        "cyberghost",
        "private internet access",
        "pia",
        "hotspot shield",
        "tunnelbear",
        "hide.me",
        "hideme",
        "torguard",
        "ivpn",
        "airvpn",
        "zoogvpn",
        "wevpn",
        "purevpn",
        "strongvpn",
        "vyprvpn",
        "ipvanish",
        "kaspersky",
        "avast",
        "avg",
        "bitdefender",
        "mcafee",
        "norton",
        "whoer",
        "frkn",
    ]
    .iter()
    .any(|&keyword| org_lower.contains(keyword) || asn_lower.contains(keyword))
}

fn now_ms() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p: Performance| p.now())
        .unwrap_or(0.0)
}

async fn check_with_fetch(url: &str, timeout_ms: u32) -> Result<u32, String> {
    let start = now_ms();

    let opts = web_sys::RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(web_sys::RequestMode::NoCors);

    let web_req = web_sys::Request::new_with_str_and_init(url, &opts)
        .map_err(|e| format!("request init failed: {e:?}"))?;

    let window = web_sys::window().ok_or("no window")?;
    let promise: Promise = window.fetch_with_request(&web_req);

    fetch_with_timeout(promise, timeout_ms).await?;
    let elapsed = ((now_ms() - start).max(0.0)) as u32;
    Ok(elapsed.max(1))
}

async fn fetch_with_timeout(promise: Promise, timeout_ms: u32) -> Result<u32, String> {
    let timeout_promise = Promise::new(&mut |resolve, _reject| {
        let window = web_sys::window().expect("no window");
        let args = js_sys::Array::new();
        args.push(&JsValue::from_str("timeout"));
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments(
            &resolve,
            timeout_ms as i32,
            &args,
        );
    });

    let result = js_sys::Promise::race(&js_sys::Array::of2(&promise, &timeout_promise));

    let value = JsFuture::from(result).await.map_err(|e| {
        let msg = e.as_string().unwrap_or_else(|| "fetch error".to_string());
        msg
    })?;

    if value.as_string().as_deref() == Some("timeout") {
        return Err("timeout".to_string());
    }

    Ok(0)
}

pub async fn measure_speed() -> Result<f64, String> {
    let timestamp = now_ms() as u64;

    // Сначала пробуем собственные файлы на status.frkn.org (тот же origin — нет CORS).
    let urls_10mb = [
        format!("https://status.frkn.org/speedtest/10mb.test?cb={}", timestamp),
        format!("https://proof.ovh.net/files/10Mb.dat?cb={}", timestamp),
        format!("https://cachefly.cachefly.net/10mb.test?cb={}", timestamp),
    ];

    for url in urls_10mb {
        if let Ok((speed, _)) = measure_speed_url(&url, 15_000).await {
            return Ok(speed);
        }
    }

    // Fallback на 1 МБ, если 10 МБ недоступны или слишком медленные.
    let urls_1mb = [
        format!("https://status.frkn.org/speedtest/1mb.test?cb={}", timestamp),
        format!("https://proof.ovh.net/files/1Mb.dat?cb={}", timestamp),
        format!("https://cachefly.cachefly.net/1mb.test?cb={}", timestamp),
    ];

    for url in urls_1mb {
        if let Ok((speed, _)) = measure_speed_url(&url, 15_000).await {
            return Ok(speed);
        }
    }

    Err("Не удалось измерить скорость ни на одном из зеркал".to_string())
}

async fn measure_speed_url(url: &str, timeout_ms: u32) -> Result<(f64, f64), String> {
    let start = now_ms();

    // Не добавляем кастомные заголовки, иначе браузер делает CORS-preflight,
    // который часто не проходит на публичных speedtest-зеркалах.
    // Cache-busting через ?cb=... достаточно.
    let send_fut = Request::get(url).send();

    let resp = match select(Box::pin(send_fut), TimeoutFuture::new(timeout_ms)).await {
        // A failed fetch covers DNS, refused connection and a blocked port alike:
        // the browser hides which one it was, so the UI says "нет связи".
        Either::Left((result, _)) => result.map_err(|_| "network".to_string())?,
        Either::Right(_) => return Err("timeout".to_string()),
    };

    if !resp.ok() {
        return Err(format!("http {}", resp.status()));
    }

    let bytes = resp.binary().await.map_err(|_| "body".to_string())?;
    let elapsed_sec = (now_ms() - start) / 1000.0;
    if elapsed_sec <= 0.0 {
        return Err("too fast".to_string());
    }

    let bits = (bytes.len() as f64) * 8.0;
    let mbps = bits / elapsed_sec / 1_000_000.0;
    Ok((mbps, (now_ms() - start)))
}

/// Bench path a node is expected to serve for per-node speed measurement.
/// Same convention as the files on status.frkn.org: an nginx/web root with
/// `Access-Control-Allow-Origin: *`, `Cache-Control: no-store` and a 10 MB
/// (fallback 1 MB) non-compressible payload.
const BENCH_10MB: &str = "/speedtest/10mb.test";
const BENCH_1MB: &str = "/speedtest/1mb.test";

/// Title of a pool for the active language: the translation when the node
/// carries one, otherwise the pool id verbatim (the id is the contract).
pub fn pool_title(pool: &str, translation: Option<&BTreeMap<String, String>>, lang: Lang) -> String {
    let code = match lang {
        Lang::Ru => "ru",
        Lang::En => "en",
    };
    translation
        .and_then(|m| m.get(code))
        .cloned()
        .unwrap_or_else(|| pool.to_string())
}

/// Median of the measured speeds. `None` when nothing was measured, so the UI
/// can say "измерилось 0 из N" instead of inventing a zero.
#[allow(dead_code)]
pub fn median(values: &[f64]) -> Option<f64> {
    let mut v: Vec<f64> = values.iter().copied().filter(|x| x.is_finite() && *x > 0.0).collect();
    if v.is_empty() {
        return None;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = v.len() / 2;
    Some(if v.len() % 2 == 0 { (v[mid - 1] + v[mid]) / 2.0 } else { v[mid] })
}

/// Per-node bench origin on the dedicated bench port. Names follow the zone
/// template `<hoster>-<cc>-<city>-<purpose>-<nn>` (see `beacon.frkn.org`), so a
/// node cannot be found by guessing its hostname: the map below is the source.
/// Nodes without an entry fall back to `<first-label>.bench`, the old scheme.
/// The wildcard cert `*.bench.beacon.frkn.org` covers every name; the cert scope
/// stays bench-only so a pool node cannot MITM premium H2 SNI or irc/chat.
const BENCH_PORT: u16 = 19443;
const BENCH_DOMAIN: &str = "bench.beacon.frkn.org";

const BENCH_HOSTS: &[(&str, &str)] = &[
    ("at.frkn.org", "mis-at-vie-bench-01"),
    ("ee.frkn.org", "soul-ee-tll-bench-01"),
    ("eu01.frkn.org", "ncr-nl-ams-bench-01"),
    ("eu02.frkn.org", "aeza-nl-ams-bench-01"),
    ("fr.frkn.org", "aeza-fr-par-bench-01"),
    ("pl.frkn.org", "fbw-fr-par-bench-01"),
    ("tr.frkn.org", "hk-tr-ist-bench-01"),
    ("msk-tw.frkn.org", "tw-ru-msk-bench-01"),
    ("msk-tw-nvsb.frkn.org", "adm-ru-nsk-bench-01"),
    ("msk-vdska.frkn.org", "vdska-ru-msk-bench-01"),
];

thread_local! {
    /// hostname -> bench base URL as reported by the api.
    static BENCH_URLS: std::cell::RefCell<std::collections::HashMap<String, String>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

fn bench_origin(hostname: &str) -> String {
    // 1. the api's value (single source of truth)
    if let Some(url) = BENCH_URLS.with(|m| m.borrow().get(hostname).cloned()) {
        return url;
    }
    // 2. local map, kept until api 0.6.134 is everywhere
    let label = BENCH_HOSTS
        .iter()
        .find(|(host, _)| *host == hostname)
        .map(|(_, label)| *label)
        .unwrap_or_else(|| hostname.split('.').next().unwrap_or(hostname));
    format!("https://{label}.{BENCH_DOMAIN}:{BENCH_PORT}")
}

/// Speed from THIS device to one node, downloading its bench file directly.
/// Measures the user's own path to that node, not a third-party mirror.
pub async fn measure_node_speed(hostname: &str) -> Result<f64, String> {
    let ts = now_ms() as u64;
    let origin = bench_origin(hostname);
    // Probe with 1 MB: a 10 MB body times out on slow links and only doubles the
    // wait. Switch to 10 MB only when 1 MB arrived fast enough for the bigger
    // sample to be accurate instead of just slow.
    let (small_mbps, small_ms) =
        measure_speed_url(&format!("{origin}{BENCH_1MB}?cb={ts}"), 6_000).await?;
    if small_ms < 900.0 {
        if let Ok((big_mbps, _)) =
            measure_speed_url(&format!("{origin}{BENCH_10MB}?cb={ts}"), 12_000).await
        {
            return Ok(big_mbps);
        }
    }
    Ok(small_mbps)
}

/// Round-trip latency from this device to one node's bench endpoint.
///
/// Uses the 1-byte `ping.txt` (a plain GET — a Range header would force a CORS
/// preflight and double the measured time). Returns milliseconds.
pub async fn measure_node_ping(hostname: &str) -> Option<(u32, u32)> {
    let origin = bench_origin(hostname);
    // Every request carries its own timeout: on a blocked path the browser's
    // default fetch timeout is tens of seconds, which made a whole pool run look
    // hung (ping has no deadline of its own).
    const PING_TIMEOUT_MS: u32 = 4_000;
    async fn one(url: &str, timeout_ms: u32) -> Option<()> {
        let send_fut = gloo_net::http::Request::get(url).send();
        match select(Box::pin(send_fut), TimeoutFuture::new(timeout_ms)).await {
            Either::Left((Ok(res), _)) if res.ok() => {
                res.binary().await.ok()?;
                Some(())
            }
            _ => None,
        }
    }
    // Warm DNS + TCP + TLS so the samples below are real round trips, not the
    // handshake (the cold request reads as 3-4x the path RTT).
    one(&format!("{origin}/speedtest/ping.txt?warm=1"), PING_TIMEOUT_MS).await?;
    let mut samples: Vec<u32> = Vec::new();
    for i in 0..3 {
        let url = format!("{origin}/speedtest/ping.txt?cb={}-{}", now_ms() as u64, i);
        let start = now_ms();
        one(&url, PING_TIMEOUT_MS).await?;
        samples.push((now_ms() - start).max(0.0) as u32);
    }
    let best = *samples.iter().min()?;
    let worst = *samples.iter().max()?;
    // spread of the three samples: the "is it stable for gaming" signal
    Some((best, worst.saturating_sub(best)))
}

/// IPv6 reachability from this device. If it answers while the VPN only carries
/// IPv4, that is the classic IPv6 leak: the traffic leaves the tunnel.
pub async fn probe_ipv6() -> Option<String> {
    let send_fut = gloo_net::http::Request::get("https://ipv6.icanhazip.com").send();
    let resp = match select(Box::pin(send_fut), TimeoutFuture::new(4_000)).await {
        Either::Left((Ok(r), _)) if r.ok() => r,
        _ => return None,
    };
    let text = resp.text().await.ok()?;
    let ip = text.trim().to_string();
    if ip.contains(':') { Some(ip) } else { None }
}

/// IPs leaked by WebRTC ICE candidates (STUN). The browser may reveal the real
/// provider address even when the page itself goes through the tunnel.
pub async fn probe_webrtc() -> Vec<String> {
    let js = r#"
      (async () => {
        const ips = new Set();
        try {
          const pc = new RTCPeerConnection({iceServers:[{urls:'stun:stun.l.google.com:19302'}]});
          pc.onicecandidate = e => {
            if (!e.candidate || !e.candidate.candidate) return;
            const m = /([0-9]{1,3}(\.[0-9]{1,3}){3}|[a-f0-9]{1,4}(:[a-f0-9]{0,4}){2,7})/i.exec(e.candidate.candidate);
            if (m) ips.add(m[1]);
          };
          pc.createDataChannel('probe');
          await pc.setLocalDescription(await pc.createOffer());
          await new Promise(r => setTimeout(r, 2500));
          pc.close();
        } catch (e) { /* no RTCPeerConnection: nothing to leak this way */ }
        return JSON.stringify([...ips]);
      })()
    "#;
    let promise = match js_sys::eval(js) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let value = match wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(promise)).await {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let raw = value.as_string().unwrap_or_default();
    serde_json::from_str::<Vec<String>>(&raw).unwrap_or_default()
}

// ---------------------------------------------------------------- dns leak

/// One resolver that asked our authoritative server for the probe name.
#[derive(Clone, Debug, PartialEq)]
pub struct DnsResolverHit {
    pub ip: String,
    pub country: String,
    pub org: String,
    pub seen: u32,
}

/// Probes go to `*.<token>.leakprobe.beacon.frkn.org`, a zone delegated to our own
/// logger (`dnsleak` on darkmachine2). Its result API is exposed through
/// api.frkn.org/leak/ — that host already has a certificate and CORS setup that the
/// browser accepts (a fresh beacon name was rejected by Chrome in practice).
const LEAK_RESULT: &str = "https://api.frkn.org/leak/r/";
const LEAK_PROBE_ZONE: &str = "leakprobe.beacon.frkn.org";

pub fn probe_token() -> String {
    // 10 chars from the clock: unique enough per run, no crypto needed
    let mut n = (now_ms() as u64) ^ 0x9e37_79b9_7f4a_7c15;
    let alphabet = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut out = String::with_capacity(10);
    for _ in 0..10 {
        out.push(alphabet[(n % alphabet.len() as u64) as usize] as char);
        n = n / 7 + 0x2545_f491;
    }
    out
}

/// Ask the browser to resolve five random names under our logger zone, then read
/// back which resolvers were seen. The lookups themselves fail (nothing serves
/// them), but the DNS query has already reached our server — that is the point.
/// What a neutral resolver sees for our own canary. Our authoritative server
/// answers 127.0.0.1 for every probe name; if Google's DoH says something else,
/// the answer was rewritten on the way (fake-ip / DNS hijack).
pub async fn doh_canary(token: &str) -> Result<Vec<String>, String> {
    let url = format!(
        "https://dns.google/resolve?name=canary.{token}.{LEAK_PROBE_ZONE}&type=A"
    );
    let resp = gloo_net::http::Request::get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    Ok(json["Answer"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|a| a["type"].as_u64() == Some(1))
        .filter_map(|a| a["data"].as_str().map(str::to_string))
        .collect())
}

pub async fn dns_leak_probe(token: &str) -> Result<Vec<DnsResolverHit>, String> {
    for i in 0..5 {
        let host = format!("q{i}{}.{token}.{LEAK_PROBE_ZONE}", (now_ms() as u64) % 100000);
        // Image is the cheapest way to force a DNS lookup without CORS noise.
        let _ = js_sys::eval(&format!(
            "(() => {{ const im = new Image(); im.src = 'https://{host}/x'; }})()"
        ));
    }
    TimeoutFuture::new(2_000).await;

    // The resolver chain needs a moment: poll instead of reading once, otherwise
    // a slow upstream shows up as "no queries arrived".
    let url = format!("{LEAK_RESULT}{token}");
    let mut list: Vec<DnsResolverHit> = Vec::new();
    for _ in 0..8 {
        TimeoutFuture::new(1_500).await;
        let req = match gloo_net::http::Request::get(&url).send().await {
            Ok(r) => r,
            Err(_) => continue,
        };
        if !req.ok() {
            continue;
        }
        let json: serde_json::Value = match req.json().await {
            Ok(v) => v,
            Err(_) => continue,
        };
        list = json["resolvers"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|r| DnsResolverHit {
                ip: r["ip"].as_str().unwrap_or_default().to_string(),
                country: r["country"].as_str().unwrap_or_default().to_string(),
                org: r["org"].as_str().unwrap_or_default().to_string(),
                seen: r["seen"].as_u64().unwrap_or(0) as u32,
            })
            .collect();
        if !list.is_empty() {
            break;
        }
    }
    Ok(list)
}

// ---------------------------------------------------------------- mtu

/// POST bodies of increasing size: detects an MTU/blackhole problem where pages
/// open but large transfers hang (the classic "sites load, nothing downloads").
/// Returns (size, delivered) per step.
pub async fn mtu_probe() -> Vec<(usize, bool)> {
    let body = "x".repeat(1600);
    let mut out = Vec::new();
    for size in [1200usize, 1400, 1500] {
        let url = "https://api.frkn.org/leak/upload";
        let fut = gloo_net::http::Request::post(url)
            .header("Content-Type", "text/plain")
            .body(body[..size].to_string())
            .map_err(|e| e.to_string());
        let ok = match fut {
            Ok(req) => {
                match select(Box::pin(req.send()), TimeoutFuture::new(8_000)).await {
                    Either::Left((Ok(r), _)) => r.ok(),
                    _ => false,
                }
            }
            Err(_) => false,
        };
        out.push((size, ok));
    }
    out
}
