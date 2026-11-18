GTAxSkate Friends-Only Coop — optional, experimental
====================================================

This is a separate opt-in package. It is not installed by the base setup.
Two-PC gameplay has not been validated; localhost works on the maintainer's
machine. Online hosting may fail behind DS-Lite or carrier-grade NAT.
Third-party license texts and notices are included under licenses/ and in
THIRD-PARTY-NOTICES.txt.

The package intentionally contains no host key. In a private, extracted copy
of this package, generate your own local key and server settings:

  .\Setup-FriendsCoop.ps1 -Mode GenerateHostConfig -TargetDir .\server

Install the client on the host PC (defaults to localhost) with:

  .\Install-OptionalCoop.ps1 -GtaDir "<GTA V Legacy folder>"

For another PC, install the same package and set -HostIp to a private LAN or
Tailscale/WireGuard address and -FriendKey to the host's locally generated
key. Share the key only with trusted friends using a private channel. Do not
post it in logs, screenshots or public issues. To run the standalone server,
start .\server\Start-FriendsCoopServer.ps1 from this extracted package.

The installer records a rollback receipt. Uninstall through:

  .\Install-OptionalCoop.ps1 -Mode Uninstall -GtaDir "<GTA V Legacy folder>"
