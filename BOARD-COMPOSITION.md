# Optional composed shop boards

The accepted v0.6.0 / SK8V 1.1.10 Story build can prepare the selected deck, wheels,
griptape and separately painted truck parts as one skeletal drawable. SK8V keeps
control of animation and GTA/Skate transitions. Preparation runs outside the game
tick; the current board remains until the new model is verified and loaded.
This helper is an experimental opt-in, separate from the stable installer.

On 16 October 2026 the tester reported that wheel speed appeared correct and
confirmed the rear-wheel direction correction works. Stationary stopping was
not separately confirmed. Truck steering, shop variations, carrying and the
remaining [release tests](TEST-RUN.md) still need their own evidence.

## Requirements

- Windows, GTA V Legacy 1.0.3889.0 Story Mode, working SK8V/GTAxSkate converted data.
- The matching release host/runtime pair and its manifest; arbitrary
  host/runtime combinations are unsupported.
- Python 3.10 or newer and the existing pinned RAGE executable used for asset
  preparation. Keep the reviewed tool versions; do not substitute an unknown tool.
- Your assets directory containing `private/board-native/skatev_board.ydr`,
  `private/board-native/skatev_board.ytyp` and `private/board-parts/`.
- The four helper scripts in the test package's `composer/` directory, or in
  the source repository's `scripts/` directory.

No converted game assets, private paths or local configuration belong in GitHub.

## Set up and check

1. Close GTA, FiveM, OpenIV and CodeWalker. Preserve the test package and its manifest.
2. From the extracted Story test directory, run the command below, replacing
   the three placeholder paths with existing local paths. The assets path is
   the root containing `private/`, rather than `private/` itself.

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\composer\setup-board-composer.ps1 -Assets "YOUR ASSETS FOLDER" -Python "YOUR python.exe" -Rage "YOUR rage.exe"
```

3. Keep the printed helper backup path. Setup copies the worker/preparation
   scripts into `private/board-composed/`, records tool hashes, writes local
   `worker.ini` and enables the helper. It preserves original board files and
   player settings. Paths containing links are rejected.
4. Install the paired Story test binaries with `Install Test.cmd`; keep its
   separate binary backup path. Fully restart GTA after installation.
5. For comparison keep `BoardShop=1` and `CarryBoard=0`. Select a custom board
   in the shop and wait for preparation; it can take several seconds. The host
   starts the helper automatically. Check the host log for
   `board parts: riding skatev_c_... (single drawable)`.
6. Check preview, purchase and riding after changing deck/wheels, split truck
   paint and griptape; check getting on/off and returning to the saved setup.
   Separately test stopping for 10 seconds and slow forward/reverse travel.

When using the source scripts instead, replace `./composer/` in the command
with `./scripts/`. The stable installer does not perform this setup.

## Roll back

Close GTA and the model tools first. Helper and binary backups are independent.
Use the printed backup paths from the respective installation steps.

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File "HELPER BACKUP FOLDER\restore-board-composer.ps1" -Backup "HELPER BACKUP FOLDER"
powershell -NoProfile -ExecutionPolicy Bypass -File .\restore-test-build.ps1 -Backup "BINARY BACKUP FOLDER"
```

Helper rollback restores its previous scripts/configuration/enabled marker;
it does not replace the original models or player settings. Binary rollback
restores the prior host/runtime and installation state; it does not undo
settings deliberately changed through the menu. See the package's TESTPLAN.

## Report a problem

Report package/build ID, selected parts, expected/actual result, failing step
and relevant timestamped host/runtime logs through [GTAxSkate issues](https://github.com/Manatros/GTAxSkate/issues).
Redact names, local paths, addresses and credentials. Keep `worker.ini`, helper
backup receipts and converted models local; the configuration and receipts
contain machine-specific paths. A short written report is sufficient when no
recording is available. See [support guidance](SUPPORT.md).
