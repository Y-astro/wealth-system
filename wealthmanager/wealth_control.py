"""One lifecycle for the laptop backend, optional local website and Redmi relay."""
import argparse
import socket
import subprocess
import time
import urllib.error
import urllib.request


def listening(port):
    try:
        with socket.create_connection(('127.0.0.1', port), timeout=0.5):
            return True
    except OSError:
        return False


def active(unit):
    return subprocess.run(['systemctl', '--user', 'is-active', '--quiet', unit], timeout=3).returncode == 0


def run(action, local=False):
    if action == 'stop':
        subprocess.run(['systemctl', '--user', 'stop', 'wealth-local.service', 'wealth-portal.service'], check=True)
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline and any(listening(p) for p in (1420, 8080, 8930)):
            time.sleep(0.25)
        if any(listening(p) for p in (1420, 8080, 8930)):
            raise SystemExit('A separately started Wealthfolio process is still listening. Check ports 1420, 8080 and 8930.')
        print('Wealthfolio stopped: backend, localhost and access through the Redmi.')
        return
    if not local:
        subprocess.run(['systemctl', '--user', 'stop', 'wealth-local.service'], check=True)
    units = ['wealth-portal.service'] + (['wealth-local.service'] if local else [])
    subprocess.run(['systemctl', '--user', 'start', *units], check=True)
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        try:
            if not all(active(unit) for unit in units) or not listening(8080):
                raise OSError('Backend starting')
            urls = ['http://127.0.0.1:8930/healthz'] + (['http://localhost:1420/'] if local else [])
            for url in urls:
                with opener.open(url, timeout=1) as response:
                    if response.status != 200:
                        raise OSError('Website starting')
            break
        except (OSError, urllib.error.URLError):
            time.sleep(0.25)
    else:
        raise SystemExit('Wealthfolio is not ready. Check: systemctl --user status wealth-portal wealth-local')
    print('Wealthfolio backend and Redmi portal service are running.')
    if local:
        print('Local website: http://localhost:1420/ (same production build and Wealthfolio password).')
    else:
        print('Local website is off. Use wf-start --local to enable it.')
    print('Redmi connection: ' + ('connected' if listening(18001) else 'waiting for the phone; reconnects automatically') + '.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=('start', 'stop'))
    parser.add_argument('--local', action='store_true', help='also enable the loopback production website')
    args = parser.parse_args()
    if args.action == 'stop' and args.local:
        parser.error('wf-stop stops both entry points; --local is unnecessary')
    run(args.action, args.local)


if __name__ == '__main__':
    main()
