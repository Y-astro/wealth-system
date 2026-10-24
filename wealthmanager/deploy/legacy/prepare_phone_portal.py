from pathlib import Path
import os
import py_compile
import re
import shutil
import subprocess
import time

HOME = Path.home()
PREFIX = Path('/data/data/com.termux/files/usr')
dashboard = PREFIX/'bin/dashboard'
caddy = HOME/'.config/caddy/Caddyfile'
stamp = str(int(time.time()))
backup = HOME/('.config/dashboard/wealth-backup-'+stamp)
backup.mkdir(mode=0o700)
shutil.copy2(dashboard, backup/'dashboard')
shutil.copy2(caddy, backup/'Caddyfile')
source = dashboard.read_text()
assert 'Unlock Wealthfolio' not in source
card = '''<a href="/wealth/" class="golden-glass-card block p-6 rounded-3xl border border-amber-500/20 hover:border-amber-400/60 transition-all">
<span class="text-[10px] font-bold uppercase tracking-wider text-amber-400/80">Private finance</span>
<h3 class="text-xl font-bold text-white mt-1">Wealthfolio</h3>
<p class="text-sm text-amber-100/70 mt-2">Bank statements, cash flow and investments, streamed from the laptop.</p>
<p class="text-xs text-amber-200 mt-4">Separate password required · Laptop must be on</p>
<span class="gold-gradient-btn inline-block px-4 py-2 rounded-xl text-xs font-bold mt-4">Unlock Wealthfolio →</span></a>
'''
family_start = source.index('FAMILY_DASHBOARD_HTML = ')
family_end = source.index('ADMIN_DASHBOARD_HTML = ')
family = source[family_start:family_end]
marker = '    </div>\n\n    <!-- Footer -->'
assert family.count(marker) == 1
family = family.replace(marker, card+'\n'+marker)
source = source[:family_start]+family+source[family_end:]
marker = '<section class="golden-glass-card p-6 rounded-3xl space-y-4" aria-labelledby="laptop-heading">'
assert source.count(marker) == 1
source = source.replace(marker, card+'\n'+marker)
assert source.count('<script src="https://cdn.tailwindcss.com"></script>') == 3
source = source.replace('<script src="https://cdn.tailwindcss.com"></script>', '<link rel="stylesheet" href="/portal.css">')
source = '\n'.join(line for line in source.split('\n') if 'fonts.googleapis.com' not in line and 'fonts.gstatic.com' not in line)
pending = dashboard.with_name('dashboard.wealth-pending')
pending.write_text(source)
pending.chmod(dashboard.stat().st_mode)
py_compile.compile(str(pending), doraise=True)
config = caddy.read_text()
route = '''
    @portalPages path / /login /api/login /api/logout
    header @portalPages {
        Cache-Control "no-store, private"
        X-Content-Type-Options nosniff
        X-Frame-Options DENY
        Referrer-Policy no-referrer
        Content-Security-Policy "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self' data:; connect-src 'self'; frame-ancestors 'none'; object-src 'none'; base-uri 'self'; form-action 'self'"
    }
    handle /portal.css {
        root * /data/data/com.termux/files/home/.config/dashboard/static
        file_server
    }
    handle /wealth* {
        redir https://{host}:8444/wealth/ 303
    }
'''
assert config.count('encode zstd gzip') == 2
config = config.replace('encode zstd gzip', 'encode zstd gzip\n'+route)
config += '''
# Wealthfolio is a separate origin from uploaded files. All private bytes stay
# on the laptop; these loopback forwards are supplied by its SSH connection.
https://192.168.1.8:8444, https://192.168.1.10:8444, https://100.75.159.53:8444, https://redmi-k20-pro.tail939a82.ts.net:8444 {
    tls internal
    encode zstd gzip
    header {
        Cache-Control "no-store, private, max-age=0"
        Pragma "no-cache"
        X-Content-Type-Options nosniff
        X-Frame-Options DENY
        Referrer-Policy no-referrer
        -Server
        -Access-Control-Allow-Origin
        -Access-Control-Allow-Credentials
    }
    redir / /wealth/ 303
    redir /login https://{host}:8443/login 303
    handle /wealth/api/* {
        route {
            forward_auth 127.0.0.1:8001 {
                uri /api/auth-check
            }
            forward_auth 127.0.0.1:18930 {
                uri /wealth/_check
            }
            uri strip_prefix /wealth
            reverse_proxy 127.0.0.1:18080 {
                flush_interval -1
                header_up X-Real-IP {remote_host}
                header_down -Access-Control-Allow-Origin
                header_down -Access-Control-Allow-Credentials
            }
        }
    }
    handle /wealth/* {
        route {
            forward_auth 127.0.0.1:8001 {
                uri /api/auth-check
            }
            reverse_proxy 127.0.0.1:18930 {
                header_up X-Real-IP {remote_host}
                flush_interval -1
            }
        }
    }
    handle_errors {
        header Cache-Control no-store
        respond "Laptop unavailable. Switch on the laptop and sign in to its desktop, then retry Wealthfolio." 503
    }
}
'''
prepared = caddy.with_name('Caddyfile.wealth-pending')
prepared.write_text(config)
subprocess.run([str(PREFIX/'bin/caddy'), 'validate', '--config', str(prepared), '--adapter', 'caddyfile'], check=True)
(HOME/'.config/dashboard/wealth-backup-path').write_text(str(backup))
print('Prepared dashboard and Caddy configuration; backup:', backup)
