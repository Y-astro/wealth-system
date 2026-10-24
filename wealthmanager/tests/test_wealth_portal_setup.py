"""Isolated tests with disposable credentials; no live account changes."""
import json
import unittest
from unittest.mock import patch
from urllib.parse import urlencode
import wealth_portal as gate
import wealth_portal_setup as setup
import test_wealth_portal as fixtures

class SetupTests(unittest.TestCase):
    tearDown = fixtures.GateTests.tearDown
    request = fixtures.GateTests.request
    def setUp(self):
        fixtures.GateTests.setUp(self)
        self.server.RequestHandlerClass = setup.Handler
        self.server.state.config['origins'] = ['https://home:8444']
        self.server.config_path = self.server.dist/'private-auth.json'
        self.primary.stop()
        self.primary = patch.object(gate, 'primary_user', lambda value, admin=False: ('admin', value.split(';')[0]) if value else None)
        self.primary.start()

    def test_setup_form_has_no_credential(self):
        status, headers, body = self.request('/wealth/')
        self.assertEqual(status, 200)
        self.assertIn(('Referrer-Policy', 'same-origin'), headers)
        self.assertIn('Finish Wealthfolio setup', body)
        self.assertIn('name="confirm"', body)
        self.assertNotIn('test-secret', body)
        self.assertEqual(self.request('/wealth/setup', 'POST', 'password=x', {'Origin':'https://evil'})[0], 403)

    def test_administrator_required(self):
        with patch.object(gate, 'primary_user', return_value=None):
            self.assertEqual(self.request('/wealth/')[0], 403)

    def test_disposable_setup_hash(self):
        self.request('/wealth/')
        token = next(iter(self.server.state.challenges))
        headers = {'Cookie':'session_token=primary-one; '+setup.SETUP_COOKIE+'='+token,'Origin':'https://home:8444'}
        fields = urlencode({'csrf':token,'password':'disposable-test-password','confirm':'disposable-test-password'})
        with patch.object(self.server, 'shutdown'):
            status, _, _ = self.request('/wealth/setup','POST',fields,headers)
        self.assertEqual(status, 200)
        contents = self.server.config_path.read_text()
        self.assertNotIn('disposable-test-password', contents)
        self.assertTrue(gate.PH.verify(json.loads(contents)['password_hash'],'disposable-test-password'))
        self.assertEqual(self.server.config_path.stat().st_mode & 0o777, 0o600)
