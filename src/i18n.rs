use leptos::*;

const LANG_KEY: &str = "frkn-lang";

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Lang {
    Ru,
    En,
}

impl Lang {
    pub fn as_str(&self) -> &'static str {
        match self {
            Lang::Ru => "ru",
            Lang::En => "en",
        }
    }

    pub fn from_str(s: &str) -> Self {
        if s.to_lowercase().starts_with("ru") {
            Lang::Ru
        } else {
            Lang::En
        }
    }
}

/// Определяет язык: сначала смотрим localStorage, потом navigator.language.
pub fn detect_lang() -> Lang {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(Some(value)) = storage.get_item(LANG_KEY) {
                return Lang::from_str(&value);
            }
        }
        let nav_lang = window.navigator().language().unwrap_or_default();
        return Lang::from_str(&nav_lang);
    }
    Lang::En
}

/// Сохраняет выбранный язык в localStorage.
pub fn save_lang(lang: Lang) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.set_item(LANG_KEY, lang.as_str());
        }
    }
}

/// Перевод строки.
pub fn t(lang: Lang, key: &str) -> &str {
    match lang {
        Lang::Ru => ru(key),
        Lang::En => en(key),
    }
}

fn ru(key: &str) -> &str {
    match key {
        "header_title" => "Status Page",
        "header_subtitle" => "Проверьте VPN, статус серверов FRKN и скорость соединения",
        "how_we_measure" => "Как мы измеряем?",
        "vpn_detected" => "VPN обнаружен",
        "vpn_not_detected" => "VPN не обнаружен",
        "determining_ip" => "Определяем ваш IP...",
        "refresh" => "Обновить",
        "checking" => "Проверяем...",
        "frkn_badge" => "FRKN",
        "protected" => "Защищённо",
        "enable_vpn" => "Включите VPN",
        "speed_title" => "Скорость соединения",
        "speed_subtitle" => "Загружаем тестовый файл без кэша",
        "measure_speed" => "Проверить скорость",
        "measuring" => "Измеряем...",
        "speed_failed" => "Не удалось измерить скорость",
        "speed_prompt" => "Нажмите кнопку, чтобы начать замер",
        "mbps" => "Мбит/с",
        "pools_title" => "По пулам",
        "details" => "Подробно",
        "leaks_title" => "Проверка утечек",
        "leak_ipv6" => "IPv6 вне туннеля",
        "leak_webrtc" => "WebRTC (адреса из ICE)",
        "leak_ok" => "утечки нет",
        "leak_found" => "возможна утечка",
        "leak_possible" => "возможна утечка",
        "leak_no_vpn" => "есть, VPN не включён",
        "leak_dns" => "DNS: какие резолверы вас обслуживают",
        "leak_dns_run" => "Проверить DNS",
        "leak_dns_none" => "запросы не дошли — проверьте ещё раз",
        "leak_mtu" => "MTU: проходят ли крупные пакеты",
        "leak_mtu_run" => "Проверить MTU",
        "vpn_warn_frkn" => "Ты подключён к FRKN ({node}): замер идёт через туннель, поэтому цифры показывают путь до ноды, а не чиcтую скорость твоего канала. Для честного замера выключи VPN.",
        "vpn_warn_other" => "Включён VPN: замер идёт через туннель, цифры показывают твой текущий путь, а не скорость канала провайдера. Для честного замера выключи VPN.",
        "pool_median" => "медиана {mbps} · измерилось {n} из {m}",
        "measure_pool" => "Замерить пул",
        "measure_node" => "Замерить",
        "no_bench" => "нет bench",
        "no_answer" => "нет связи",
        "q_8k_multi" => "Отлично для 8K на нескольких устройствах",
        "q_8k" => "Отлично для 8K",
        "q_4k_multi" => "Отлично для 4K на нескольких устройствах",
        "q_great_4k" => "Отлично для 4K",
        "q_good_4k" => "Хорошо для 4K",
        "q_great_hd" => "Отлично для HD",
        "q_good_hd" => "Хорошо для HD",
        "q_basic" => "Только веб и мессенджеры",
        "q_gaming" => "гейминг",
        "pool_measured" => "измерилось {n} из {m}",
        "pool_unreachable" => " · не ответили {k}",
        "pool_none" => "Без пула",
        "frkn_servers_title" => "Серверы FRKN",
        "frkn_servers_note" => "Проверка идёт из вашего браузера: доступность зависит от провайдера и сети. Сервер не отвечает — не значит, что он лежит.",
        "of_available" => "{online} из {total} отвечают с вашего подключения",
        "show_servers" => "Показать {total} серверов",
        "hide" => "Скрыть",
        "api_label" => "FRKN",
        "api_hostname" => "api.frkn.org",
        "aggregate_all_online" => "Все серверы работают",
        "aggregate_partial" => "Часть серверов не отвечает",
        "aggregate_all_offline" => "Серверы не отвечают",
        "aggregate_error" => "Не удалось загрузить",
        "service_checking" => "Проверка...",
        "service_online" => "Доступен",
        "service_offline" => "Недоступен",
        "service_error" => "Ошибка",
        "browser_extension_title" => "Браузерное расширение",
        "browser_extension_text" => "Проверка доступности YouTube, X/Twitter, Instagram, Telegram и других сервисов, IP/VPN и скорость — всё в одном расширении.",
        "store_chrome" => "Chrome Web Store",
        "store_firefox" => "Firefox Add-ons",
        "zip_fallback" => "Если магазин недоступен в вашем регионе, можно установить расширение вручную из архива.",
        "download_zip_chrome" => "Скачать .zip для Chrome",
        "download_zip_firefox" => "Скачать .zip для Firefox",
        "footer_powered" => "powered by",
        "privacy_ru" => "Политика конфиденциальности",
        "privacy_en" => "Privacy Policy",
        "lang_ru" => "RU",
        "lang_en" => "EN",
        _ => key,
    }
}

fn en(key: &str) -> &str {
    match key {
        "header_title" => "Status Page",
        "header_subtitle" => "Check VPN, FRKN server status and connection speed",
        "how_we_measure" => "How do we measure?",
        "vpn_detected" => "VPN detected",
        "vpn_not_detected" => "VPN not detected",
        "determining_ip" => "Determining your IP...",
        "refresh" => "Refresh",
        "checking" => "Checking...",
        "frkn_badge" => "FRKN",
        "protected" => "Protected",
        "enable_vpn" => "Enable VPN",
        "speed_title" => "Connection speed",
        "speed_subtitle" => "Downloading test file without cache",
        "measure_speed" => "Measure speed",
        "measuring" => "Measuring...",
        "speed_failed" => "Failed to measure speed",
        "speed_prompt" => "Click the button to start measurement",
        "mbps" => "Mbps",
        "pools_title" => "By pool",
        "details" => "Details",
        "leaks_title" => "Leak checks",
        "leak_ipv6" => "IPv6 outside the tunnel",
        "leak_webrtc" => "WebRTC (ICE addresses)",
        "leak_ok" => "no leak",
        "leak_found" => "possible leak",
        "leak_possible" => "possible leak",
        "leak_no_vpn" => "present, VPN off",
        "leak_dns" => "DNS: which resolvers serve you",
        "leak_dns_run" => "Check DNS",
        "leak_dns_none" => "no queries arrived — try again",
        "leak_mtu" => "MTU: do large packets pass",
        "leak_mtu_run" => "Check MTU",
        "vpn_warn_frkn" => "You are on FRKN ({node}): the test runs through the tunnel, so the numbers describe your path to the node, not your raw channel. Turn the VPN off for an honest measurement.",
        "vpn_warn_other" => "VPN is on: the test runs through the tunnel, so the numbers describe your current path, not your provider's channel. Turn the VPN off for an honest measurement.",
        "pool_median" => "median {mbps} · measured {n} of {m}",
        "measure_pool" => "Measure pool",
        "measure_node" => "Measure",
        "no_bench" => "no bench",
        "no_answer" => "unreachable",
        "q_8k_multi" => "Great for 8K on several devices",
        "q_8k" => "Great for 8K video",
        "q_4k_multi" => "Great for 4K on several devices",
        "q_great_4k" => "Great for 4K video",
        "q_good_4k" => "Good for 4K video",
        "q_great_hd" => "Great for HD video",
        "q_good_hd" => "Good for HD video",
        "q_basic" => "Web and messengers only",
        "q_gaming" => "gaming",
        "pool_measured" => "measured {n} of {m}",
        "pool_unreachable" => " · {k} unreachable",
        "pool_none" => "No pool",
        "frkn_servers_title" => "FRKN servers",
        "frkn_servers_note" => "Checked from your browser: reachability depends on your provider and network. A server not answering does not mean it is down.",
        "of_available" => "{online} of {total} respond from your connection",
        "show_servers" => "Show {total} servers",
        "hide" => "Hide",
        "api_label" => "FRKN",
        "api_hostname" => "api.frkn.org",
        "aggregate_all_online" => "All servers running",
        "aggregate_partial" => "Some servers do not respond",
        "aggregate_all_offline" => "Servers do not respond",
        "aggregate_error" => "Failed to load",
        "service_checking" => "Checking...",
        "service_online" => "Available",
        "service_offline" => "Unavailable",
        "service_error" => "Error",
        "browser_extension_title" => "Browser extension",
        "browser_extension_text" => "Check availability of YouTube, X/Twitter, Instagram, Telegram and other services, IP/VPN and speed — all in one extension.",
        "store_chrome" => "Chrome Web Store",
        "store_firefox" => "Firefox Add-ons",
        "zip_fallback" => "If the store is unavailable in your region, you can install the extension manually from the archive.",
        "download_zip_chrome" => "Download .zip for Chrome",
        "download_zip_firefox" => "Download .zip for Firefox",
        "footer_powered" => "powered by",
        "privacy_ru" => "Политика конфиденциальности",
        "privacy_en" => "Privacy Policy",
        "lang_ru" => "RU",
        "lang_en" => "EN",
        _ => key,
    }
}
