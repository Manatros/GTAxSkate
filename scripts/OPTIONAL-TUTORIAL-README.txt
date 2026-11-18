GTAxSkate Tutorial Island — optional, experimental
====================================================

This download contains tools only. It does not contain Skate 3 assets. You
must own and supply your own extracted Skate 3 files. The island is incomplete:
collision, textures and travel position have known problems. The base mod works
without this add-on.

Requirements
  - GTA V Legacy 1.0.3889.0 with GTAxSkate v0.9.1 installed
  - Extracted Skate 3 disc folder (contains default.xex and data/)
  - Python 3, Git, and the pinned SK8-ENGINE/skate-3-rust-engine donor checkout
  - rage.exe from the installed SK8V tools
  - GTA V, OpenIV and CodeWalker closed during installation

From a terminal, download the exact read-only donor revision:

  git clone https://github.com/SK8-ENGINE/skate-3-rust-engine.git tutorial-donor
  git -C tutorial-donor checkout cb7968930f14dad38457e98720d1a274e469eec2

Use rage.exe from the SK8V 1.1.10 tools folder (the installer downloads it
under your local GTAxSkate build cache). Then run:

  .\Install-TutorialIsland.ps1 -Skate3 "<extracted Skate 3 folder>" `
    -GtaDir "<GTA V Legacy folder>" -DonorDir "<donor checkout>" `
    -RageExe "<path to rage.exe>"

The preparation writes retail-derived output under this download's local/
folder. It validates both DIST_SkateSchool and DIST_StartPark before installing.
The installer writes only into SK8V's custom addon folder and the
update\x64\dlcpacks\skate_school\dlc.rpf location. It never edits update.rpf
or any x64*.rpf archive. To uninstall, run:

  .\install-tutorial-addon.ps1 -Mode Uninstall -GtaDir "<GTA V Legacy folder>"

This experiment is not covered by the release's gameplay acceptance.
The archive includes GTAxSkate and SK8V license and notice files.
