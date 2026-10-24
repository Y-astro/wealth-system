# Deployment migration sources

These scripts record the one-time migrations used to set up the existing Redmi
portal and laptop connection. They contain code only, without password hashes,
private certificate keys, or session credentials. Several intentionally assert
that a legacy configuration is present and must not be rerun on an updated host.

For the current laptop use `wealthmanager/install_web_portal.py` and
`wealthmanager/WEB_PORTAL.md`. The initial laptop migration captures the existing
backend configuration without creating a new database. The phone migrations add
the family/admin cards, separate HTTPS origin, authentication forwarding, secure
HTTP redirects, local assets, and uploaded-file sandboxing. They require an
already configured home portal and its local certificate authority.
