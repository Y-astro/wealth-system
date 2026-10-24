from pathlib import Path
import shutil
import subprocess

base = Path('/home/astro/wealth-system/wealthmanager')
if base.is_dir():
    path = base/'wealth_portal.py'
    path.write_text(path.read_text().replace("'Referrer-Policy': 'no-referrer'", "'Referrer-Policy': 'same-origin'"))
    subprocess.run(['systemctl','--user','restart','wealth-portal.service'],check=True)
else:
    path = Path.home()/'.config/caddy/Caddyfile'
    source = path.read_text().replace('Referrer-Policy no-referrer', 'Referrer-Policy same-origin')
    source = source.replace('redir /login https://{host}:8443/login 303', 'redir /login https://{host}:8443/login 303\n    redir /portal https://{host}:8443/ 303')
    candidate = path.with_name('Caddyfile.form-fix')
    candidate.write_text(source)
    caddy = '/data/data/com.termux/files/usr/bin/caddy'
    subprocess.run([caddy,'validate','--config',str(candidate),'--adapter','caddyfile'],check=True)
    shutil.copy2(path,path.with_name('Caddyfile.before-form-fix'))
    shutil.copy2(candidate,path)
    subprocess.run([caddy,'reload','--config',str(path),'--adapter','caddyfile'],check=True)
print('Same-origin form referrer policy applied; cross-origin validation retained.')
