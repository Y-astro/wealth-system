"""One-time administrator setup; the human enters the new credential."""
from http.server import HTTPServer
import hmac
import json
import os
import secrets
import threading
from urllib.parse import parse_qs, urlsplit
import wealth_portal as gate

ORIGINS = ['https://'+host+':8444' for host in ('192.168.1.8','192.168.1.10','100.75.159.53','redmi-k20-pro.tail939a82.ts.net')]
SETUP_COOKIE = '__Secure-wealth_setup'

class Handler(gate.Handler):
    def administrator(self):
        if self.headers.get('X-Forwarded-Proto') != 'https':
            self.send(403, 'HTTPS required.'); return None
        user = gate.primary_user(self.headers.get('Cookie', ''), admin=True)
        if not user:
            self.send(403, 'A portal administrator must finish the one-time Wealthfolio setup.'); return None
        return self.server.state.binding(user[1])

    def show(self, binding, message=''):
        state = self.server.state
        token = secrets.token_urlsafe(32)
        with state.lock:
            state.prune()
            if len(state.challenges) >= 512: state.challenges.pop(next(iter(state.challenges)))
            state.challenges[token] = {'binding':binding, 'at':state.clock()}
        body = gate.login_html(token, message)
        body = body.replace('Unlock Wealthfolio', 'Finish Wealthfolio setup').replace('/wealth/login', '/wealth/setup')
        body = body.replace('Your portal login is verified. Enter the separate Wealthfolio password to open the dashboard.',
            'Set the separate password you requested. Only a salted password hash will be saved, on the Linux laptop.')
        body = body.replace('required maxlength="1024"', 'required minlength="8" maxlength="1024"')
        body = body.replace('<button type="submit">Open dashboard</button>',
            '<label for="confirm">Confirm password</label><input id="confirm" name="confirm" type="password" required minlength="8" maxlength="1024" autocomplete="off"><button type="submit">Save password on laptop</button>')
        self.send(200, body, extra=[self.set_cookie(SETUP_COOKIE, token, 600)])

    def do_GET(self):
        binding = self.administrator()
        if not binding: return
        if urlsplit(self.path).path not in ('/wealth/', '/wealth/setup'):
            self.send(503, 'Finish Wealthfolio setup from the portal before opening the dashboard.'); return
        self.show(binding)

    def do_POST(self):
        binding = self.administrator()
        if not binding: return
        if self.path != '/wealth/setup': self.send(404, 'Not found.'); return
        if not self.same_origin(): self.send(403, 'Cross-site request rejected.'); return
        try: length = int(self.headers.get('Content-Length', '0'))
        except ValueError: length = 0
        if not 0 < length <= 4096 or self.headers.get('Transfer-Encoding'):
            self.send(413, 'Invalid setup request.'); return
        try: values = parse_qs(self.rfile.read(length).decode(), max_num_fields=3)
        except (ValueError, UnicodeError): self.send(400, 'Invalid setup request.'); return
        token = values.get('csrf',[''])[0]
        with self.server.state.lock:
            self.server.state.prune()
            challenge = self.server.state.challenges.pop(token, None)
        if not (challenge and hmac.compare_digest(token, self.cookie(SETUP_COOKIE)) and hmac.compare_digest(binding, challenge['binding'])):
            self.send(403, 'The setup form expired. Reopen Wealthfolio.'); return
        password, confirm = values.pop('password',[''])[0], values.pop('confirm',[''])[0]
        if not (8 <= len(password) and len(password.encode()) <= 1024 and hmac.compare_digest(password.encode(), confirm.encode())):
            self.show(binding, 'Enter matching passwords of 8 or more characters.'); return
        config = {'password_hash':gate.PH.hash(password), 'origins':ORIGINS}
        password = confirm = ''
        path = self.server.config_path
        path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
        try: fd = os.open(path, os.O_WRONLY|os.O_CREAT|os.O_EXCL, 0o600)
        except FileExistsError: self.send(409, 'Setup has already completed. Reopen Wealthfolio.'); return
        with os.fdopen(fd,'w') as stream:
            json.dump(config, stream)
            stream.flush()
            os.fsync(stream.fileno())
        self.send(200, '<!doctype html><title>Wealthfolio ready</title><h1>Wealthfolio is ready</h1><p>Your password hash is saved on the laptop. The private dashboard is starting.</p><a href="/wealth/">Open Wealthfolio</a>',
            extra=[self.set_cookie(SETUP_COOKIE, '', 0), ('Refresh','5;url=/wealth/')])
        threading.Thread(target=self.server.shutdown, daemon=True).start()

def main():
    server = HTTPServer(('127.0.0.1',8930),Handler)
    server.state = gate.State({'origins':ORIGINS})
    server.config_path = gate.CONFIG
    try: server.serve_forever()
    finally: server.server_close()

if __name__ == '__main__': main()
