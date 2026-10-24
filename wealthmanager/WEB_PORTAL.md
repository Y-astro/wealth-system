# Private Wealthfolio portal

The Redmi serves the home portal and streams Wealthfolio through private SSH
forwards. Statements, converted Markdown, reports, the database, AI jobs and the
separate Wealthfolio password hash remain on the Linux laptop.

- Home Wi-Fi portal: `https://192.168.1.10:8443/`. The router reserves `.10` for the Redmi.
- Wealthfolio: `https://192.168.1.10:8444/wealth/`.
- The same ports work at the Redmi's Tailscale IP `100.75.159.53`.
- Both administrator and family portal pages contain the Wealthfolio entry.
- Every fresh dashboard document requires the separate password. A visible Lock
  button, 15-minute user inactivity lock and a four-hour absolute session limit
  protect an open dashboard. The setup endpoint stops accepting passwords after
  the initial configuration exists.

The laptop must be awake and connected. The user service is enabled at boot with
user lingering, reconnects SSH automatically, and owns the backend process using
the existing database. AI conversion also needs the laptop's existing local model runtime.
When the laptop is unavailable the Redmi returns an unavailable message, with no
saved financial copy to show.

## Certificate installation

The public certificate on the laptop is:
`/home/astro/.local/share/wealth-portal/RedmiHomeServerCA.crt`.
Family devices can download the same public certificate from
`http://192.168.1.10:8000/wealth-ca.crt` (or `.8`). Verify its SHA-256 fingerprint:

`68cce8da1ab34195fdeb589345d3eaaf12029065bffb19eff8434631d033ec1c`

On CachyOS/Arch:

```sh
sudo trust anchor --store /home/astro/.local/share/wealth-portal/RedmiHomeServerCA.crt
sudo update-ca-trust
```

Restart the browser. Other family devices need the equivalent one-time trusted
CA installation. The certificate authority's private key stays on the Redmi.

## Maintenance

```sh
wf-start           # Backend and access through the Redmi; localhost stays off
wf-start --local   # Also enable http://localhost:1420/
wf-start--local    # Equivalent shortcut
wf-stop            # Stop the backend, optional local website and Redmi relay
```

Both website entry points serve the same `dist-portal` production build. No Vite
development server is needed. The optional local website binds only to
`127.0.0.1:1420` and asks for the same Wealthfolio password; it works without the
phone's primary login. Its separate HttpOnly, SameSite cookie never leaves
loopback. Host/origin validation protects local APIs against DNS rebinding and
cross-site writes. Uploads and responses stream in bounded chunks.

The login page shows laptop connection status. After unlocking, the status widget
shows queued/processing/error counts without exposing filenames or reports in the
status endpoint. Failed files can be retried from the existing statement library.

Install or update service definitions and commands after building the website:

```sh
python3 /home/astro/wealth-system/wealthmanager/install_web_portal.py
```

The installer validates the configured executable, working directory and database
before adopting an externally started backend. It refuses migration during active
statement processing and retains private backups of previous service definitions.

From `/home/astro/wealth-system`:

```sh
python3 wealthmanager/build_web_portal.py
systemctl --user restart wealth-portal.service
systemctl --user status wealth-portal.service
```

The build script explicitly selects `BUILD_TARGET=web`, uses `/wealth/app/` as
its base, externalizes the theme initializer for the script security policy, and
deploys to the laptop's `wealthfolio/apps/frontend/dist-portal`. The normal desktop
build is not appropriate for this portal. The preceding bundle is retained beside
the build for rollback.

Private configuration: `wealthmanager/data/web-portal/` (directory mode 700;
configuration files mode 600). `auth.json` contains a salted Argon2id hash, not a
plaintext password. `backend.json` preserves the existing database location and
backend environment; do not replace it with a new database or use the destructive
upstream `start_server.sh` helper.

The service uses loopback-only ports: laptop 8930 and 8080, optionally 1420; Redmi 18930 and 18080.
The laptop independently verifies the primary portal session via its SSH local
forward 18001. A three-second, bounded memory cache coalesces that verification
during asset bursts; the Redmi still checks the primary session on each request.

Login pages, HTML and financial API responses use `no-store`. Versioned website
assets may be cached privately in the family's browser; these contain no user
statements or credentials. No proxy disk cache or request-body logging is enabled.
Uploaded active documents in the file browser are downloaded with a restrictive
sandbox policy so they cannot execute with portal administrator permissions.

Tests:

```sh
cd /home/astro/wealth-system/wealthmanager
PYTHONPATH=.:tests python3 -m unittest -v test_wealth_portal test_wealth_portal_setup test_wealth_local test_wealth_runtime
node portal-bootstrap.test.mjs
```

The phone's Caddy configuration and portal source have restricted backups under
its `.config/dashboard/` and `.config/caddy/`. This task's temporary screenshots
and review exports are returned to the laptop's private `transfer-backup` folder
after their checksums are verified.

The Git repository backs up code only. Private configuration, uploaded originals,
converted Markdown, report databases, local checkpoints and generated website
bundles are excluded. Restoring the Git repository does not restore financial data.

## Redmi address reservation

On the Airtel H660GM-A, use Advanced Setup → LAN Setup → DHCP Reservation. Reserve
the Redmi's current Wi-Fi MAC for `192.168.1.10`; keep that phone's Wi-Fi identity
consistent. A reservation is already saved in the current router configuration.
The reservation does not require changing WAN, DNS or port-forwarding settings.
