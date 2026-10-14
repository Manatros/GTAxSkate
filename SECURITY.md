# Security reporting

This is an experimental game mod project, not a security product. No security response SLA, dedicated contact address or private reporting service is established by this document.

## Report sensitive findings carefully

If GitHub offers **Report a vulnerability** in this repository's Security tab, use that private reporting route. If that option is unavailable, open a minimal [GTAxSkate issue](https://github.com/Manatros/GTAxSkate/issues/new) asking for a private reporting route, without exploit steps, secrets or personal information. Do not assume an issue or an ordinary pull request is private.

Provide affected package/resource versions, the trust boundary involved and a minimal reproduction once a suitable channel is available. Never include Cfx server license keys, account tokens/passwords, private game assets or unrelated personal data. Revoke any exposed credential through its provider rather than posting it for diagnosis.

## Relevant boundaries

- Story Mode is targeted at Windows GTA V Legacy 1.0.3889.0; the Story ASI must not be enabled for GTA Online.
- Experimental FiveM rules validate the adapter's supported settings and permissions on the server. The Lua friends prototype still uses client-owned movement/physics; it is not a server-authoritative simulation or anti-cheat.
- A policy resource beside an unrelated/unmodified skating resource does not enforce that resource's behavior. Report bypasses with the exact adapter and source revision.
- Local installers and preparation tools should preserve configuration, refuse incompatible/linked destinations where specified and avoid overwriting a live installation. Report unintended writes with redacted paths and reproducible steps.

There is no published security-maintenance matrix for older releases. Include the exact version and whether it reproduces on the relevant development build; do not use an unverified update on a public server solely because it is newer.
