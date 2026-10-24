"""User-session service: private website gate and encrypted loopback forwards."""
from pathlib import Path
import os
import signal
import subprocess
import sys
import time

BASE = Path(__file__).resolve().parent
stopping = False
children = {}

def stop(*args):
    global stopping
    stopping = True

def main():
    signal.signal(signal.SIGTERM, stop)
    signal.signal(signal.SIGINT, stop)
    commands = {
        'backend': [sys.executable, str(BASE / 'wealth_backend.py')],
        'gate': [sys.executable, str(BASE / 'wealth_portal.py')],
        'tunnel': ['/usr/bin/ssh', '-F', '/dev/null', '-NT', '-p', '8022', '-o', 'BatchMode=yes',
                   '-o', 'ExitOnForwardFailure=yes', '-o', 'ServerAliveInterval=20', '-o', 'ServerAliveCountMax=3',
                   '-L', '127.0.0.1:18001:127.0.0.1:8001',
                   '-R', '127.0.0.1:18930:127.0.0.1:8930',
                   '-R', '127.0.0.1:18080:127.0.0.1:8080', 'u0_a505@100.75.159.53'],
    }
    try:
        while not stopping:
            for name, command in commands.items():
                process = children.get(name)
                if process is None or process.poll() is not None:
                    children[name] = subprocess.Popen(command, stdin=subprocess.DEVNULL,
                        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, cwd=BASE,
                        env={**os.environ, 'PYTHONDONTWRITEBYTECODE': '1'})
            time.sleep(3)
    finally:
        for process in children.values():
            if process.poll() is None: process.terminate()
        for process in children.values():
            try: process.wait(timeout=5)
            except subprocess.TimeoutExpired: process.kill()

if __name__ == '__main__': main()
