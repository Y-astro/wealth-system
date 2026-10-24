import contextlib
import io
import json
from pathlib import Path
import sqlite3
import tempfile
import unittest
from unittest.mock import Mock, patch

import wealth_backend
import wealth_control
import wealth_status


class RuntimeTests(unittest.TestCase):
    def test_existing_database_is_required_and_configuration_preserved(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            executable, database = root/'backend', root/'existing.sqlite'
            executable.touch()
            config = {'exe':str(executable), 'cwd':str(root), 'env':{'WF_DB_PATH':str(database), 'WF_TEST_VALUE':'preserve'}}
            path = root/'backend.json'
            path.write_text(json.dumps(config))
            with self.assertRaises(RuntimeError):
                wealth_backend.configuration(path)
            self.assertFalse(database.exists())
            database.touch()
            self.assertEqual(wealth_backend.configuration(path), config)
            with patch.object(wealth_backend, 'configuration', return_value=config), patch.object(wealth_backend.os, 'chdir'), patch.object(wealth_backend.os, 'execve') as execute:
                wealth_backend.main()
                self.assertEqual(execute.call_args.args[2]['WF_DB_PATH'], str(database))
                self.assertEqual(execute.call_args.args[2]['WF_TEST_VALUE'], 'preserve')
                self.assertEqual(execute.call_args.args[2]['WF_LISTEN_ADDR'], '127.0.0.1:8080')

    def test_commands_select_local_and_stop_the_whole_stack(self):
        opener = Mock()
        opener.open.return_value.__enter__ = Mock(return_value=Mock(status=200))
        opener.open.return_value.__exit__ = Mock(return_value=False)
        with patch.object(wealth_control, 'listening', return_value=True), patch.object(wealth_control, 'active', return_value=True), patch.object(wealth_control.urllib.request, 'build_opener', return_value=opener), patch.object(wealth_control.subprocess, 'run') as run, contextlib.redirect_stdout(io.StringIO()):
            wealth_control.run('start')
            self.assertEqual(run.call_args_list[0].args[0], ['systemctl','--user','stop','wealth-local.service'])
            self.assertEqual(run.call_args_list[1].args[0], ['systemctl','--user','start','wealth-portal.service'])
            run.reset_mock()
            wealth_control.run('start', local=True)
            self.assertEqual(run.call_args.args[0], ['systemctl','--user','start','wealth-portal.service','wealth-local.service'])
        with patch.object(wealth_control, 'listening', return_value=False), patch.object(wealth_control.subprocess, 'run') as run, contextlib.redirect_stdout(io.StringIO()):
            wealth_control.run('stop')
            self.assertEqual(run.call_args.args[0], ['systemctl','--user','stop','wealth-local.service','wealth-portal.service'])

    def test_processing_summary_contains_only_counts(self):
        with tempfile.TemporaryDirectory() as directory:
            db = sqlite3.connect(Path(directory)/'library.sqlite3')
            db.execute('CREATE TABLE uploads (status TEXT, archived INTEGER, filename TEXT)')
            db.executemany('INSERT INTO uploads VALUES (?,0,?)', [('ready','private.pdf'),('processing','private2.pdf'),('failed','private3.pdf')])
            db.commit()
            db.close()
            with patch.dict(wealth_status.os.environ, {'WEALTH_STATEMENT_LIBRARY':directory}), patch.object(wealth_status.socket, 'create_connection', return_value=contextlib.nullcontext()):
                self.assertEqual(wealth_status.snapshot(), {'backend':'ready'})
                result = wealth_status.snapshot(True)
                self.assertEqual(result['processing'], {'ready':1,'processing':1,'failed':1,'queued':0})
                self.assertNotIn('private', json.dumps(result))
            with patch.object(wealth_status.socket, 'create_connection', side_effect=OSError):
                self.assertEqual(wealth_status.snapshot(True), {'backend':'unavailable'})


if __name__ == '__main__':
    unittest.main()
