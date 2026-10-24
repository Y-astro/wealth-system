"""Open the existing admin/family Wealthfolio cards in a new tab."""
import ast
import os
from pathlib import Path
import re
import shutil
import signal
import time

path = Path('/data/data/com.termux/files/usr/bin/dashboard')
source = path.read_text()
pattern = re.compile(r'<a\b[^>]*\bhref="/wealth/"[^>]*>')
links = list(pattern.finditer(source))
if len(links) != 2:
    raise SystemExit('Expected exactly the admin and family Wealthfolio links; no changes made.')

def update(match):
    tag = re.sub(r'\s+(?:target|rel)="[^"]*"', '', match.group())
    return tag[:-1] + ' target="_blank" rel="noopener noreferrer">'

candidate = pattern.sub(update, source)
tree = ast.parse(candidate, filename=str(path))
for name in ('FAMILY_DASHBOARD_HTML', 'ADMIN_DASHBOARD_HTML'):
    values = [node.value for node in tree.body if isinstance(node, ast.Assign)
              and any(isinstance(target, ast.Name) and target.id == name for target in node.targets)]
    if len(values) != 1:
        raise SystemExit('Could not verify both dashboard templates; no changes made.')
    document = ast.literal_eval(values[0])
    tags = pattern.findall(document)
    assert len(tags) == 1 and 'target="_blank"' in tags[0] and 'rel="noopener noreferrer"' in tags[0]

if candidate == source:
    print('Already configured: both Wealthfolio cards open in a new tab.')
else:
    backup = Path.home() / '.config/dashboard' / ('dashboard.before-new-tab-' + str(time.time_ns()))
    backup.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(path, backup)
    pending = path.with_name('dashboard.new-tab-pending')
    pending.write_text(candidate)
    pending.chmod(path.stat().st_mode)
    os.replace(pending, path)
    for process in Path('/proc').iterdir():
        if not process.name.isdigit():
            continue
        try:
            argv = (process / 'cmdline').read_bytes().split(b'\0')
            if len(argv) > 2 and argv[1] == b'-u' and argv[2] == bytes(path):
                os.kill(int(process.name), signal.SIGTERM)
        except OSError:
            pass
    print('Updated admin and family links. The supervisor will reload the dashboard; refresh the portal.')
