const SERVICES = [
  { id: 'gosuslugi', name: 'Госуслуги', url: 'https://www.gosuslugi.ru' },
  { id: 'youtube', name: 'YouTube', url: 'https://www.youtube.com' },
  { id: 'twitter', name: 'X / Twitter', url: 'https://twitter.com' },
  { id: 'instagram', name: 'Instagram', url: 'https://www.instagram.com' },
  { id: 'telegram', name: 'Telegram', url: 'https://web.telegram.org' },
  { id: 'roblox', name: 'Roblox', url: 'https://www.roblox.com' },
  { id: 'chatgpt', name: 'ChatGPT', url: 'https://chatgpt.com' },
  { id: 'claude', name: 'Claude', url: 'https://claude.ai' },
  { id: 'yandex', name: 'Яндекс', url: 'https://ya.ru' },
  { id: 'yandex-disk', name: 'Яндекс Диск', url: 'https://disk.yandex.ru' },
  { id: 'cloudflare', name: 'Cloudflare', url: 'https://1.1.1.1' },
  { id: 'google-dns', name: 'Google DNS', url: 'https://8.8.8.8' },
];

const BLOCKED_PATTERNS = [
  'Unable to load site',
  'Доступ ограничен',
  'доступ ограничен',
  'Access denied',
  'access denied',
  '403 Forbidden',
  'blocked',
  'Заблокировано',
  'Не удается получить доступ',
  "This site can't be reached",
  'ERR_CONNECTION_RESET',
  'ERR_TIMED_OUT',
  'Сайт недоступен',
];

const VPN_KEYWORDS = [
  'vpn', 'proxy', 'hosting', 'datacenter', 'cloud', 'server', 'vps',
  'dedicated', 'teleport', 'outline', 'wireguard', 'openvpn', 'm247',
  'ovh', 'hetzner', 'digitalocean', 'linode', 'aws', 'amazon',
  // Популярные VPN/прокси и антивирусы с VPN
  'mullvad', 'nord', 'nordvpn', 'expressvpn', 'surfshark', 'proton',
  'protonvpn', 'windscribe', 'cyberghost', 'private internet access', 'pia',
  'hotspot shield', 'tunnelbear', 'hide.me', 'hideme', 'torguard', 'ivpn',
  'airvpn', 'zoogvpn', 'wevpn', 'purevpn', 'strongvpn', 'vyprvpn', 'ipvanish',
  'kaspersky', 'avast', 'avg', 'bitdefender', 'mcafee', 'norton', 'whoer',
  'frkn',
];

const DEFAULT_HEADERS = {
  'User-Agent': 'FRKN-Service-Checker/1.0',
  'Accept': '*/*',
  'Accept-Language': 'ru-RU,ru;q=0.9,en-US;q=0.8,en;q=0.7',
};

async function fetchWithTimeout(url, options = {}, timeoutMs = 15000) {
  const controller = new AbortController();
  const id = setTimeout(() => controller.abort(), timeoutMs);
  try {
    const resp = await fetch(url, { ...options, signal: controller.signal });
    clearTimeout(id);
    return resp;
  } catch (e) {
    clearTimeout(id);
    throw e;
  }
}

function isBlocked(text) {
  return BLOCKED_PATTERNS.some((p) => text.includes(p));
}

async function checkService(service) {
  const start = performance.now();
  const cb = Date.now();
  const mainUrl = `${service.url}/?cb=${cb}`;
  const robotsUrl = `${service.url}/robots.txt?cb=${cb}`;

  // 1. HEAD главного URL — быстро, без скачивания тела.
  try {
    const resp = await fetchWithTimeout(
      mainUrl,
      { method: 'HEAD', headers: DEFAULT_HEADERS, redirect: 'follow' },
      10000
    );
    const latency = Math.round(performance.now() - start);
    if (resp.ok) {
      return { ...service, status: 'online', latency, method: 'HEAD' };
    }
  } catch (e) {
    // fallback
  }

  // 2. GET robots.txt — лёгкий файл, часто не блокируется.
  try {
    const resp = await fetchWithTimeout(
      robotsUrl,
      { method: 'GET', headers: DEFAULT_HEADERS, redirect: 'follow' },
      10000
    );
    const latency = Math.round(performance.now() - start);
    const text = await resp.text();
    if (resp.ok && !isBlocked(text)) {
      return { ...service, status: 'online', latency, method: 'robots' };
    }
  } catch (e) {
    // offline
  }

  return { ...service, status: 'offline', reason: 'no response' };
}

function checkAllServices() {
  return Promise.all(SERVICES.map(checkService));
}

async function fetchIpInfo() {
  const endpoints = [
    {
      url: 'https://ipapi.co/json/',
      map: (j) => ({
        ip: j.ip,
        country_code: j.country_code || '',
        country_name: j.country_name || '',
        city: j.city || '',
        region: j.region || '',
        org: j.org || '',
        asn: j.asn || '',
        // ipapi.co может отдавать threat-поля только на платных тарифах,
        // но если они есть — используем их.
        vpn: !!(j.threat?.is_anonymous || j.threat?.is_proxy || j.threat?.is_vpn),
      }),
    },
    {
      url: 'https://ipwho.is/',
      map: (j) => ({
        ip: j.ip,
        country_code: j.country_code || '',
        country_name: j.country || '',
        city: j.city || '',
        region: j.region || '',
        org: j.connection?.org || j.org || '',
        asn: String(j.connection?.asn || j.asn || ''),
        vpn: !!(j.security?.vpn || j.security?.proxy || j.security?.datacenter || j.security?.tor),
      }),
    },
    {
      url: 'https://ipinfo.io/json',
      map: (j) => ({
        ip: j.ip,
        country_code: j.country || '',
        country_name: j.country || '',
        city: j.city || '',
        region: j.region || '',
        org: j.org || '',
        asn: j.asn || j.org || '',
        vpn: !!(j.privacy?.vpn || j.privacy?.proxy || j.privacy?.hosting || j.privacy?.tor),
      }),
    },
  ];

  // Запрашиваем все сервисы параллельно, чтобы не упустить VPN-флаги
  // из одного сервиса, если другой ответил быстрее без них.
  const settled = await Promise.allSettled(
    endpoints.map(async (ep) => {
      const resp = await fetchWithTimeout(ep.url, { headers: DEFAULT_HEADERS }, 8000);
      if (!resp.ok) throw new Error('not ok');
      const json = await resp.json();
      return ep.map(json);
    })
  );

  const infos = settled
    .filter((r) => r.status === 'fulfilled')
    .map((r) => r.value);

  if (infos.length === 0) {
    return null;
  }

  // Для отображения используем первый успешный (приоритет ipapi → ipwho → ipinfo).
  const display = infos[0];

  // VPN считаем обнаруженным, если хотя бы один сервис выставил флаг
  // или хотя бы у одного провайдер/ASN похож на VPN/дата-центр.
  const anyVpn = infos.some((info) => info.vpn);
  const anyKeyword = infos.some((info) => {
    const text = `${info.org} ${info.asn}`.toLowerCase();
    return VPN_KEYWORDS.some((k) => text.includes(k));
  });

  return { ...display, vpn: display.vpn || anyVpn || anyKeyword };
}

function detectVpn(info, nodes) {
  // 1. Прямые флаги от IP-сервисов (vpn/proxy/hosting/datacenter/tor).
  if (info.vpn) return true;

  // 2. Эвристика по названию провайдера/ASN.
  const org = (info.org || '').toLowerCase();
  const asn = (info.asn || '').toLowerCase();
  if (VPN_KEYWORDS.some((k) => org.includes(k) || asn.includes(k))) {
    return true;
  }

  // 3. Если внешний IP совпадает с адресом одной из нод FRKN — значит, это FRKN VPN.
  if (nodes && nodes.length > 0 && info.ip) {
    return nodes.some((n) => n.address === info.ip);
  }

  return false;
}

async function checkFrknPing() {
  const url = `https://api.frkn.org/nodes?cb=${Date.now()}`;
  const start = performance.now();
  try {
    const resp = await fetchWithTimeout(url, { headers: DEFAULT_HEADERS }, 8000);
    const latency = Math.round(performance.now() - start);
    return { status: resp.ok ? 'online' : 'offline', latency };
  } catch (e) {
    return {
      status: 'offline',
      reason: e.name === 'AbortError' ? 'timeout' : e.message,
    };
  }
}

async function fetchFrknNodes() {
  try {
    const resp = await fetchWithTimeout(
      `https://api.frkn.org/nodes?cb=${Date.now()}`,
      { headers: DEFAULT_HEADERS },
      10000
    );
    if (!resp.ok) return null;
    const json = await resp.json();
    return json.response || null;
  } catch (e) {
    return null;
  }
}

function checkInboundPort(hostname, port, timeoutMs = 4000) {
  return new Promise((resolve) => {
    const url = `wss://${hostname}:${port}/`;
    const start = performance.now();
    let settled = false;
    let ws;
    try {
      ws = new WebSocket(url);
    } catch (e) {
      return resolve({ status: 'error', reason: e.message });
    }

    function finish(reachable, reason) {
      if (settled) return;
      settled = true;
      try {
        if (ws && ws.readyState === WebSocket.OPEN) ws.close();
      } catch (_) {}
      const latency = Math.round(performance.now() - start);
      if (reachable) {
        resolve({ status: 'online', latency });
      } else {
        resolve({ status: 'offline', reason });
      }
    }

    ws.addEventListener('open', () => finish(true));
    ws.addEventListener('error', () => finish(true));

    setTimeout(() => finish(false, 'timeout'), timeoutMs);
  });
}

async function checkFrknNodes() {
  const nodes = await fetchFrknNodes();
  if (!nodes) return { aggregate: 'error', nodes: [] };

  const checkable = nodes.filter((n) => n.type !== 'PremiumNode');
  const results = await Promise.all(
    checkable.map(async (node) => {
      const url = `https://${node.hostname}/favicon.ico?cb=${Date.now()}`;
      const start = performance.now();
      let nodeStatus = 'offline';
      let nodeLatency = null;
      try {
        const resp = await fetchWithTimeout(url, { method: 'HEAD' }, 4000);
        if (resp.ok) {
          nodeStatus = 'online';
          nodeLatency = Math.round(performance.now() - start);
        }
      } catch (e) {}

      const inbounds = await Promise.all(
        (node.inbounds || []).map(async (inb) => {
          const result = await checkInboundPort(node.hostname, inb.port);
          return {
            tag: inb.tag,
            port: inb.port,
            status: result.status,
            latency: result.latency || null,
          };
        })
      );

      // Общий статус ноды = online, если хотя бы один инбаунд доступен
      // или отвечает HTTP/фавикон. Reality-ноды часто не отдают favicon,
      // поэтому инбаунды — более надёжный критерий.
      const inboundOnline = inbounds.some((i) => i.status === 'online');
      if (inboundOnline) {
        nodeStatus = 'online';
        const inLatencies = inbounds
          .filter((i) => i.latency)
          .map((i) => i.latency);
        if (inLatencies.length > 0) {
          nodeLatency = Math.min(...inLatencies);
        }
      }

      return {
        label: node.label,
        hostname: node.hostname,
        address: node.address,
        country: node.country,
        apiStatus: node.status,
        status: nodeStatus,
        latency: nodeLatency,
        inbounds,
      };
    })
  );

  const api = await checkFrknPing();
  results.unshift({
    label: 'API',
    hostname: 'api.frkn.org',
    address: 'api.frkn.org',
    country: '',
    apiStatus: 'API',
    status: api.status,
    latency: api.latency || null,
    inbounds: [],
  });

  const total = results.length;
  const online = results.filter((r) => r.status === 'online').length;
  let aggregate = 'partial';
  if (total === 0) aggregate = 'offline';
  else if (online === total) aggregate = 'online';
  else if (online === 0) aggregate = 'offline';

  return { aggregate, nodes: results };
}

async function measureSpeed() {
  const urls = [
    `https://status.frkn.org/speedtest/10mb.test?cb=${Date.now()}`,
    `https://proof.ovh.net/files/10Mb.dat?cb=${Date.now()}`,
    `https://cachefly.cachefly.net/10mb.test?cb=${Date.now()}`,
  ];

  for (const url of urls) {
    try {
      const start = performance.now();
      const resp = await fetchWithTimeout(url, {}, 20000);
      const blob = await resp.blob();
      const elapsedSec = (performance.now() - start) / 1000.0;
      if (elapsedSec <= 0) continue;
      const bits = blob.size * 8;
      const mbps = bits / elapsedSec / 1_000_000;
      return { mbps, size: blob.size, elapsedSec };
    } catch (e) {}
  }

  // Fallback 1 MB
  const fallbackUrls = [
    `https://status.frkn.org/speedtest/1mb.test?cb=${Date.now()}`,
    `https://proof.ovh.net/files/1Mb.dat?cb=${Date.now()}`,
    `https://cachefly.cachefly.net/1mb.test?cb=${Date.now()}`,
  ];
  for (const url of fallbackUrls) {
    try {
      const start = performance.now();
      const resp = await fetchWithTimeout(url, {}, 15000);
      const blob = await resp.blob();
      const elapsedSec = (performance.now() - start) / 1000.0;
      if (elapsedSec <= 0) continue;
      const bits = blob.size * 8;
      const mbps = bits / elapsedSec / 1_000_000;
      return { mbps, size: blob.size, elapsedSec };
    } catch (e) {}
  }

  return null;
}

function setupRuntime(runtime) {
  runtime.onMessage.addListener((request, sender, sendResponse) => {
    if (request.action === 'checkServices') {
      checkAllServices().then(sendResponse);
      return true;
    }

    if (request.action === 'checkNetwork') {
      Promise.all([fetchIpInfo(), checkFrknPing(), fetchFrknNodes()])
        .then(([info, frkn, nodes]) => {
          sendResponse({
            info,
            vpn: info ? detectVpn(info, nodes) : null,
            frkn,
          });
        })
        .catch(() => sendResponse({ info: null, vpn: null, frkn: { status: 'offline' } }));
      return true;
    }

    if (request.action === 'measureSpeed') {
      measureSpeed().then((result) => sendResponse({ result }));
      return true;
    }

    if (request.action === 'checkFrknNodes') {
      checkFrknNodes().then(sendResponse);
      return true;
    }
  });
}

if (typeof chrome !== 'undefined' && chrome.runtime && chrome.runtime.onMessage) {
  setupRuntime(chrome.runtime);
}

if (typeof browser !== 'undefined' && browser.runtime && browser.runtime.onMessage) {
  setupRuntime(browser.runtime);
}
