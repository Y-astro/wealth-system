# Wealth system: local statement processing

Upload PDF, Excel, CSV, Markdown, text, or an exported report from the Statements
page. Originals, converted Markdown, reports, and review edits persist in the
laptop's statement library across browser sessions. Duplicate file contents reuse
saved conversion/results. Retry a failed item from its saved original.

PDF conversion uses the installed MarkItDown command. Native table/CSV parsers run
before local AI; unsupported bank layouts use the laptop's local model. Large
statements use bounded chunks and resumable extraction checkpoints. The upload
limit is 50 MB. Model context is bounded and managed models are unloaded after
processing to release RAM.

Bank cash flows and investment/NSDL snapshots are separate. The combined bank
Sankey supports filters and category drill-down. Credit-card filenames containing
a CC token are treated as card statements: refunds and fuel surcharge reversals
reduce expenses, repayments are transfers, and the report shows payment coverage.
Statement balance history is available on the dashboard.

## Website controls

```sh
wf-start           # Backend and access through the Redmi
wf-start --local   # Also serve the same production website at localhost:1420
wf-start--local    # Equivalent shortcut
wf-stop            # Stop the backend and both website entry points
```

The Redmi relays authenticated HTTPS traffic. All statement data, AI processing,
and the separate password hash stay on the laptop. Both admin and family portal
accounts can open Wealthfolio, and must enter its separate password. The optional
loopback website also requires that password.

See [WEB_PORTAL.md](WEB_PORTAL.md) for installation, certificates, service status,
shared production builds, the router reservation, and deployment instructions.

## Tests and storage

```sh
cd /home/astro/wealth-system/wealthmanager
PYTHONPATH=.:tests python3 -m unittest discover -v -s tests
node portal-bootstrap.test.mjs
```

Tests use fictional statements and temporary databases. Private originals,
Markdown, reports, checkpoints, password hashes and backend configuration live
under `wealthmanager/data/` and are excluded from Git. GitHub backs up code only;
restoring the repository does not restore the financial library.
