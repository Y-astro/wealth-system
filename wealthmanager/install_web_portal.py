"""Install the user service/commands, adopting only the configured backend."""
from pathlib import Path
import json
import os
import re
import shutil
import signal
import sqlite3
import subprocess
import time

from wealth_backend import BASE, configuration


def main():
    config = configuration()
    private = BASE / 'data/web-portal'
    db = BASE / 'data/statements/library.sqlite3'
    if db.is_file():
        with sqlite3.connect(db.as_uri() + '?mode=ro', uri=True) as connection:
            busy = connection.execute("SELECT COUNT(*) FROM uploads WHERE archived=0 AND status IN ('queued','processing')").fetchone()[0]
            if busy:
                raise SystemExit('Finish or pause active statement processing before installing the managed backend.')
    old_pid = None
    listeners = subprocess.check_output(['ss', '-ltnp', 'sport = :8080'], text=True)
    match = re.search(r'pid=(\d+)', listeners)
    if match:
        pid = int(match[1])
        proc = Path('/proc') / str(pid)
        if (proc / 'exe').resolve() != Path(config['exe']).resolve():
            raise SystemExit('Port 8080 belongs to a different executable; no services changed.')
        if (proc / 'cwd').resolve() != Path(config['cwd']).resolve():
            raise SystemExit('Existing backend working directory differs; no services changed.')
        environment = dict(item.split(b'=', 1) for item in (proc / 'environ').read_bytes().split(b'\0') if b'=' in item)
        actual_db = environment.get(b'WF_DB_PATH')
        if actual_db:
            actual_path = Path(os.fsdecode(actual_db))
            if not actual_path.is_absolute():
                actual_path = Path(config['cwd']) / actual_path
            if actual_path.resolve() != Path(config['env']['WF_DB_PATH']).resolve():
                raise SystemExit('Existing backend database differs; no services changed.')
        if 'wealth-portal.service' not in (proc / 'cgroup').read_text():
            old_pid = pid
    home = Path.home()
    units = home / '.config/systemd/user'
    dropin = units / 'wealth-portal.service.d/local.conf'
    if dropin.exists() and dropin.read_text() != '[Unit]\nWants=wealth-local.service\n':
        raise SystemExit('The local service drop-in has custom changes; no services changed.')
    backup = private / ('deployment-before-' + str(time.time_ns()))
    backup.mkdir(mode=0o700, parents=True)
    for path in (units/'wealth-portal.service', units/'wealth-local.service', dropin,
                 home/'.local/bin/wf-start', home/'.local/bin/wf-stop', home/'.local/bin/wf-start--local'):
        if path.is_file():
            shutil.copy2(path, backup / (str(path.relative_to(home)).replace('/', '_')))
    # This is the requested backend ownership migration, not an availability test.
    subprocess.run(['systemctl', '--user', 'stop', 'wealth-local.service', 'wealth-portal.service'], check=True)
    try:
        if old_pid is not None:
            try:
                os.kill(old_pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
            deadline = time.monotonic() + 15
            while Path('/proc', str(old_pid)).exists() and time.monotonic() < deadline:
                time.sleep(0.1)
            if Path('/proc', str(old_pid)).exists():
                raise RuntimeError('The old backend did not stop; refusing to launch a duplicate.')
        if dropin.exists():
            dropin.unlink()
        units.mkdir(parents=True, exist_ok=True)
        (units/'wealth-portal.service').write_text(f'''[Unit]
Description=Wealthfolio backend and private Redmi connection
After=network-online.target

[Service]
Type=simple
WorkingDirectory={BASE}
ExecStart=/usr/bin/python3 {BASE}/wealth_portal_supervisor.py
Restart=on-failure
RestartSec=3
TimeoutStopSec=30
KillMode=control-group
UMask=0077
NoNewPrivileges=true
PrivateTmp=true

[Install]
WantedBy=default.target
''')
        (units/'wealth-local.service').write_text(f'''[Unit]
Description=Optional Wealthfolio production website on localhost:1420
Requires=wealth-portal.service
After=wealth-portal.service
PartOf=wealth-portal.service

[Service]
Type=simple
WorkingDirectory={BASE}
ExecStart=/usr/bin/python3 {BASE}/wealth_local.py
Restart=on-failure
RestartSec=3
UMask=0077
NoNewPrivileges=true
PrivateTmp=true
''')
        commands = home / '.local/bin'
        commands.mkdir(parents=True, exist_ok=True)
        for name, action in (('wf-start', 'start'), ('wf-stop', 'stop')):
            path = commands / name
            path.write_text(f'#!/bin/sh\nexec /usr/bin/python3 "{BASE}/wealth_control.py" {action} "$@"\n')
            path.chmod(0o755)
        shortcut = commands / 'wf-start--local'
        shortcut.write_text(f'#!/bin/sh\nexec "{commands}/wf-start" --local "$@"\n')
        shortcut.chmod(0o755)
        subprocess.run(['systemctl', '--user', 'daemon-reload'], check=True)
        subprocess.run(['systemctl', '--user', 'enable', 'wealth-portal.service'], check=True)
    finally:
        subprocess.run(['systemctl', '--user', 'start', 'wealth-portal.service'], check=True)
    print('Installed: wf-start, wf-start --local (also wf-start--local), and wf-stop.')
    print('The local production website is off by default; the backend is now owned by wealth-portal.service.')


if __name__ == '__main__':
    main()
