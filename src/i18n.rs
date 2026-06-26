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
        "frkn_servers_title" => "Серверы FRKN",
        "frkn_servers_note" => "Проверка портов через WebSocket handshake (TCP/TLS). UDP-протоколы из браузера недоступны.",
        "of_available" => "{online} из {total} доступны",
        "show_servers" => "Показать {total} серверов",
        "hide" => "Скрыть",
        "api_label" => "FRKN",
        "api_hostname" => "api.frkn.org",
        "aggregate_all_online" => "Все доступны",
        "aggregate_partial" => "Частично доступны",
        "aggregate_all_offline" => "Недоступны",
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
        "frkn_servers_title" => "FRKN servers",
        "frkn_servers_note" => "Port checks via WebSocket handshake (TCP/TLS). UDP protocols are not available from the browser.",
        "of_available" => "{online} of {total} available",
        "show_servers" => "Show {total} servers",
        "hide" => "Hide",
        "api_label" => "FRKN",
        "api_hostname" => "api.frkn.org",
        "aggregate_all_online" => "All available",
        "aggregate_partial" => "Partially available",
        "aggregate_all_offline" => "Unavailable",
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
