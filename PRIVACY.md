# Privacy and diagnostics

GTAxSkate is a free, noncommercial hobby project. Its public project identity is
Manatros. This page describes the reviewed local tools and public support
workflow; it is not a certification of every dependency or older release.

## Local data

The installer stores selected game/tool folders and installation receipts
locally. The mod and helper tools write settings, board selections and
diagnostic logs locally. Logs, crash diagnostics and receipts can contain
Windows account names, absolute paths, machine details and information about
other loaded mods. Keep originals on your own computer.

The reviewed active Story host contains no project telemetry or automatic log
upload. This does not describe GTA/Rockstar, GitHub, third-party tools or
multiplayer providers. The installer downloads dependencies over the network;
download providers receive the normal connection data, including your IP
address. Multiplayer clients and server operators have separate data flows.

## Sharing a report

Share the minimum needed to reproduce a problem: package ID, relevant versions,
steps and a small redacted log excerpt. Remove personal names, account names,
private paths, IP addresses, passwords, tokens and server license keys. Avoid
full configuration files, memory dumps, recordings with unrelated people or
private game assets. The installer does not upload its transcript automatically.

GitHub issues, comments, commit metadata and release downloads are public. Do
not put private information in them. GitHub processes information under its
[own privacy statement](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement).
Use [SECURITY.md](SECURITY.md) for sensitive findings. If you accidentally publish
a secret, revoke it at its provider; deleting a post does not revoke it or erase
copies. For other personal information, use GitHub's available edit/removal and
support mechanisms. Do not repost that information in a removal request.

## Publication controls and current limits

Publication tools scan packages for known personal patterns, local profile/game
paths and common credentials, including binary strings and ZIP members.
Source paths must be remapped during compilation. A clean scan is a technical
check, not proof that no personal information can exist.

The 16 October 2026 audit found private path strings in historical release
assets. Those releases must not be described as privacy-cleared. The installer's remote binary
fallback fetches only the reviewed v0.6.0 release, pinned by tag and SHA-256
digest. Earlier copies can remain outside this project's control.

Current source fixes and documentation are not automatically present in an
older downloaded installer. Verify the exact package and its checksums.

## Legal and operational scope

EU/German privacy obligations depend on actual processing and who operates a
service. Operating a public multiplayer server, collecting user diagnostics or
introducing monetization requires a separate assessment of information duties,
lawful processing, retention, security and third-party services. Do not assume
that a free project or an English privacy page establishes legal compliance.

References: [GDPR](https://eur-lex.europa.eu/eli/reg/2016/679/oj), particularly
Articles 5, 25 and 32; [BSI software development guidance](https://www.bsi.bund.de/SharedDocs/Downloads/DE/BSI/Grundschutz/IT-GS-Kompendium_Einzel_PDFs_2023/03_CON_Konzepte_und_Vorgehensweisen/CON_8_Software_Entwicklung_Edition_2023.html).
