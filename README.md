# Status Page

Одностраничная страница статуса на Rust + WebAssembly (Leptos).

Показывает:

- ваш внешний IP, страну и провайдера;
- факт использования VPN и бейджик FRKN, если ваш внешний IP совпадает с одной из FRKN-нод;
- доступность API FRKN (`api.frkn.org`) и сводный статус серверов FRKN;
- скорость интернет-соединения.

Проверка доступности сторонних сервисов (YouTube, X/Twitter, Instagram и др.) вынесена в отдельное браузерное расширение, потому что из чистого фронтенда нельзя корректно отличить настоящий сайт от провайдерской заглушки.

Powered by [FRKN](https://frkn.org/#trial).

## Стек

- Rust
- Leptos (CSR — клиентский рендеринг в WASM)
- `trunk` для сборки
- Tailwind CSS (CDN)

## Быстрый старт

```bash
# 1. Установить WASM target (один раз)
rustup target add wasm32-unknown-unknown

# 2. Установить trunk (один раз)
cargo install trunk

# 3. Запустить dev-сервер
trunk serve --port 8080

# 4. Открыть http://127.0.0.1:8080
```

## Сборка для продакшена

```bash
trunk build --release
```

Статические файлы появятся в `dist/`. Их можно загрузить на GitHub Pages, Netlify, Cloudflare Pages или любой другой статический хостинг.

### Деплой

```bash
./deploy.sh
```

Скрипт собирает production-версию и синхронизирует `dist/` на сервер (`darkmachine2.frkn.org:/opt/mirror/status.frkn.org`).

## Как это работает

- VPN определяется через внешний IP-геолокационный сервис (`ipapi.co`, `ipwho.is`, fallback на `ipinfo.io`): если провайдер похож на хостинг/дата-центр/VPN или внешний IP совпадает с IP-адресом одной из FRKN-нод, считаем что VPN включён.
- Список FRKN-нод загружается с `https://api.frkn.org/nodes`; доступность API проверяется через тот же endpoint `/nodes`.
- Speed test загружает тестовый файл с `https://status.frkn.org/speedtest/` (cache-busting) и считает Mbps. Если собственный файл недоступен, используются публичные зеркала как fallback.

## Браузерное расширение

В папке `extension/` находится расширение для Chrome и Firefox, которое проверяет доступность популярных сервисов уже из браузера пользователя с чтением HTTP-ответов:

- **Firefox Add-ons**: https://addons.mozilla.org/addon/frkn-service-checker/
- **Chrome Web Store**: пока на модерации

```
extension/
├── manifest.json            # Chrome (Manifest V3)
├── manifest-firefox.json    # Firefox (Manifest V3)
├── background.js            # логика проверок
├── popup.html / popup.js / popup.css
└── icons/
```

### Установка в Chrome

1. Открыть `chrome://extensions/`.
2. Включить **Режим разработчика**.
3. Нажать **Загрузить распакованное расширение**.
4. Выбрать папку `extension/`.

### Установка в Firefox

1. Переименовать `manifest-firefox.json` в `manifest.json`.
2. Открыть `about:debugging` → **Этот Firefox**.
3. Нажать **Загрузить временное дополнение**.
4. Выбрать `manifest.json`.

## Структура

```
src/
├── main.rs      # точка входа
├── app.rs       # UI на Leptos
├── network.rs   # IP, VPN, FRKN API, speed test
└── services.rs  # ServiceStatus (общий enum)
extension/       # браузерное расширение для проверки сервисов
deploy.sh        # скрипт деплоя
```

## Ограничения

- Браузерные CORS-ограничения не позволяют прочитать ответы чужих сайтов, поэтому проверка сторонних сервисов вынесена в расширение.
- Проверка FRKN API работает только если `api.frkn.org` отдаёт `Access-Control-Allow-Origin: *` (актуально, если страница находится на другом домене).
- ICMP ping недоступен из браузера; вместо него используется HTTP-латентность.
