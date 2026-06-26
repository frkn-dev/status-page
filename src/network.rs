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
pub struct FrknInbound {
    pub tag: String,
    pub port: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FrknNode {
    pub hostname: String,
    pub address: String,
    pub label: String,
    pub country: String,
    pub api_status: String,
    pub node_type: String,
    pub inbounds: Vec<FrknInbound>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FrknInboundStatus {
    pub tag: String,
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
    pub inbounds: Vec<FrknInboundStatus>,
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
    let resp = Request::get("https://api.frkn.org/nodes")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let nodes = json["response"]
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
                inbounds: n["inbounds"].as_array().map(|arr| {
                    arr.iter().filter_map(|i| {
                        Some(FrknInbound {
                            tag: i["tag"].as_str()?.to_string(),
                            port: i["port"].as_u64()? as u16,
                        })
                    }).collect()
                }).unwrap_or_default(),
            })
        })
        .collect();

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

    let node_futures = checkable_nodes.into_iter().map(|node| async move {
        let url = format!("https://{}/favicon.ico", node.hostname);
        let (favicon_status, favicon_latency) = check_frkn_node(&url).await;
        let inbound_futures = node.inbounds.iter().map(|inb| async move {
            let (in_status, in_latency) = check_inbound_port(&node.hostname, inb.port).await;
            FrknInboundStatus {
                tag: inb.tag.clone(),
                port: inb.port,
                browser_status: in_status,
                latency_ms: in_latency,
            }
        });
        let inbounds = join_all(inbound_futures).await;

        // Общий статус ноды = online, если хотя бы один инбаунд доступен
        // или отвечает HTTP/фавикон. Favicon часто не отдаётся на Reality-нодах,
        // поэтому инбаунды являются более надёжным критерием.
        let inbound_online = inbounds
            .iter()
            .any(|i| i.browser_status == ServiceStatus::Online);
        let (browser_status, latency_ms) = if inbound_online {
            let in_latency = inbounds
                .iter()
                .filter_map(|i| i.latency_ms)
                .min();
            (ServiceStatus::Online, in_latency.or(favicon_latency))
        } else if favicon_status == ServiceStatus::Online {
            (ServiceStatus::Online, favicon_latency)
        } else {
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
            inbounds,
        }
    });

    let mut statuses = join_all(node_futures).await;

    // Проверяем api.frkn.org отдельно через основной endpoint /nodes
    let api_url = "https://api.frkn.org/nodes";
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
        inbounds: Vec::new(),
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

/// Проверяет доступность TCP-порта инбаунда из браузера.
/// Из обычной веб-страницы нельзя открыть raw TCP/UDP, поэтому
/// используем WebSocket handshake (`wss://hostname:port/`). Если порт
/// открыт и принимает TLS, handshake завершится событием open/error
/// до таймаута — это означает, что порт достижим. Если порт закрыт
/// или блокируется, браузер дождётся таймаута.
async fn check_inbound_port(hostname: &str, port: u16) -> (ServiceStatus, Option<u32>) {
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
        TimeoutFuture::new(4_000).await;
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
        if let Ok(speed) = measure_speed_url(&url, 15_000).await {
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
        if let Ok(speed) = measure_speed_url(&url, 15_000).await {
            return Ok(speed);
        }
    }

    Err("Не удалось измерить скорость ни на одном из зеркал".to_string())
}

async fn measure_speed_url(url: &str, timeout_ms: u32) -> Result<f64, String> {
    let start = now_ms();

    // Не добавляем кастомные заголовки, иначе браузер делает CORS-preflight,
    // который часто не проходит на публичных speedtest-зеркалах.
    // Cache-busting через ?cb=... достаточно.
    let send_fut = Request::get(url).send();

    let resp = match select(Box::pin(send_fut), TimeoutFuture::new(timeout_ms)).await {
        Either::Left((result, _)) => result.map_err(|e| e.to_string())?,
        Either::Right(_) => return Err("timeout".to_string()),
    };

    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let bytes = resp.binary().await.map_err(|e| e.to_string())?;
    let elapsed_sec = (now_ms() - start) / 1000.0;
    if elapsed_sec <= 0.0 {
        return Err("too fast".to_string());
    }

    let bits = (bytes.len() as f64) * 8.0;
    let mbps = bits / elapsed_sec / 1_000_000.0;
    Ok(mbps)
}

