# Wealth System

A personal wealth and statement analysis system built on a customized
[Wealthfolio](wealthfolio/) application. It combines portfolio tracking and Indian
wealth-management workflows with a persistent statement library, local document
processing, and private browser access from a laptop or the Redmi home portal.

The application lives in `wealthfolio/`; the Python processing and portal tools
live in `wealthmanager/`. This README describes this fork and its current workflows.
The [Wealthfolio README](wealthfolio/README.md) covers the upstream project.

## What it does

- **Statement library:** upload PDF, Excel (`.xlsx`, `.xls`), CSV, Markdown, text,
  or an exported JSON report. Originals, conversions, reports, and review edits
  persist across browser sessions. Identical file contents reuse saved work;
  failed uploads can be retried from the library. The upload limit is 50 MB.
- **Local extraction:** PDFs use MarkItDown; spreadsheet and bank-table parsers
  run before AI. Unsupported bank layouts can use a local LM Studio model, with
  bounded chunks, extraction checkpoints, and on-demand model loading.
- **Cash-flow views:** combined bank Sankey charts with filters and category
  drill-down, statement balance history, and transaction source mappings.
- **Credit-card reconciliation:** expenses, refunds, surcharge reversals,
  repayments, and payment coverage. A `CC` token in the filename identifies a
  credit-card statement.
- **Investment snapshots:** NSDL and investment statement holdings remain
  snapshots, separate from bank cash flows.
- **Wealth tracking:** the fork retains Indian instrument presets and valuation
  logic for deposits, PPF, EPF, NPS, insurance, and loans, alongside portfolio,
  tax-calculation, and native importer modules.
- **Private portal:** the Redmi relays HTTPS traffic to the laptop over SSH.
  A separate Wealthfolio password, manual lock, inactivity lock, and session
  expiry protect access. The optional localhost website serves the same
  production build.

Statement conversion and analysis run on the laptop. Market-data integrations and
other optional Wealthfolio services can still require network access.

## Everyday use on the configured laptop

The following commands are installed by the portal installer:

```sh
wf-start           # Start the backend and access through the Redmi
wf-start --local   # Also serve the production website at localhost:1420
wf-start--local    # Shortcut for the preceding command
wf-stop            # Stop the backend, local website, and Redmi relay
```

Open Wealthfolio from the Redmi portal, or visit <http://localhost:1420/> after
starting with `--local`. Both entry points require the Wealthfolio password.
The laptop must remain awake; the Redmi does not keep a financial copy to serve
when the laptop is unavailable.

In **Statements**, upload a file, wait for processing, then review its chart,
source mappings, and reconciliation warnings. Correct report rows as needed and
save the review. Use the library's retry action after resolving a conversion or
model error.

See [statement processing instructions](wealthmanager/README_INSTRUCTIONS.md)
and the [private portal guide](wealthmanager/WEB_PORTAL.md) for more detail.

## Development setup from a fresh clone

These steps start the web development application. The existing Redmi deployment
also needs private configuration, SSH access, certificates, and a database; those
are not supplied by cloning this repository.

### Requirements

| Component | Purpose |
| --- | --- |
| Node.js 24 and pnpm 11.10.0 | Versions specified by `.node-version` and `package.json` |
| Rust/Cargo and a native build toolchain | Axum backend and Wealthfolio crates |
| `cargo-watch` | Backend reloads for `pnpm dev:web` |
| Python 3.10+ with venv/pip | Statement processing and portal tools |
| MarkItDown with PDF support | PDF conversion |
| pandas, openpyxl, xlrd | Excel conversion |
| argon2-cffi | Portal password hashing and authentication |
| LM Studio and its `lms` CLI, optional | Local AI fallback for unsupported bank layouts |

The managed portal additionally uses Linux user services, OpenSSH, and `ss`.
Desktop/Tauri development has additional platform dependencies; see
[CONTRIBUTING.md](wealthfolio/CONTRIBUTING.md).

### Install dependencies

```sh
git clone https://github.com/Y-astro/wealth-system.git
cd wealth-system

python3 -m venv wealthmanager/.venv
. wealthmanager/.venv/bin/activate
python -m pip install -r wealthmanager/requirements-portal.txt
python -m pip install 'markitdown[pdf]' pandas openpyxl xlrd
python -m pip install pymupdf  # Generates fictional PDF fixtures in the tests

cd wealthfolio
pnpm install --frozen-lockfile
pnpm setup:env
cargo install cargo-watch --locked
```

Keep the virtual environment active when launching development mode so the
backend's Python workers and conversion tools can find their dependencies.

### Configure and run web development

Edit `wealthfolio/.env.web` before starting. Keep the example's loopback binding
and explicit CORS origin. Set `WF_SECRET_KEY` to a newly generated 32-byte key:

```sh
openssl rand -base64 32
```

Paste the result into `WF_SECRET_KEY`. To enable the backend's development login,
generate a password hash locally and paste it into `WF_AUTH_PASSWORD_HASH`:

```sh
python -c 'from argon2 import PasswordHasher; from getpass import getpass; print(PasswordHasher().hash(getpass("Development password: ")))'
```

Use a separate development database through `WF_DB_PATH`; an absolute path makes
its location unambiguous. Keep `.env.web` private and preserve the secret key when
reusing a database. The development backend login is configured separately from
the managed portal's password gate.

From `wealthfolio/`, with the virtual environment active:

```sh
pnpm dev:web
```

Open <http://localhost:1420/>. The frontend proxies API requests to the local Axum
backend. Stop existing services using these ports before starting development.

Do not use `wealthfolio/start_server.sh` to start an existing installation: it
deletes `apps/server/db/web-dev.db*` before launching the server.

### Optional local AI

Start the LM Studio server with an installed chat model. The default endpoint is
`http://127.0.0.1:1234`. Supported tables and investment snapshots can be parsed
without a model; unknown bank layouts use the AI fallback.

Useful overrides, set in the backend environment:

| Variable | Purpose |
| --- | --- |
| `LM_STUDIO_BASE` | LM Studio server URL |
| `LM_STUDIO_MODEL` | Explicit model or loaded-instance selection |
| `LM_STUDIO_API_KEY` | Token if the local model server requires authentication |
| `LM_STUDIO_CLI` | Path to the `lms` executable for on-demand loading |
| `WEALTH_AI_CONTEXT` | Managed model context length, bounded to 4096–8192 tokens |
| `WEALTH_PYTHON` | Python interpreter used by the Rust statement API |
| `WEALTH_STATEMENT_ANALYZER` | Absolute path to `wealthmanager/analyze_statement.py` |
| `WEALTH_STATEMENT_LIBRARY` | Override the default private statement storage directory |
| `WEALTH_MARKITDOWN` | Path to the MarkItDown executable |

The processor unloads the model instance it manages after extraction; a model
already loaded for another application has a separate lifecycle.

## Command-line statement analysis

From the repository root, generate a report from a supported table without AI:

```sh
python wealthmanager/analyze_statement.py \
  wealthmanager/tests/fixtures/bank_statement.md --no-ai \
  --output /tmp/bank-statement.report.json
```

Process one or more documents through conversion and analysis:

```sh
python wealthmanager/process_all.py /path/to/statement.pdf
python wealthmanager/process_all.py /path/to/statement.csv --no-ai
```

Batch reports are written to `wealthmanager/reports/` by default; use
`--output-dir` to change this. These commands generate standalone reports rather
than adding uploads to the persistent browser library. Normal analysis keeps
investment holdings as snapshots and does not turn them into synthetic trades.

Encrypted PDFs must be unlocked before upload. Scanned PDFs need OCR before
analysis. Review excluded rows and balance differences against the original
statement before relying on a report.

## Managed portal maintenance

On the already configured laptop, build the shared production website from the
repository root:

```sh
python3 wealthmanager/build_web_portal.py
systemctl --user restart wealth-portal.service
systemctl --user status wealth-portal.service
```

The build explicitly selects the web target and `/wealth/app/` base, writes to
`wealthfolio/apps/frontend/dist-portal/`, and retains the preceding bundle for
rollback. Use this build script for the portal.

To install or update the user services and `wf-*` commands on that configured
machine:

```sh
python3 wealthmanager/install_web_portal.py
```

The installer expects `wealthmanager/data/web-portal/backend.json` to point to an
existing backend executable, working directory, environment, and database. It
checks for active statement jobs before adopting the backend. Services use
`/usr/bin/python3`, so that interpreter must have the required Python packages;
activating a development venv does not change the service interpreter.

The SSH destination, trusted origins, and phone-side proxy configuration are
specific to the current installation. This installer is not a complete fresh
Redmi provisioning tool. Follow [WEB_PORTAL.md](wealthmanager/WEB_PORTAL.md) for
the deployed addresses, certificate trust, authentication, and maintenance details.

## Storage and backups

| Location | Contents |
| --- | --- |
| `wealthmanager/data/statements/library.sqlite3` | Uploaded originals, converted text, saved reports, review edits, and processing status |
| `wealthmanager/data/statements/markdown/` | Saved PDF-to-Markdown conversions |
| `wealthmanager/data/statements/` | Also contains extraction checkpoints and worker lock files |
| `wealthmanager/data/web-portal/` | Private backend configuration, portal password hash, and deployment backups |
| `WF_DB_PATH` | Wealthfolio's separate portfolio database |
| `wealthmanager/reports/` | Generated command-line reports |

The statement directory above is the default; `WEALTH_STATEMENT_LIBRARY` can
override it. Private runtime data, environment files, databases, and generated
portal builds are excluded from Git. **GitHub backs up code, not financial data.**
Back up the private library, the configured Wealthfolio database, and the keys and
configuration needed to restore them separately. Take SQLite backups with the
writers stopped or with a SQLite-aware backup tool so WAL data is included.

## Verification

Python tests use fictional statements and temporary databases. From the repository
root, with the Python dependencies available:

```sh
(cd wealthmanager && PYTHONPATH=.:tests python3 -m unittest discover -v -s tests)
node wealthmanager/portal-bootstrap.test.mjs
```

Run the statement UI tests from `wealthfolio/`:

```sh
pnpm --filter frontend exec vitest run src/features/statements
```

Broader development checks are available through `pnpm type-check`, `pnpm lint`,
`pnpm --filter frontend exec vitest run`, and `cargo test` in `wealthfolio/`.

## Repository map

```text
wealthfolio/
  apps/frontend/       React/Vite interface and statement visualizations
  apps/server/         Axum API and Python worker bridge
  apps/tauri/          Desktop application
  crates/              Portfolio, storage, market data, tax, and importer logic
  packages/            Shared UI and addon tooling
wealthmanager/
  ingest.py            Document-to-text/Markdown conversion
  statement_parser.py  Deterministic statement parsing and reconciliation
  statement_library.py Persistent uploads and processing workers
  local_ai.py          Local model discovery and extraction
  ai_chunks.py         Bounded extraction chunks and checkpoints
  wealth_*.py          Portal authentication, local access, and process lifecycle
  tests/               Python tests and fictional fixtures
.planning/             Implementation notes and verification history
```

## Attribution and license

The customized Wealthfolio application retains its upstream attribution and
[AGPL-3.0 license](wealthfolio/LICENSE). See its
[trademark policy](wealthfolio/TRADEMARKS.md) as well. There is no separate root
license file for the additional `wealthmanager/` code.
