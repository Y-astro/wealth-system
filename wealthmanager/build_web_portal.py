"""Build the browser target and deploy only to the laptop's private static root."""
from pathlib import Path
import os
import re
import shutil
import subprocess
import tempfile
import time

BASE = Path(__file__).resolve().parent
FRONTEND = BASE.parent/'wealthfolio/apps/frontend'
destination = FRONTEND/'dist-portal'
stage = Path(tempfile.mkdtemp(prefix='.portal-build-',dir=FRONTEND))
try:
    subprocess.run(['pnpm','exec','vite','build','--base=/wealth/app/','--outDir='+str(stage)],
        cwd=FRONTEND,env={**os.environ,'BUILD_TARGET':'web'},check=True)
    index = stage/'index.html'
    source = index.read_text()
    # The upstream theme initializer is safe to self-host; no inline scripts
    # are needed under the portal's strict script CSP.
    count = 0
    def externalize(match):
        global count
        count += 1
        name = 'portal-init-'+str(count)+'.js'
        (stage/name).write_text(match.group(1))
        return '<script src="/wealth/app/'+name+'"></script>'
    source = re.sub(r'<script>\s*([\s\S]*?)</script>',externalize,source)
    marker = '<script type="module"'
    assert source.count(marker) == 1, 'Unexpected frontend entry point.'
    source = source.replace(marker,'<script src="/wealth/status.js" defer></script><script src="/wealth/app/portal.js"></script>'+marker,1)
    index.write_text(source)
    shutil.copy2(BASE/'portal-bootstrap.js',stage/'portal.js')
    backup = FRONTEND/('dist-portal.before-'+str(int(time.time())))
    if destination.exists(): destination.rename(backup)
    try: stage.rename(destination)
    except Exception:
        if backup.exists(): backup.rename(destination)
        raise
    print('Browser target deployed at',destination)
finally:
    if stage.exists(): shutil.rmtree(stage)
