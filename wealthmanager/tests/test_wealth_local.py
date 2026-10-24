import http.client
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import io
from pathlib import Path
import tempfile
import threading
import unittest
from unittest.mock import patch
from urllib.parse import urlencode

import wealth_local as local
import wealth_portal as portal


class Backend(BaseHTTPRequestHandler):
    def do_GET(self):
        body = b'{"saved":true}'
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_POST(self):
        body = self.rfile.read(int(self.headers['Content-Length']))
        self.send_response(200)
        self.send_header('Content-Length', str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *args):
        pass


class LocalTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.backend = ThreadingHTTPServer(('127.0.0.1', 0), Backend)
        self.server = ThreadingHTTPServer(('127.0.0.1', 0), local.LocalHandler)
        host = '127.0.0.1:' + str(self.server.server_port)
        self.origin = 'http://' + host
        self.server.allowed_hosts = {host}
        self.server.backend_port = self.backend.server_port
        self.now = 1
        self.server.state = portal.State({'password_hash': portal.PH.hash('fixture-secret')}, lambda: self.now)
        self.server.dist = Path(self.directory.name)
        (self.server.dist / 'index.html').write_text('<h1>Same production website</h1>')
        for server in (self.server, self.backend):
            threading.Thread(target=server.serve_forever, daemon=True).start()

    def tearDown(self):
        for server in (self.server, self.backend):
            server.shutdown()
            server.server_close()
        self.directory.cleanup()

    def request(self, path, method='GET', body=None, headers=None):
        connection = http.client.HTTPConnection(*self.server.server_address, timeout=5)
        connection.request(method, path, body, headers or {})
        response = connection.getresponse()
        result = response.status, dict(response.getheaders()), response.read()
        connection.close()
        return result

    def login(self):
        status, headers, page = self.request('/wealth/')
        self.assertEqual(status, 200)
        self.assertNotIn(b'fixture-secret', page)
        self.assertNotIn(b'$argon2', page)
        token = next(iter(self.server.state.challenges))
        status, headers, _ = self.request('/wealth/login', 'POST', urlencode({'csrf':token, 'password':'fixture-secret'}),
            {'Cookie':'wealth_local_challenge='+token, 'Origin':self.origin})
        self.assertEqual(status, 303)
        # There are two Set-Cookie headers; find the opaque token in server state.
        opaque = next(iter(self.server.state.sessions))
        return 'wealth_local_session='+opaque

    def test_local_login_works_without_the_phone_and_requires_password(self):
        with patch.object(portal, 'primary_user', side_effect=AssertionError('Local access must not contact the phone')):
            self.assertEqual(self.request('/')[0:2], (303, unittest.mock.ANY))
            self.assertEqual(self.request('/wealth/api/v1/test')[0], 401)
            cookie = self.login()
            status, headers, body = self.request('/wealth/app/', headers={'Cookie':cookie, 'Sec-Fetch-Dest':'document'})
            self.assertEqual((status, body), (200, b'<h1>Same production website</h1>'))
            self.assertIn('no-store', headers['Cache-Control'])

    def test_origin_host_expiry_and_private_paths(self):
        self.assertEqual(self.request('/wealth/', headers={'Host':'attacker.example'})[0], 403)
        self.assertEqual(self.request('/wealth/login', 'POST', 'password=x', {'Origin':'http://evil.example'})[0], 403)
        cookie = self.login()
        auth = {'Cookie':cookie}
        self.assertEqual(self.request('/wealth/api/v1/test', 'POST', 'x', auth)[0], 403)
        self.assertEqual(self.request('/wealth/api/v1/test', headers={**auth, 'Origin':'http://evil.example'})[0], 403)
        self.assertEqual(self.request('/wealth/app/%2e%2e/auth.json', headers=auth)[0], 404)
        self.now += 901
        self.assertEqual(self.request('/wealth/api/v1/test', headers=auth)[0], 401)

    def test_streaming_proxy_and_upload_limits(self):
        cookie = self.login()
        body = b'fictional upload\n' * 32768
        auth = {'Cookie':cookie, 'Origin':self.origin, 'Content-Type':'application/octet-stream'}
        status, headers, received = self.request('/wealth/api/v1/upload?test=true', 'POST', body, auth)
        self.assertEqual((status, received), (200, body))
        self.assertIn('no-store', headers['Cache-Control'])
        self.assertEqual(self.request('/wealth/api/v1/upload', 'POST', None,
            {**auth, 'Content-Length':str(local.MAX_BODY+1)})[0], 413)
        limited = local.LimitedBody(io.BytesIO(b'body-and-extra'), 4)
        self.assertEqual(limited.read(1024), b'body')
        self.assertEqual(limited.read(1024), b'')

    def test_status_hides_processing_until_unlock(self):
        with patch('wealth_status.snapshot', return_value={'backend':'ready'}) as snapshot:
            self.assertEqual(self.request('/wealth/status')[0], 200)
            snapshot.assert_called_with(include_processing=False)
            cookie = self.login()
            self.request('/wealth/status', headers={'Cookie':cookie})
            snapshot.assert_called_with(include_processing=True)


if __name__ == '__main__':
    unittest.main()
