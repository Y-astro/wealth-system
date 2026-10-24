"""Add a clear offline page to an already configured secure Redmi relay."""
from pathlib import Path
import re
import shutil
import subprocess
import time

PAGE = '''<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Wealthfolio · Laptop unavailable</title><style>body{margin:0;background:#111210;color:#eeede6;font:16px system-ui;min-height:100vh;display:grid;place-items:center;padding:24px;box-sizing:border-box}main{max-width:480px;padding:32px;border:1px solid #626558;border-radius:18px;background:#1b1c18}p{line-height:1.65;color:#b7b9ad}a{display:inline-block;margin:12px 12px 0 0;padding:12px 16px;border-radius:8px;background:#d5e8d5;color:#142417;text-decoration:none}.quiet{background:transparent;color:#d5e8d5;border:1px solid #626558}</style><main><p>WEALTHFOLIO · LAPTOP CONNECTION</p><h1>The laptop is not connected</h1><p>It may be asleep, restarting, or Wealthfolio may be switched off. Ask the laptop owner to start Wealthfolio and keep the laptop awake, then try again.</p><p>Your statements and processing results stay saved on the laptop. This page does not show a cached financial copy.</p><a href="/wealth/">Try again</a><a class="quiet" href="/portal">Back to home server</a></main></html>'''


def main():
    path = Path.home()/'.config/caddy/Caddyfile'
    source = path.read_text()
    if '# Wealthfolio offline status page' in source:
        print('Offline status page already configured.')
        return
    prefix, wealth = source.split('# Wealthfolio is a separate origin', 1)
    pattern = r'handle_errors\s*\{\s*header Cache-Control (?:"no-store"|no-store)\s*respond "[^"]*" 503\s*\}'
    assert len(re.findall(pattern, wealth)) == 1, 'Unexpected relay configuration; no changes made.'
    replacement = '''# Wealthfolio offline status page
    handle_errors {
        header Cache-Control "no-store, private"
        header Content-Type "text/html; charset=utf-8"
        header Content-Security-Policy "default-src 'none'; style-src 'unsafe-inline'; frame-ancestors 'none'; base-uri 'none'"
        respond `''' + PAGE + '''` 503
    }'''
    candidate = prefix + '# Wealthfolio is a separate origin' + re.sub(pattern, lambda match: replacement, wealth)
    pending = path.with_name('Caddyfile.status-pending')
    pending.write_text(candidate)
    caddy = '/data/data/com.termux/files/usr/bin/caddy'
    subprocess.run([caddy,'validate','--config',str(pending),'--adapter','caddyfile'],check=True)
    backup = path.with_name('Caddyfile.before-status-'+str(time.time_ns()))
    shutil.copy2(path, backup)
    try:
        shutil.copy2(pending, path)
        subprocess.run([caddy,'reload','--config',str(path),'--adapter','caddyfile'],check=True)
    except Exception:
        shutil.copy2(backup, path)
        subprocess.run([caddy,'reload','--config',str(path),'--adapter','caddyfile'],check=True)
        raise
    print('Installed the laptop offline page; existing authentication and HTTPS rules retained.')


if __name__ == '__main__':
    main()
