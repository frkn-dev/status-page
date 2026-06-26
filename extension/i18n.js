const LANG_KEY = 'frkn-lang';

const TRANSLATIONS = {
  ru: {
    tab_services: 'Сервисы',
    tab_nodes: 'Ноды',
    tab_network: 'IP/VPN',
    tab_speed: 'Скорость',
    status_press_refresh: 'Нажмите «Обновить»',
    status_press_measure: 'Нажмите «Измерить»',
    status_updating: 'Данные обновлены',
    status_failed_ip: 'Не удалось определить IP',
    status_data_error: 'Ошибка получения данных',
    status_nodes_error: 'Ошибка получения данных',
    status_checking: 'Проверяем...',
    status_measuring: 'Измеряем...',
    status_measurement_error: 'Ошибка измерения',
    refresh: 'Обновить',
    measure_speed: 'Измерить скорость',
    available: 'доступны',
    online: 'Доступен',
    offline: 'Недоступен',
    error: 'Ошибка',
    ip: 'IP',
    country: 'Страна',
    city: 'Город',
    region: 'Регион',
    provider: 'Провайдер',
    asn: 'ASN',
    vpn: 'VPN',
    vpn_detected: 'Обнаружен',
    vpn_not_detected: 'Не обнаружен',
    api_status: 'api.frkn.org',
    api_available: 'Доступен',
    api_unavailable: 'Недоступен',
    file: 'Файл',
    mbps: 'Мбит/с',
    all_available: 'Все доступны',
    partially_available: 'Частично доступны',
    unavailable: 'Недоступны',
    failed_to_load: 'Не удалось загрузить',
    unknown: 'Неизвестно',
    reason_no_response: 'no response',
    reason_timeout: 'timeout',
    no_inbounds: '—',
    lang_ru: 'RU',
    lang_en: 'EN',
    footer_powered: 'powered by',
  },
  en: {
    tab_services: 'Services',
    tab_nodes: 'Nodes',
    tab_network: 'IP/VPN',
    tab_speed: 'Speed',
    status_press_refresh: 'Press «Refresh»',
    status_press_measure: 'Press «Measure»',
    status_updating: 'Data updated',
    status_failed_ip: 'Failed to determine IP',
    status_data_error: 'Failed to get data',
    status_nodes_error: 'Failed to get data',
    status_checking: 'Checking...',
    status_measuring: 'Measuring...',
    status_measurement_error: 'Measurement error',
    refresh: 'Refresh',
    measure_speed: 'Measure speed',
    available: 'available',
    online: 'Available',
    offline: 'Unavailable',
    error: 'Error',
    ip: 'IP',
    country: 'Country',
    city: 'City',
    region: 'Region',
    provider: 'Provider',
    asn: 'ASN',
    vpn: 'VPN',
    vpn_detected: 'Detected',
    vpn_not_detected: 'Not detected',
    api_status: 'api.frkn.org',
    api_available: 'Available',
    api_unavailable: 'Unavailable',
    file: 'File',
    mbps: 'Mbps',
    all_available: 'All available',
    partially_available: 'Partially available',
    unavailable: 'Unavailable',
    failed_to_load: 'Failed to load',
    unknown: 'Unknown',
    reason_no_response: 'no response',
    reason_timeout: 'timeout',
    no_inbounds: '—',
    lang_ru: 'RU',
    lang_en: 'EN',
    footer_powered: 'powered by',
  },
};

function detectLang() {
  const saved = localStorage.getItem(LANG_KEY);
  if (saved && TRANSLATIONS[saved]) {
    return saved;
  }
  const nav = (navigator.language || 'en').toLowerCase();
  return nav.startsWith('ru') ? 'ru' : 'en';
}

let currentLang = detectLang();

function setLang(lang) {
  if (!TRANSLATIONS[lang]) return;
  currentLang = lang;
  localStorage.setItem(LANG_KEY, lang);
  applyStaticTranslations();
  // Переводим динамические блоки, если они уже отрендерены
  document.dispatchEvent(new Event('frkn:langchange'));
}

function t(key) {
  return TRANSLATIONS[currentLang][key] || TRANSLATIONS.en[key] || key;
}

function applyStaticTranslations() {
  document.querySelectorAll('[data-i18n]').forEach((el) => {
    const key = el.getAttribute('data-i18n');
    if (key) el.textContent = t(key);
  });
  document.querySelectorAll('[data-i18n-aria]').forEach((el) => {
    const key = el.getAttribute('data-i18n-aria');
    if (key) el.setAttribute('aria-label', t(key));
  });
}

if (typeof module !== 'undefined' && module.exports) {
  module.exports = { t, setLang, applyStaticTranslations, detectLang };
}
