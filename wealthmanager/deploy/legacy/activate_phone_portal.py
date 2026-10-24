from pathlib import Path
import os
import shutil
import signal
import subprocess

home = Path.home()
prefix = Path('/data/data/com.termux/files/usr')
dashboard = prefix/'bin/dashboard'
caddy = home/'.config/caddy/Caddyfile'
assert (home/'.config/dashboard/static/portal.css').is_file()
backup = Path((home/'.config/dashboard/wealth-backup-path').read_text())
shutil.copy2(dashboard.with_name('dashboard.wealth-pending'), dashboard)
shutil.copy2(caddy.with_name('Caddyfile.wealth-pending'), caddy)
try:
    subprocess.run([str(prefix/'bin/caddy'), 'reload', '--config', str(caddy), '--adapter', 'caddyfile'], check=True)
except Exception:
    shutil.copy2(backup/'dashboard', dashboard)
    shutil.copy2(backup/'Caddyfile', caddy)
    raise
for process in Path('/proc').iterdir():
    if not process.name.isdigit(): continue
    try:
        argv = (process/'cmdline').read_bytes().split(b'\0')
        if len(argv)>2 and argv[1] == b'-u' and argv[2] == bytes(dashboard):
            os.kill(int(process.name), signal.SIGTERM)
    except (OSError, PermissionError): pass
print('Portal activated. The existing supervisor will restart the dashboard.')
