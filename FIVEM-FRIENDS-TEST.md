# FiveM: vorbereiteter Friends-Test (SK8M Lua-Fallback)

Stand 14.10.2026. Dieser Test nutzt SK8Ms Lua-Skating. Die Skate-3-Physik ist
nicht angebunden; der Zwei-Spieler-Test steht noch aus. Bewegung und Tricks
laufen auf dem jeweiligen Spieler-Client. Serverbestätigte Einstellungswerte
sind **keine serverautoritative Physik und kein Anti-Cheat**.

## Was bereitsteht

| Bestandteil | Umfang |
|---|---|
| Basis | SK8M `50b923097691d7c73d6e8eb719cffc5cab40d138`, Apache-2.0; LICENSE/NOTICE liegen im Paket. |
| Darstellung | GTA-eigenes Board-Prop und GTA-Standanimation; keine Skate-3-Dateien, keine WASM-Engine. Sounds bleiben aus. |
| Netz | Vorhandene SK8M-Netzwerkobjekte und normaler Spieler-Ped; keine zusätzliche Multiplayer-Engine. |
| Adapter | Vier Multiplikatoren: PushSpeed, PushPower, Braking, Steering. Originalwerte werden bei jedem Update neu skaliert. |
| Regeln | locked/selected/ranges/all; andere Katalogoptionen bleiben gesperrt und werden ausgeblendet. |
| Grenzen | Kein F10-Menü, kein vollständiger SK8V-Port, keine Schwierigkeit-/Kamera-Preset-Gleichwertigkeit. Andere Einstellungsbefehle dieser Testbasis sind deaktiviert. |

Story-Installation und originaler SK8M-Checkout bleiben unverändert.
Der Overlay erzeugt nur ein neues Paket in `build/fivem-friends`.
`package-manifest.json` enthält Quellrevision und Quell-/Overlay-/Ausgabehashes.

## 1. Lokal vorbereiten

1. `scripts/prepare-fivem-friends.ps1` ausführen, optional `-Name mein-test`.
   Ein existierender Zielname wird abgewiesen.
2. `build/fivem-friends/<Name>` enthält drei Resources und
   `server-local.example.cfg`. Es wurde nichts installiert oder gestartet.
3. Das Beispiel ist **keine vollständige startbare Serverkonfiguration**. Es
   enthält keinen Schlüssel und setzt Loopback/`sv_lan 1` für einen isolierten
   Test auf demselben PC. Es öffnet keinen Port.

## 2. FiveM und Server einrichten (manuell)

1. Beide Spieler installieren [FiveM](https://fivem.net/) für ihre eigene
   GTA-Installation. Story-ASI/Script Hook V werden dafür nicht gebraucht.
2. FiveM-Serverdateien fehlen: Richte zuerst den Server anhand der
   [offiziellen txAdmin-Anleitung](https://docs.fivem.net/docs/server-manual/setting-up-a-server-txadmin/)
   ein. Anmeldung, Lizenzbedingungen und Admin-Zugang erledigt der Serverbesitzer.
3. Erstelle den Server-Schlüssel im offiziellen Cfx-Portal und trage ihn nur
   lokal ein. Nicht in Chats oder Git ablegen. Verwende den Portal-Link in der
   offiziellen Anleitung. Keine Zugangsdaten in dieses Paket schreiben.
4. Einen unveränderten CFX-Default-Server zuerst erfolgreich starten und wieder
   stoppen. Unsere Vorbereitung lädt FXServer nicht herunter und startet ihn nicht.
5. Nur in einen **neuen, isolierten Testserver** kopieren: die drei Verzeichnisse
   unter `resources` aus dem vorbereiteten Paket. Falls dort `skateboard`,
   `gtaxskate_rules` oder `gtaxskate_sk8m` existiert, einen neuen Testserver
   verwenden. Vorhandene Resources nicht ersetzen. Der bisherige Rules-Installer
   installiert weiterhin nur Regeln, nicht diesen Friends-Test.
6. In der vom Benutzer eingerichteten `server.cfg` ergänzen:

   ```cfg
   sv_scriptHookAllowed 0
   ensure gtaxskate_rules
   ensure gtaxskate_sk8m
   ensure skateboard
   ```

   Für den isolierten Test vorhandene Endpoint-Zeilen bewusst durch die
   Loopback-Zeilen des Beispiels ersetzen; keine Doppelbindungen anhängen.
   Schlüssel und sonstige Serverkonfiguration lokal behalten.

## 3. Einstellungen prüfen

Standard ist `locked`: die vier Servervorgaben gelten, Spieler dürfen nichts
ändern. Die Policy liegt unter `resources/gtaxskate_rules/policy.json`.

| Modus | Wirkung für diese vier unterstützten Einstellungen |
|---|---|
| locked | Jede Spieleränderung gesperrt, auch einzeln freigegebene Zeilen. |
| selected | Nur ausdrücklich mit `adjustable: true` freigegebene Schlüssel. |
| ranges | Nur ausdrücklich freigegeben; numerische Schlüssel brauchen min und max. |
| all | Alle vier innerhalb 0,5–3; individuelle Sperren/Bereiche gelten weiterhin. |

Für einen begrenzten Test `mode` auf `ranges` setzen und `rules` ersetzen:

```json
{
  "PushSpeed": { "adjustable": true, "min": 0.8, "max": 1.1 },
  "PushPower": { "adjustable": true, "min": 0.8, "max": 1.1 },
  "Braking": { "adjustable": true, "min": 0.8, "max": 1.2 },
  "Steering": { "adjustable": false }
}
```

Vorgaben müssen innerhalb ihrer Bereiche liegen. Im Serverterminal
`skaterules_reload` ausführen; im Spiel braucht dieser Adminbefehl die ACE
`command.skaterules_reload`. Fehlerhafte Policies lassen den letzten gültigen
Stand bestehen. Regeländerungen stoppen Fahrer und löschen ihre Anpassungen;
erneutes Aufsteigen verwendet den neuen Stand.

Spieler: F8 → `skaterules`; `skateset PushPower 1.1`. Unterstützte akzeptierte
Spieleranpassungen wirken sofort. `skateset Pop 2` bleibt gesperrt: Pop ist nicht
implementiert. Ohne gültige Policy/Adapter darf kein Aufsteigen gelingen.
Ein Wegfall während der Fahrt stoppt sie und gibt den Ped über SK8Ms eigene
Aufräumfunktionen frei.

## 4. Erst lokal, dann zu zweit testen

1. Server manuell starten. Lokal in FiveM F8 → `connect 127.0.0.1:30120`.
2. F6 steigt auf/ab (in FiveM umbelegbar). Das ist der SK8M-Teststandard,
   noch nicht unsere vollständige Story-Belegung.
3. Schieben, bremsen, lenken, springen, Bail und erneutes Aufsteigen prüfen.
4. Vor dem Internet-Test entscheidet der Besitzer über Hosting, erreichbare
   TCP-/UDP-Spielendpoints, Firewall/Router und Zugangsbeschränkung. Loopback
   und `sv_lan 1` sind nur für den isolierten Lokaltest; Freunde auf anderen
   PCs können damit nicht verbinden. Unsere Skripte richten keine Freigabe oder
   öffentliche Serverkonfiguration ein. txAdmin ist nicht der Spielendpoint.
   Keine globale Entity-Lockdown-Lockerung als Workaround verwenden.
5. Beide Spieler prüfen: gegenseitiges Board und Ped, Tricks, Bail-Board,
   Distanz/Streaming, später Beitritt, Besitzer-Disconnect und Reconnect.
6. Während der Fahrt Rules-/Adapter-/Skateboard-Resource stoppen und neu starten.
   Keine eingefrorene Figur, Kamera oder alten Boards dulden. Ohne Regeln darf
   erneutes Aufsteigen nicht möglich sein.
7. locked/selected/ranges/all, gesperrte Felder, Bereichsgrenzen und Regelwechsel
   prüfen. Alte Generation-/Revision-Grants dürfen nicht freigeben.

Server-/Clientlogs, Ping und Ressourcen-/Framekosten mit beiden Spielern
festhalten. Keine Schlüssel oder Kontodaten mitschicken. Erst the maintainer bestätigt
den realen Test; Offlineprüfungen sind keine Mehrspieler-Freigabe.


## Developer-Preset und konkreter Kurztest

Story: F10 > Presets > Developer schaltet ShowInputs, Debugtext, VerboseLog,
KeyboardHints, TrickAnalyzer und Original-HUD ein. In Developer kann man die
Diagnoseanzeigen/VerboseLog wieder ausschalten. Das aendert keine Fahrphysik.

FiveM hat bewusst einen eigenen, kleineren Developer-Test:

1. Im neuen Testserver die bisherige policy.json separat sichern.
2. Aus diesem Paket policy-presets/developer.json als
   resources/gtaxskate_rules/policy.json kopieren. Nur der Admin waehlt dieses
   Preset; ein Spieler kann es nicht aktivieren.
3. In der SERVER-Konsole skaterules_reload eingeben. Alle vier unterstuetzten
   Regler sind danach in0,5-3 freigegeben, Grundwerte jeweils1.
4. Im FiveM-Client F8 oeffnen, skatedev und skaterules eingeben. Erwartet:
   gueltige Generation/Revision, vier Werte und deren Freigabe; keine anderen
   angeblich unterstuetzten Optionen. skatedev zeigt nur Diagnose an.
5. F6, W push, S brake, A/D steer, Space ollie testen (SK8M-Standard;
   persoenliche FiveM-Keybindings koennen abweichen).
6. F8: skateset PushSpeed 1.1. Die Anzeige soll den bestaetigten Wert zeigen;
   skateset PushSpeed 99 und skateset Pop 2 muessen abgelehnt werden.
7. Wieder die gesicherte locked-Policy einsetzen, SERVER skaterules_reload.
   Der Fahrer muss absteigen; eine weitere Spieleranpassung muss gesperrt sein.
8. Mit zweitem Spieler gegenseitige Boards/Tricks pruefen, danach Rule-Resource
   stoppen/neustarten und Disconnect/erneutes Beitreten testen.

FXServer/txAdmin-Anmeldung und Cfx-Schluessel sind Voraussetzungen. Sie sind
kein Bestandteil des ZIP; ohne fertig eingerichteten Server kann man noch nicht
verbinden. Die offizielle Downloadseite verlangt Zustimmung zur Plattformlizenz
beim Download; diese Entscheidung trifft der Besitzer im offiziellen Ablauf.
