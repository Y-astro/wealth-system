"""Small connection/processing summary; never returns filenames or report data."""
import os
from pathlib import Path
import socket
import sqlite3


def snapshot(include_processing=False):
    try:
        with socket.create_connection(('127.0.0.1', 8080), timeout=0.5):
            result = {'backend': 'ready'}
    except OSError:
        return {'backend': 'unavailable'}
    if not include_processing:
        return result
    root = Path(os.environ.get('WEALTH_STATEMENT_LIBRARY', str(Path(__file__).resolve().parent / 'data/statements')))
    path = (root / 'library.sqlite3').resolve()
    counts = {'queued': 0, 'processing': 0, 'failed': 0, 'ready': 0}
    if path.is_file():
        try:
            with sqlite3.connect(path.as_uri() + '?mode=ro', uri=True, timeout=1) as db:
                for status, count in db.execute('SELECT status, COUNT(*) FROM uploads WHERE archived=0 GROUP BY status'):
                    if status in counts:
                        counts[status] = count
        except sqlite3.Error:
            result['processing'] = 'unknown'
            return result
    result['processing'] = counts
    return result
