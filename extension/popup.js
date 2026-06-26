document.addEventListener('DOMContentLoaded', () => {
  const runtime = typeof browser !== 'undefined' ? browser.runtime : chrome.runtime;

  const tabs = document.querySelectorAll('.tab');
  const contents = document.querySelectorAll('.tab-content');

  tabs.forEach((tab) => {
    tab.addEventListener('click', () => {
      tabs.forEach((t) => t.classList.remove('active'));
      contents.forEach((c) => c.classList.remove('active'));
      tab.classList.add('active');
      document.getElementById(`tab-${tab.dataset.tab}`).classList.add('active');
    });
  });

  // Переключатель языка
  function updateLangButtons() {
    document.querySelectorAll('.lang-btn').forEach((btn) => {
      btn.classList.toggle('active', btn.dataset.lang === currentLang);
    });
  }

  document.querySelectorAll('.lang-btn').forEach((btn) => {
    btn.addEventListener('click', () => {
      setLang(btn.dataset.lang);
      updateLangButtons();
    });
  });
  updateLangButtons();

  document.addEventListener('frkn:langchange', () => {
    runServicesCheck();
    runNetworkCheck();
    runFrknNodesCheck();
  });

  function setStatus(id, text) {
    const el = document.getElementById(id);
    if (el) el.textContent = text;
  }

  function sendMessage(action) {
    return new Promise((resolve) => {
      runtime.sendMessage({ action }, (response) => {
        if (runtime.lastError) {
          resolve(null);
        } else {
          resolve(response);
        }
      });
    });
  }

  function statusLabel(status) {
    if (status === 'online') return t('online');
    if (status === 'offline') return t('offline');
    return t('error');
  }

  function renderServices(results) {
    const servicesEl = document.getElementById('services');
    servicesEl.innerHTML = '';

    const onlineCount = results.filter((r) => r.status === 'online').length;
    const total = results.length;
    setStatus('services-status', `${onlineCount}/${total} ${t('available')}`);

    results.forEach((r) => {
      const div = document.createElement('div');
      div.className = 'service';

      const statusClass = r.status === 'online' ? 'online' : 'offline';
      const label = statusLabel(r.status);

      const nameSpan = document.createElement('span');
      nameSpan.className = 'service-name';
      nameSpan.textContent = r.name;

      const right = document.createElement('div');

      const statusSpan = document.createElement('span');
      statusSpan.className = `service-status ${statusClass}`;
      statusSpan.textContent = label;
      right.appendChild(statusSpan);

      if (r.latency) {
        const latencySpan = document.createElement('span');
        latencySpan.className = 'latency';
        latencySpan.textContent = `(${r.latency} ms)`;
        right.appendChild(latencySpan);
      }

      if (r.method) {
        const methodSpan = document.createElement('span');
        methodSpan.className = 'latency';
        methodSpan.textContent = r.method;
        right.appendChild(methodSpan);
      }

      if (r.reason && r.status === 'offline') {
        const reasonDiv = document.createElement('div');
        reasonDiv.className = 'reason';
        reasonDiv.textContent = r.reason === 'no response' ? t('reason_no_response') : r.reason;
        right.appendChild(reasonDiv);
      }

      div.appendChild(nameSpan);
      div.appendChild(right);
      servicesEl.appendChild(div);
    });
  }

  function renderNetwork(data) {
    const infoEl = document.getElementById('network-info');
    infoEl.innerHTML = '';

    if (!data.info) {
      setStatus('network-status', t('status_failed_ip'));
      return;
    }

    setStatus('network-status', t('status_updating'));
    const rows = [
      { label: t('ip'), value: data.info.ip },
      { label: t('country'), value: data.info.country_name || data.info.country_code },
      { label: t('city'), value: data.info.city },
      { label: t('region'), value: data.info.region },
      { label: t('provider'), value: data.info.org },
      { label: t('asn'), value: data.info.asn },
      {
        label: t('vpn'),
        value: data.vpn ? t('vpn_detected') : t('vpn_not_detected'),
        className: data.vpn ? 'vpn-on' : 'vpn-off',
      },
      {
        label: t('api_status'),
        value: data.frkn.status === 'online' ? `${t('api_available')} (${data.frkn.latency} ms)` : t('api_unavailable'),
        className: data.frkn.status === 'online' ? 'vpn-on' : 'vpn-off',
      },
    ];

    rows.forEach((row) => {
      if (!row.value) return;
      const div = document.createElement('div');
      div.className = 'network-row';

      const labelSpan = document.createElement('span');
      labelSpan.className = 'network-label';
      labelSpan.textContent = row.label;

      const valueSpan = document.createElement('span');
      valueSpan.className = `network-value ${row.className || ''}`;
      valueSpan.textContent = row.value;

      div.appendChild(labelSpan);
      div.appendChild(valueSpan);
      infoEl.appendChild(div);
    });
  }

  function renderSpeed(data) {
    if (!data.result) {
      setStatus('speed-status', t('status_measurement_error'));
      return;
    }
    setStatus('speed-status', `${t('file')}: ${(data.result.size / 1024 / 1024).toFixed(1)} MB`);
    const div = document.createElement('div');
    div.className = 'speed-result';

    const valueSpan = document.createElement('span');
    valueSpan.className = 'speed-value';
    valueSpan.textContent = data.result.mbps.toFixed(1);

    const unitSpan = document.createElement('span');
    unitSpan.className = 'speed-unit';
    unitSpan.textContent = t('mbps');

    div.appendChild(valueSpan);
    div.appendChild(unitSpan);

    const container = document.getElementById('tab-speed');
    const existing = container.querySelector('.speed-result');
    if (existing) existing.remove();
    container.insertBefore(div, document.getElementById('measure-speed'));
  }

  function renderNodes(data) {
    const nodesEl = document.getElementById('nodes');
    nodesEl.innerHTML = '';

    if (!data || !data.nodes) {
      setStatus('nodes-status', t('status_nodes_error'));
      return;
    }

    const aggregateMap = {
      online: t('all_available'),
      partial: t('partially_available'),
      offline: t('unavailable'),
      error: t('failed_to_load'),
    };
    setStatus('nodes-status', aggregateMap[data.aggregate] || t('unknown'));

    data.nodes.forEach((n) => {
      const div = document.createElement('div');
      div.className = 'node';

      const statusClass = n.status === 'online' ? 'online' : 'offline';
      const statusText = statusLabel(n.status);

      const header = document.createElement('div');
      header.className = 'node-header';

      const left = document.createElement('div');
      left.style.minWidth = '0';

      const nameDiv = document.createElement('div');
      nameDiv.className = 'node-name';
      nameDiv.textContent = n.label;

      const metaDiv = document.createElement('div');
      metaDiv.className = 'node-meta';
      metaDiv.textContent = `${n.country} ${n.address} • API: ${n.apiStatus || t('no_inbounds')}`;

      left.appendChild(nameDiv);
      left.appendChild(metaDiv);

      const right = document.createElement('div');
      right.style.flexShrink = '0';

      const nodeStatusSpan = document.createElement('span');
      nodeStatusSpan.className = `node-status ${statusClass}`;
      nodeStatusSpan.textContent = statusText;
      right.appendChild(nodeStatusSpan);

      if (n.latency) {
        const latencySpan = document.createElement('span');
        latencySpan.className = 'latency';
        latencySpan.textContent = `(${n.latency} ms)`;
        right.appendChild(latencySpan);
      }

      header.appendChild(left);
      header.appendChild(right);
      div.appendChild(header);

      if (n.inbounds && n.inbounds.length > 0) {
        const inboundList = document.createElement('div');
        inboundList.className = 'inbound-list';

        n.inbounds.forEach((inb) => {
          const dotClass = inb.status === 'online' ? 'online' : 'offline';

          const inboundSpan = document.createElement('span');
          inboundSpan.className = 'inbound';

          const tagSpan = document.createElement('span');
          tagSpan.textContent = `${inb.tag}:${inb.port}`;

          const dot = document.createElement('span');
          dot.className = `inbound-dot ${dotClass}`;

          inboundSpan.appendChild(tagSpan);
          inboundSpan.appendChild(dot);
          inboundList.appendChild(inboundSpan);
        });

        div.appendChild(inboundList);
      }

      nodesEl.appendChild(div);
    });
  }

  async function runServicesCheck() {
    setStatus('services-status', t('status_checking'));
    document.getElementById('services').innerHTML = '';
    const response = await sendMessage('checkServices');
    if (response && Array.isArray(response)) {
      renderServices(response);
    } else {
      setStatus('services-status', t('status_data_error'));
    }
  }

  async function runNetworkCheck() {
    setStatus('network-status', t('status_checking'));
    document.getElementById('network-info').innerHTML = '';
    const response = await sendMessage('checkNetwork');
    if (response) {
      renderNetwork(response);
    } else {
      setStatus('network-status', t('status_data_error'));
    }
  }

  async function runSpeedCheck() {
    setStatus('speed-status', t('status_measuring'));
    document.getElementById('measure-speed').disabled = true;
    const response = await sendMessage('measureSpeed');
    document.getElementById('measure-speed').disabled = false;
    if (response) {
      renderSpeed(response);
    } else {
      setStatus('speed-status', t('status_measurement_error'));
    }
  }

  document.getElementById('refresh').addEventListener('click', async () => {
    document.getElementById('refresh').disabled = true;
    await Promise.all([runServicesCheck(), runFrknNodesCheck(), runNetworkCheck()]);
    document.getElementById('refresh').disabled = false;
  });

  document.getElementById('measure-speed').addEventListener('click', runSpeedCheck);

  // Автозапуск при открытии popup
  runServicesCheck();
  runFrknNodesCheck();
  runNetworkCheck();
});
