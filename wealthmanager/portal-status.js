/* Connection and processing state only; no credentials or reports are stored. */
(() => {
  const init = () => {
    let box = document.getElementById('wealth-connection');
    if (!box) {
      box = document.createElement('aside');
      box.id = 'wealth-connection';
      box.setAttribute('role', 'status');
      box.style.cssText = 'position:fixed;left:12px;bottom:12px;z-index:49;max-width:min(480px,calc(100vw - 190px));padding:9px 12px;border:1px solid #626558;border-radius:9px;background:#1b1c18;color:#eeede6;font:12px system-ui;line-height:1.5';
      document.body.append(box);
    }
    let busy = false;
    let timer;
    const render = (text, retry = false, library = false) => {
      box.replaceChildren(document.createTextNode(text));
      if (retry || library) {
        const button = document.createElement('button');
        button.type = 'button';
        button.textContent = retry ? 'Retry connection' : 'Review statements';
        button.style.cssText = 'margin:4px 0 0 8px;padding:4px 8px;background:#d5e8d5;color:#142417;border:0;border-radius:5px;cursor:pointer;font:inherit';
        button.addEventListener('click', retry ? refresh : () => {
          history.pushState({}, '', '/wealth/app/statements');
          window.dispatchEvent(new PopStateEvent('popstate'));
        });
        box.append(button);
      }
    };
    const refresh = async () => {
      if (busy) return;
      busy = true;
      clearTimeout(timer);
      try {
        const response = await fetch('/wealth/status', {cache:'no-store', signal:AbortSignal.timeout(5000)});
        if (!response.ok) throw new Error('unavailable');
        const state = await response.json();
        if (state.backend !== 'ready') render('The laptop backend is starting or unavailable. Saved statements are kept on the laptop.', true);
        else if (state.processing === 'unknown') render('Connected. Processing status is temporarily unavailable.', true);
        else if (state.processing?.processing || state.processing?.queued) render(`Processing ${state.processing.processing} statement(s) locally · ${state.processing.queued} waiting. You can leave this tab open.`, false, true);
        else if (state.processing?.failed) render(`Connected · ${state.processing.failed} statement(s) need attention. Retry saved files from the statement library.`, false, true);
        else render('Connected to the laptop' + (state.processing ? ' · No statements waiting for processing.' : '.'));
      } catch {
        render('Connection interrupted. The laptop may be asleep or reconnecting.', true);
      } finally {
        busy = false;
        timer = setTimeout(refresh, document.hidden ? 30000 : 10000);
      }
    };
    window.addEventListener('online', refresh);
    document.addEventListener('visibilitychange', () => { if (!document.hidden) void refresh(); });
    void refresh();
  };
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', init, {once:true});
  else init();
})();
