"""Optional loopback website: same production files and password as the relay."""
import http.client
import json
from urllib.parse import urlsplit

from wealth_portal import CONFIG, DIST, HEADERS, Handler, State, ThreadingHTTPServer

MAX_BODY = 52 * 1024 * 1024
HOP_HEADERS = {'connection', 'keep-alive', 'proxy-authenticate', 'proxy-authorization',
               'te', 'trailer', 'transfer-encoding', 'upgrade'}


class LimitedBody:
    def __init__(self, stream, length):
        self.stream, self.remaining = stream, length

    def read(self, size=-1):
        if self.remaining <= 0:
            return b''
        data = self.stream.read(min(self.remaining, 65536 if size < 0 else size))
        self.remaining -= len(data)
        return data


class LocalHandler(Handler):
    session_cookie = 'wealth_local_session'
    challenge_cookie = 'wealth_local_challenge'
    secure_cookie = False  # HTTP is bound exclusively to laptop loopback.

    def identity(self):
        if self.headers.get('Host') not in self.server.allowed_hosts:
            self.send(403, 'Use localhost on this laptop.'); return None
        if self.headers.get('Origin') and not self.same_origin():
            self.send(403, 'Cross-origin request rejected.'); return None
        return 'local', self.server.state.binding('local-loopback')

    def same_origin(self):
        return (self.headers.get('Origin') == 'http://' + self.headers.get('Host', '') and
                self.headers.get('Sec-Fetch-Site', 'same-origin') == 'same-origin')

    def proxy_api(self):
        identity = self.identity()
        if not identity:
            return
        if not self.session(identity[1]):
            self.send(401, '{"message":"Unlock Wealthfolio first."}', 'application/json'); return
        if self.command not in ('GET', 'HEAD') and not self.same_origin():
            self.send(403, 'Cross-site request rejected.'); return
        if self.headers.get('Transfer-Encoding'):
            self.send(400, 'Chunked request bodies are not supported.'); return
        try:
            length = int(self.headers.get('Content-Length', '0'))
        except ValueError:
            self.send(400, 'Invalid request length.'); return
        if not 0 <= length <= MAX_BODY:
            self.send(413, 'Choose a statement up to 50 MB.'); return
        headers = {key: self.headers[key] for key in ('Content-Type', 'Accept', 'Content-Length') if key in self.headers}
        connection = http.client.HTTPConnection('127.0.0.1', self.server.backend_port, timeout=300)
        try:
            connection.request(self.command, self.path[len('/wealth'):],
                               body=LimitedBody(self.rfile, length) if length else None, headers=headers)
            response = connection.getresponse()
        except (OSError, http.client.HTTPException):
            connection.close()
            self.send(503, '{"message":"The laptop backend is starting or unavailable. Please retry."}', 'application/json'); return
        try:
            self.send_response(response.status)
            for key, value in HEADERS.items():
                self.send_header(key, value)
            for key, value in response.getheaders():
                if key.lower() not in HOP_HEADERS | {'set-cookie', 'cache-control', 'pragma', 'access-control-allow-origin', 'access-control-allow-credentials'}:
                    self.send_header(key, value)
            self.send_header('Connection', 'close')
            self.end_headers()
            self.close_connection = True
            if self.command != 'HEAD':
                while chunk := response.read1(65536):
                    self.wfile.write(chunk)
        except (OSError, http.client.HTTPException):
            self.close_connection = True
        finally:
            connection.close()

    def do_GET(self):
        if urlsplit(self.path).path.startswith('/wealth/api/v1/'):
            self.proxy_api(); return
        if self.path == '/':
            if self.identity():
                self.send(303, extra=[('Location', '/wealth/')])
            return
        super().do_GET()

    def do_POST(self):
        if urlsplit(self.path).path.startswith('/wealth/api/v1/'):
            self.proxy_api(); return
        super().do_POST()

    def write_api(self):
        if urlsplit(self.path).path.startswith('/wealth/api/v1/'):
            self.proxy_api()
        else:
            self.send(405, 'Method not allowed.')

    do_PUT = do_PATCH = do_DELETE = do_OPTIONS = write_api


def main():
    config = json.loads(CONFIG.read_text())
    if not config['password_hash'].startswith('$argon2id$'):
        raise SystemExit('Invalid private portal configuration.')
    server = ThreadingHTTPServer(('127.0.0.1', 1420), LocalHandler)
    server.daemon_threads = True
    server.allowed_hosts = {'localhost:1420', '127.0.0.1:1420'}
    server.backend_port = 8080
    server.state, server.dist = State(config), DIST
    server.serve_forever()


if __name__ == '__main__':
    main()
