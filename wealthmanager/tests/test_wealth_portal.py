import http.client
from unittest.mock import Mock
import tempfile
import threading
import unittest
from pathlib import Path
from urllib.parse import urlencode
from unittest.mock import patch
import wealth_portal as portal


class GateTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        Path(self.tmp.name, 'index.html').write_text('<h1>Private dashboard</h1>')
        self.server = portal.ThreadingHTTPServer(('127.0.0.1', 0), portal.Handler)
        self.server.dist = Path(self.tmp.name)
        self.now = 10
        self.server.state = portal.State({'origins': ['https://home:8444'],
            'password_hash': portal.PH.hash('test-secret')}, lambda: self.now)
        self.primary = patch.object(portal, 'primary_user', lambda value: ('family', value.split(';')[0]) if value else None)
        self.primary.start()
        threading.Thread(target=self.server.serve_forever, daemon=True).start()
        self.headers = {'X-Forwarded-Proto': 'https', 'Cookie': 'session_token=primary-one'}

    def tearDown(self):
        self.server.shutdown()
        self.server.server_close()
        self.primary.stop()
        self.tmp.cleanup()

    def request(self, path, method='GET', body=None, headers=None):
        connection = http.client.HTTPConnection(*self.server.server_address, timeout=5)
        connection.request(method, path, body, {**self.headers, **(headers or {})})
        response = connection.getresponse()
        result = response.status, response.getheaders(), response.read().decode()
        connection.close()
        return result

    def login(self):
        _, headers, body = self.request('/wealth/')
        token = next(iter(self.server.state.challenges))
        self.assertNotIn('test-secret', body)
        self.assertNotIn('$argon2', body)
        status, headers, _ = self.request('/wealth/login', 'POST', urlencode({'csrf': token, 'password': 'test-secret'}),
            {'Cookie': 'session_token=primary-one; '+portal.CHALLENGE+'='+token, 'Origin': 'https://home:8444'})
        self.assertEqual(status, 303)
        cookie = next(value for key, value in headers if key == 'Set-Cookie' and value.startswith(portal.COOKIE+'='))
        self.assertIn('Secure; HttpOnly; SameSite=Strict', cookie)
        return cookie.split(';')[0]

    def test_primary_and_https_required(self):
        self.assertEqual(self.request('/wealth/', headers={'Cookie': '', 'X-Forwarded-User': 'admin'})[0], 401)
        self.assertEqual(self.request('/wealth/', headers={'X-Forwarded-Proto': 'http'})[0], 403)
        self.assertEqual(self.request('/wealth/_check')[0], 401)

    def test_login_document_binding_expiry(self):
        cookie = self.login()
        auth = {'Cookie': 'session_token=primary-one; '+cookie}
        self.assertEqual(self.request('/wealth/_check', headers=auth)[0], 200)
        self.assertEqual(self.request('/wealth/_check', headers={'Cookie': 'session_token=primary-two; '+cookie})[0], 401)
        self.assertEqual(self.request('/wealth/app/', headers={**auth, 'Sec-Fetch-Dest': 'document'})[2], '<h1>Private dashboard</h1>')
        self.assertIn('Unlock Wealthfolio', self.request('/wealth/app/', headers={**auth, 'Sec-Fetch-Dest': 'document'})[2])
        self.assertEqual(self.request('/wealth/_check', headers=auth)[0], 401)
        cookie = self.login()
        self.now += 901
        self.assertEqual(self.request('/wealth/_check', headers={'Cookie': 'session_token=primary-one; '+cookie})[0], 401)

    def test_origin_csrf_and_private_files(self):
        self.assertEqual(self.request('/wealth/login', 'POST', 'password=x', {'Origin': 'https://home:8443'})[0], 403)
        self.assertEqual(self.request('/wealth/login', 'POST', 'password=x', {'Origin': 'https://home:8444'})[0], 403)
        cookie = self.login()
        auth = {'Cookie': 'session_token=primary-one; '+cookie}
        self.assertEqual(self.request('/wealth/_check', headers={**auth, 'Origin': 'https://home:8443'})[0], 403)
        self.assertEqual(self.request('/wealth/app/%2e%2e/auth.json', headers=auth)[0], 404)
        self.assertEqual(self.request('/wealth/_check', headers={**auth, 'X-Forwarded-Method': 'POST'})[0], 403)

    def test_rate_limit(self):
        state = self.server.state
        with state.lock:
            for _ in range(5): state.failed('family', 'client')
            self.assertFalse(state.allowed('family', 'client'))
            self.assertTrue(state.allowed('admin', 'other'))
            self.now += 901
            self.assertTrue(state.allowed('family', 'client'))

    def test_only_website_assets_are_cacheable(self):
        assets = self.server.dist/'assets'
        assets.mkdir()
        (assets/'app-hash1234.js').write_text('console.log("website code")')
        cookie = self.login()
        auth = {'Cookie':'session_token=primary-one; '+cookie}
        status, headers, _ = self.request('/wealth/app/assets/app-hash1234.js',headers=auth)
        self.assertEqual(status,200)
        self.assertIn(('Cache-Control','private, max-age=31536000, immutable'),headers)
        self.assertNotIn('Pragma',dict(headers))
        status, headers, _ = self.request('/wealth/app/',headers=auth)
        self.assertEqual(status,200)
        self.assertIn('no-store',dict(headers)['Cache-Control'])
        self.now += 901
        status, headers, _ = self.request('/wealth/app/assets/app-hash1234.js',headers=auth)
        self.assertEqual(status,303)
        self.assertIn('no-store',dict(headers)['Cache-Control'])


class PrimaryVerificationTests(unittest.TestCase):
    def test_burst_cache_is_short_bounded_and_role_specific(self):
        portal.PRIMARY_CACHE.clear()
        response = Mock(status=200)
        response.getheader.return_value = 'fixture-user'
        connection = Mock()
        connection.getresponse.return_value = response
        with patch.object(portal.http.client,'HTTPConnection',return_value=connection) as factory:
            with patch.object(portal.time,'monotonic',return_value=10):
                self.assertEqual(portal.primary_user('session_token=fixture')[0],'fixture-user')
                portal.primary_user('session_token=fixture')
                self.assertEqual(factory.call_count,1)
                portal.primary_user('session_token=fixture',admin=True)
                self.assertEqual(factory.call_count,2)
            with patch.object(portal.time,'monotonic',return_value=14):
                portal.primary_user('session_token=fixture')
                self.assertEqual(factory.call_count,3)
                for i in range(260): portal.primary_user('session_token=fixture'+str(i))
                self.assertLessEqual(len(portal.PRIMARY_CACHE),256)
                response.status = 401
                self.assertIsNone(portal.primary_user('session_token=invalid'))
                self.assertIsNone(portal.primary_user('session_token=invalid'))
        portal.PRIMARY_CACHE.clear()


if __name__ == '__main__': unittest.main()
