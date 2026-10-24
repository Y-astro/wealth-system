from pathlib import Path
import json
import os
import shutil
import subprocess
import time

ROOT = Path('/home/astro/wealth-system')
BASE = ROOT / 'wealthmanager'
STAGE = Path('/tmp/wealth-portal-stage')
PRIVATE = BASE / 'data/web-portal'
PRIVATE.mkdir(parents=True, exist_ok=True, mode=0o700)
PRIVATE.chmod(0o700)
processes = []
for proc in Path('/proc').iterdir():
    if not proc.name.isdigit(): continue
    try:
        exe = Path(os.readlink(proc / 'exe'))
        if exe.name == 'wealthfolio-server': processes.append((proc, exe))
    except (OSError, PermissionError): pass
assert len(processes) == 1, 'Expected exactly one current Wealthfolio backend.'
proc, exe = processes[0]
cwd = Path(os.readlink(proc / 'cwd'))
env = dict(item.split('=', 1) for item in (proc/'environ').read_bytes().decode().split('\0') if '=' in item)
selected = {k:v for k,v in env.items() if k.startswith(('WF_', 'WEALTH_', 'LM_STUDIO_'))}
databases = []
for fd in (proc/'fd').iterdir():
    try:
        target = Path(os.readlink(fd))
        if target.name.endswith('.db') and target.is_file(): databases.append(target)
    except OSError: pass
db = Path(selected.get('WF_DB_PATH', str(cwd/'db/app.db')))
if not db.is_absolute(): db = cwd/db
if not db.is_file():
    assert len(set(databases)) == 1, 'Cannot safely identify existing database.'
    db = databases[0]
selected['WF_DB_PATH'] = str(db.resolve())
selected['WF_STATIC_DIR'] = str(ROOT/'wealthfolio/apps/frontend/dist')
config = PRIVATE/'backend.json'
if not config.exists():
    config.write_text(json.dumps({'exe': str(exe), 'cwd': str(cwd), 'env': selected}))
    config.chmod(0o600)
for name in ('wealth_portal.py', 'wealth_portal_supervisor.py'):
    destination = BASE/name
    if destination.exists(): shutil.copy2(destination, str(destination)+'.before-'+str(int(time.time())))
    shutil.copy2(STAGE/name, destination)
    destination.chmod(0o600)
tests = BASE/'tests/test_wealth_portal.py'
shutil.copy2(STAGE/'test_wealth_portal.py', tests)
route = ROOT/'wealthfolio/apps/frontend/src/routes.tsx'
source = route.read_text()
if '<BrowserRouter>' in source:
    assert source.count('<BrowserRouter>') == 1
    route.write_text(source.replace('<BrowserRouter>', '<BrowserRouter basename={import.meta.env.BASE_URL}>'))
unit = Path.home()/'.config/systemd/user/wealth-portal.service'
unit.parent.mkdir(parents=True, exist_ok=True)
unit.write_text('''[Unit]
Description=Private Wealthfolio laptop gate and Redmi connection
After=network-online.target

[Service]
Type=simple
WorkingDirectory=/home/astro/wealth-system/wealthmanager
ExecStart=/usr/bin/python3 /home/astro/wealth-system/wealthmanager/wealth_portal_supervisor.py
Restart=on-failure
RestartSec=5
UMask=0077
NoNewPrivileges=true
PrivateTmp=true

[Install]
WantedBy=default.target
''')
subprocess.run(['systemctl', '--user', 'daemon-reload'], check=True)
print('Laptop files staged; existing database preserved; service ready to enable after configuration.')
