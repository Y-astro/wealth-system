"""Laptop-only secondary authentication and static Wealthfolio presentation.

No financial data is stored here. Caddy streams API traffic directly through a
loopback SSH forward after consulting /wealth/_check. The Redmi portal session
is independently verified through the same SSH connection in the other direction.
"""
from __future__ import annotations
from collections import defaultdict, deque
import hashlib
import hmac
import html
from http import cookies
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import http.client
import json
import mimetypes
import os
from pathlib import Path
import secrets
import threading
import time
from urllib.parse import parse_qs, unquote, urlsplit
from argon2 import PasswordHasher
from argon2.exceptions import VerificationError, InvalidHashError

BASE = Path(__file__).resolve().parent
CONFIG = BASE / 'data/web-portal/auth.json'
DIST = BASE.parent / 'wealthfolio/apps/frontend/dist-portal'
COOKIE = '__Secure-wealth_session'
CHALLENGE = '__Secure-wealth_challenge'
PREFIX = '/wealth/'
PH = PasswordHasher(time_cost=3, memory_cost=65536, parallelism=1)
PRIMARY_CACHE = {}
PRIMARY_CACHE_LOCK = threading.Lock()
HEADERS = {
    'Cache-Control': 'no-store, private, max-age=0', 'Pragma': 'no-cache',
    'X-Content-Type-Options': 'nosniff', 'X-Frame-Options': 'DENY',
    'Referrer-Policy': 'same-origin',
    'Permissions-Policy': 'camera=(), microphone=(), geolocation=()',
    'Content-Security-Policy': "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self' data:; connect-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self'; object-src 'none'",
}


class State:
    def __init__(self, config, clock=time.monotonic):
        self.config, self.clock = config, clock
        self.secret = secrets.token_bytes(32)
        self.sessions, self.challenges = {}, {}
        self.failures = defaultdict(deque)
        self.lock = threading.Lock()
        self.verifiers = threading.BoundedSemaphore(2)

    def binding(self, token):
        return hmac.new(self.secret, token.encode(), hashlib.sha256).hexdigest()

    def prune(self):
        now = self.clock()
        self.challenges = {k:v for k,v in self.challenges.items() if now-v['at'] < 600}
        self.sessions = {k:v for k,v in self.sessions.items() if now-v['seen'] < 900 and now-v['at'] < 14400}
        for key in list(self.failures):
            while self.failures[key] and now-self.failures[key][0] >= 900:
                self.failures[key].popleft()
            if not self.failures[key]: del self.failures[key]

    def allowed(self, user, client):
        self.prune()
        return (len(self.failures.get('all', ())) < 30 and
                len(self.failures.get('user:'+user, ())) < 5 and
                len(self.failures.get('ip:'+client, ())) < 10)

    def failed(self, user, client):
        for key in ('all', 'user:'+user, 'ip:'+client):
            self.failures[key].append(self.clock())
        # Session-protected entry points plus bounded dictionaries limit RAM use.
        while len(self.failures) > 4096: self.failures.pop(next(iter(self.failures)))


def primary_user(cookie_header, admin=False):
    jar = cookies.SimpleCookie()
    try: jar.load(cookie_header)
    except cookies.CookieError: return None
    token = jar.get('session_token')
    if not token or len(token.value) > 2048: return None
    # The Redmi still validates the primary session on every proxied request.
    # Coalesce the laptop's independent verification during each asset burst.
    key = (admin, hashlib.sha256(token.value.encode()).digest())
    now = time.monotonic()
    with PRIMARY_CACHE_LOCK:
        cached = PRIMARY_CACHE.get(key)
        if cached and now-cached[1] < 3: return cached[0], token.value
    connection = http.client.HTTPConnection('127.0.0.1', 18001, timeout=4)
    try:
        connection.request('GET', '/api/auth-check-admin' if admin else '/api/auth-check', headers={'Cookie': 'session_token='+token.value, 'X-Forwarded-Uri': '/wealth/api/v1/'})
        response = connection.getresponse()
        user = response.getheader('X-Forwarded-User') if response.status == 200 else None
        if user:
            with PRIMARY_CACHE_LOCK:
                while len(PRIMARY_CACHE) >= 256: PRIMARY_CACHE.pop(next(iter(PRIMARY_CACHE)))
                PRIMARY_CACHE[key] = (user, time.monotonic())
        return (user, token.value) if user else None
    except (OSError, ValueError, http.client.HTTPException): return None
    finally: connection.close()


def login_html(csrf, message=''):
    return '''<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Wealthfolio · Private access</title><style>
*{box-sizing:border-box}body{margin:0;background:#111210;color:#eeede6;font:16px system-ui,sans-serif;min-height:100vh;display:grid;place-items:center;padding:24px}main{width:min(100%,480px);padding:36px;border:1px solid #3c3d36;border-radius:20px;background:#1b1c18}.eyebrow{color:#a5c9ae;font-size:12px;letter-spacing:.15em;text-transform:uppercase}h1{font-size:30px;margin:12px 0}p{color:#b7b9ad;line-height:1.6}label{display:block;margin:28px 0 9px}input{width:100%;background:#111210;border:1px solid #626558;color:#fff;font:inherit;padding:13px;border-radius:9px}button{margin-top:18px;width:100%;padding:14px;border:0;border-radius:9px;background:#d5e8d5;color:#142417;font:600 16px system-ui;cursor:pointer}a{color:#bdd6bc}.small{font-size:13px}.error{color:#ffc1b6}input:focus{outline:2px solid #a3c9a3;outline-offset:2px}</style>
<main><div class="eyebrow">K20 Pro · Laptop connection</div><h1>Unlock Wealthfolio</h1><p>Your portal login is verified. Enter the separate Wealthfolio password to open the dashboard.</p>''' + (
        '<p role="alert" class="error">'+html.escape(message)+'</p>' if message else '') + '''
<form method="post" action="/wealth/login"><input type="hidden" name="csrf" value="'''+html.escape(csrf, quote=True)+'''">
<label for="password">Wealthfolio password</label><input id="password" name="password" type="password" required maxlength="1024" autocomplete="off" autofocus>
<button type="submit">Open dashboard</button></form><p class="small">The laptop must stay switched on. Statements, balances and AI processing remain on the laptop; the Redmi relays the connection.</p><a class="small" href="/portal">Back to portal</a></main></html>'''


class Handler(BaseHTTPRequestHandler):
    server_version = 'PrivatePortal'
    sys_version = ''
    session_cookie = COOKIE
    challenge_cookie = CHALLENGE
    secure_cookie = True

    def setup(self):
        super().setup()
        self.connection.settimeout(15)

    def log_message(self, *args): pass  # No URLs, cookies, passwords or request bodies in logs.

    def send(self, status, body=b'', kind='text/html; charset=utf-8', extra=(), immutable=False):
        if isinstance(body, str): body = body.encode()
        self.send_response(status)
        for key, value in HEADERS.items():
            if immutable and key == 'Pragma': continue
            if immutable and key == 'Cache-Control': value = 'private, max-age=31536000, immutable'
            self.send_header(key, value)
        self.send_header('Content-Type', kind)
        self.send_header('Content-Length', str(len(body)))
        for key, value in extra: self.send_header(key, value)
        self.end_headers()
        if self.command != 'HEAD': self.wfile.write(body)

    def cookie(self, name):
        jar = cookies.SimpleCookie()
        try: jar.load(self.headers.get('Cookie', ''))
        except cookies.CookieError: return ''
        return jar[name].value if name in jar else ''

    def set_cookie(self, name, value, age=None):
        # Session cookies have no Max-Age and are never placed in JS storage.
        secure = '; Secure' if self.secure_cookie else ''
        return ('Set-Cookie', f'{name}={value}; Path=/wealth/{secure}; HttpOnly; SameSite=Strict'+(f'; Max-Age={age}' if age is not None else ''))

    def identity(self):
        if self.headers.get('X-Forwarded-Proto') != 'https':
            self.send(403, 'Open Wealthfolio using the HTTPS portal.'); return None
        if self.headers.get('Origin') and not self.same_origin():
            self.send(403, 'Cross-origin request rejected.'); return None
        verified = primary_user(self.headers.get('Cookie', ''))
        if not verified:
            self.send(401, '{"message":"Sign in to the server portal first."}', 'application/json'); return None
        user, token = verified
        return user, self.server.state.binding(token)

    def same_origin(self):
        return (self.headers.get('Origin') in self.server.state.config['origins'] and
                self.headers.get('Sec-Fetch-Site', 'same-origin') not in ('cross-site', 'none'))

    def session(self, binding):
        state = self.server.state
        with state.lock:
            state.prune()
            entry = state.sessions.get(self.cookie(self.session_cookie))
            if entry and hmac.compare_digest(entry['binding'], binding):
                entry['seen'] = state.clock()
                return entry
        return None

    def show_login(self, binding, message='', status=200):
        state = self.server.state
        token = secrets.token_urlsafe(32)
        with state.lock:
            state.prune()
            state.sessions.pop(self.cookie(self.session_cookie), None)
            if len(state.challenges) >= 512: state.challenges.pop(next(iter(state.challenges)))
            state.challenges[token] = {'binding': binding, 'at': state.clock()}
        page = login_html(token, message)
        if not self.secure_cookie:
            page = page.replace('Your portal login is verified.', 'You are connected directly on this laptop.').replace('<a class="small" href="/portal">Back to portal</a>', '')
        page = page.replace('</main>', '<p id="wealth-connection" class="small" role="status">Checking laptop services…</p></main><script src="/wealth/status.js" defer></script>')
        self.send(status, page, extra=[self.set_cookie(self.challenge_cookie, token, 600), self.set_cookie(self.session_cookie, '', 0)])

    def do_HEAD(self): self.do_GET()

    def do_GET(self):
        if self.path == '/healthz':
            self.send(200, '{"status":"ok"}', 'application/json'); return
        identity = self.identity()
        if not identity: return
        user, binding = identity
        path = urlsplit(self.path).path
        if path == '/wealth/status.js':
            self.send(200, (BASE / 'portal-status.js').read_bytes(), 'application/javascript'); return
        if path in ('/wealth', '/wealth/'):
            self.show_login(binding); return
        entry = self.session(binding)
        if path == '/wealth/status':
            from wealth_status import snapshot
            self.send(200, json.dumps(snapshot(include_processing=bool(entry))), 'application/json'); return
        if path == '/wealth/_check':
            method = self.headers.get('X-Forwarded-Method', 'GET')
            if not entry:
                self.send(401, '{"message":"Wealthfolio session expired. Open Wealthfolio from the portal to sign in again."}', 'application/json'); return
            if method not in ('GET', 'HEAD', 'OPTIONS') and not self.same_origin():
                self.send(403, '{"message":"Cross-site request rejected."}', 'application/json'); return
            self.send(200, '', 'text/plain'); return
        if not path.startswith('/wealth/app/'):
            self.send(404, 'Not found.'); return
        if not entry:
            self.send(303, extra=[('Location', '/wealth/')]); return
        # Each fresh dashboard navigation requires a new password. A successful
        # login permits exactly its initial document; SPA navigation stays usable.
        document = self.headers.get('Sec-Fetch-Dest') == 'document' or 'text/html' in self.headers.get('Accept', '')
        if document:
            with self.server.state.lock:
                permitted = entry.pop('initial_document', False)
            if not permitted:
                self.show_login(binding); return
        relative = unquote(path[len('/wealth/app/'):])
        root = self.server.dist.resolve()
        candidate = (root / relative).resolve()
        if not candidate.is_relative_to(root) or '\\' in relative:
            self.send(404, 'Not found.'); return
        if not candidate.is_file():
            if relative.startswith('assets/') or Path(relative).suffix:
                self.send(404, 'Not found.'); return
            candidate = root / 'index.html'
        try:
            body = candidate.read_bytes()
        except OSError:
            self.send(503, 'The laptop website is rebuilding. Please retry shortly.'); return
        kind = mimetypes.guess_type(candidate.name)[0] or 'application/octet-stream'
        self.send(200, body, kind, immutable=relative.startswith('assets/'))

    def do_POST(self):
        identity = self.identity()
        if not identity: return
        user, binding = identity
        if not self.same_origin(): self.send(403, 'Cross-site request rejected.'); return
        if self.path == '/wealth/logout':
            with self.server.state.lock: self.server.state.sessions.pop(self.cookie(self.session_cookie), None)
            self.send(303, extra=[('Location', '/wealth/'), self.set_cookie(self.session_cookie, '', 0)]); return
        if self.path != '/wealth/login': self.send(404, 'Not found.'); return
        try: length = int(self.headers.get('Content-Length', '0'))
        except ValueError: length = 0
        if not 0 < length <= 4096 or self.headers.get('Transfer-Encoding'):
            self.send(413, 'Invalid login request.'); return
        try: fields = parse_qs(self.rfile.read(length).decode('utf-8', errors='replace'), max_num_fields=4)
        except ValueError:
            self.send(400, 'Invalid login request.'); return
        token = fields.get('csrf', [''])[0]
        password = fields.get('password', [''])[0]
        state = self.server.state
        client = self.headers.get('X-Real-IP', '')[:80]
        with state.lock:
            state.prune()
            challenge = state.challenges.pop(token, None)
            valid = challenge and hmac.compare_digest(token, self.cookie(self.challenge_cookie)) and hmac.compare_digest(challenge['binding'], binding)
            allowed = state.allowed(user, client)
        if not valid:
            self.show_login(binding, 'The login form expired. Please try again.', 403); return
        if not allowed:
            self.show_login(binding, 'Too many attempts. Please wait 15 minutes.', 429); return
        if not state.verifiers.acquire(blocking=False):
            self.show_login(binding, 'Another login is being checked. Please retry.', 429); return
        try:
            correct = False
            if 0 < len(password.encode()) <= 1024:
                try: correct = PH.verify(state.config['password_hash'], password)
                except (VerificationError, InvalidHashError): pass
        finally:
            password = ''
            state.verifiers.release()
        if not correct:
            with state.lock: state.failed(user, client)
            self.show_login(binding, 'Incorrect password. Please try again.', 401); return
        opaque = secrets.token_urlsafe(32)
        with state.lock:
            if len(state.sessions) >= 256: state.sessions.pop(next(iter(state.sessions)))
            state.sessions[opaque] = {'binding': binding, 'at': state.clock(), 'seen': state.clock(), 'initial_document': True}
        self.send(303, extra=[('Location', '/wealth/app/'), self.set_cookie(self.session_cookie, opaque), self.set_cookie(self.challenge_cookie, '', 0)])


def main():
    if not CONFIG.exists():
        from wealth_portal_setup import main as setup
        setup()
        return
    config = json.loads(CONFIG.read_text())
    if not config['password_hash'].startswith('$argon2id$'):
        raise SystemExit('Invalid private portal configuration.')
    server = ThreadingHTTPServer(('127.0.0.1', 8930), Handler)
    server.daemon_threads = True
    server.state, server.dist = State(config), DIST
    server.serve_forever()


if __name__ == '__main__': main()
