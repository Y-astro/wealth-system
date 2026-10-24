/* Shared production build for the Redmi and optional loopback website. */
(() => {
  const relayUrl = input => {
    const url = new URL(String(input), location.href);
    if (url.origin === location.origin && /^\/api\/v1(?:\/|$)/.test(url.pathname)) url.pathname = '/wealth' + url.pathname;
    return url;
  };
  const NativeEventSource = window.EventSource;
  if (NativeEventSource) {
    window.EventSource = class extends NativeEventSource {
      constructor(url, options) { super(relayUrl(url), options); }
    };
  }
  document.addEventListener('DOMContentLoaded', () => {
    let locking = false;
    const lock = async () => {
      if (locking) return;
      locking = true;
      document.documentElement.style.visibility = 'hidden';
      try { await window.fetch('/wealth/logout', {method:'POST', credentials:'same-origin', redirect:'manual'}); }
      finally { location.replace('/wealth/'); }
    };
    let lastActivity = Date.now();
    const checkIdle = () => { if (Date.now() - lastActivity >= 15 * 60 * 1000) void lock(); };
    for (const event of ['pointerdown','keydown','scroll']) {
      document.addEventListener(event, () => { checkIdle(); if (!locking) lastActivity = Date.now(); }, {passive:true});
    }
    document.addEventListener('visibilitychange', checkIdle);
    setInterval(checkIdle, 30000);
    window.addEventListener('pageshow', event => { if (event.persisted) void lock(); });
    window.addEventListener('pagehide', () => {
      document.documentElement.style.visibility = 'hidden';
      navigator.sendBeacon('/wealth/logout', '');
    });
    const button = document.createElement('button');
    button.type = 'button';
    button.textContent = 'Lock Wealthfolio';
    button.setAttribute('aria-label', 'Lock Wealthfolio and require the password again');
    button.style.cssText = 'position:fixed;bottom:14px;right:18px;z-index:50;padding:8px 12px;border:1px solid #777;border-radius:8px;background:#20221e;color:#eee;font:12px system-ui;cursor:pointer';
    button.addEventListener('click', lock);
    document.body.append(button);
  });
  const original = window.fetch.bind(window);
  window.fetch = async (input, init) => {
    const isRequest = input instanceof Request;
    const url = new URL(isRequest ? input.url : String(input), location.href);
    if (url.origin === location.origin && /^\/api\/v1(?:\/|$)/.test(url.pathname)) {
      url.pathname = '/wealth' + url.pathname;
      input = isRequest ? new Request(url, input) : url;
    }
    const response = await original(input, init);
    if (response.status === 401 && url.origin === location.origin && url.pathname.startsWith('/wealth/api/')) {
      location.replace('/wealth/');
    }
    return response;
  };
})();
