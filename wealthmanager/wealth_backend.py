"""Launch only the configured existing backend/database under the user service."""
import json
import os
from pathlib import Path

BASE = Path(__file__).resolve().parent


def configuration(path=BASE / 'data/web-portal/backend.json'):
    config = json.loads(path.read_text())
    if not Path(config['env']['WF_DB_PATH']).is_file():
        raise RuntimeError('The existing Wealthfolio database is unavailable; refusing to create a new one.')
    if not Path(config['exe']).is_file():
        raise RuntimeError('The configured Wealthfolio backend executable is unavailable.')
    return config


def main():
    config = configuration()
    os.chdir(config['cwd'])
    os.execve(config['exe'], [config['exe']],
              {**os.environ, **config['env'], 'WF_LISTEN_ADDR': '127.0.0.1:8080'})


if __name__ == '__main__':
    main()
