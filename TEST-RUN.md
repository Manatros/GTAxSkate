# Naechster Testlauf vor main

Stand:14.10.2026. Ziel: den naechsten dev-Testkandidaten pruefen, bevor ein
zugehoeriger Stand nach main oder als bestaetigtes Release veroeffentlicht wird.
Ein gruener Workflow ersetzt keinen Spieltest. Keine Freigabe allein aus
"Test ist fertig" ableiten: Ergebnisse/Video/Logs muessen zugeordnet sein.

## Vorher (etwa3 Minuten)

1. Laufendes GTA/FiveM, OpenIV und CodeWalker schliessen. Testpaket mit seinem
   Installer installieren, Backup-/Rollbackpfad aufbewahren. Keine INI, Boards,
   Keybindings, Spielstaende oder Archive loeschen/zuruecksetzen.
2. Build-ID aus manifest.json und F10 > Developer (Hostbuild/Receipt) notieren;
   Quelltag und Receipt sind andere Angaben als der gepruefte Binaerhash.
   Der Kandidat muss den Controller-Fix fuer GitHub#1 enthalten; v0.5.4 und
   fruehere lokale Testpakete enthalten ihn nicht.
3. Controller anschliessen, Modell/SteamInput/DS4Windows notieren. Aufnahme mit
   Mikrofon starten. GTA Legacy1.0.3889.0 Story, BattlEye aus. Kein GTA Online.
4. F10 > Presets > Developer waehlen. Eingabe-/Debuganzeigen und VerboseLog
   muessen erscheinen. Physikwerte bleiben dabei unveraendert.

## Story-Pflichttests (etwa20-25 Minuten)

| ID | Handlung | Erwartung / Nachweis |
|---|---|---|
| S01 | Board aus-/einpacken, E/F auf-/absteigen, F10 zu Fuss und auf Board zweimal oeffnen/schliessen | Genau ein Menue, keine Haenger oder doppelten Boards. |
| S02 - GitHub#1 | Stillstehen und geradeaus rollen; Controller-Rechtsstick kreisen/flicken und mehrere Tricks ausfuehren | Keine vom Trickstick ausgeloeste freie Kameradrehung. Automatisches Folgen der Fahrtrichtung bleibt erlaubt. Zeitpunkt mit Eingabeanzeige festhalten. |
| S03 - GitHub#1 | S02 in V-Ansichten nah/mittel/fern, Skater-Ego und Skate-Ansicht wiederholen | Blick bleibt vom Trickstick getrennt. Kein Effekt nur in einer Ansicht uebersehen. |
| S04 - GitHub#1 | Mit Controller zielen, Waffenrad oeffnen; dann Maus/Keyboard benutzen und zurueck zum Pad | Bewusstes Zielen/Look funktioniert; Maus-Look kehrt zurueck; ruhendes/virtuelles Pad blockiert Keyboard nicht. |
| S05 | R Kino/Filmer und V mehrfach wechseln, an Wand, Rampe, Bordstein und niedriger Decke fahren | Kamera bleibt vor Hindernissen; kein Festklemmen/Unter-die-Map. Kino verbirgt Skate-HUD, normale Ansicht zeigt es wieder. Einzelne GTA-SMS sind kein Skate-HUD. |
| S06 | Bestehenden Spielstand dreimal laden: inaktiv, waehrend Skaten, anderer Standort. Nicht ueberschreiben | Kein Callback-Fatal, Skaten/Boards/Kollision/Inputs danach weiter nutzbar. |
| S07 | Nach JEDEM Laden fahren, pushen, springen/landen; ohne Presetwechsel aufs HUD achten | Trick-HUD kommt allein zurueck; Skate-Sounds folgen aktueller Bewegung. Ein lebender Audiostream im Log allein genuegt nicht. |
| S08 | GTA-WIP, Skate und RP auswaehlen; danach GTA komplett neu starten | GTA-WIP aendert nichts; kontrollierte Skate/RP-Werte bleiben gespeichert. Eigene Boards/Bindings/Audio bleiben erhalten. Kein Test eines noch fehlenden User-Presets behaupten. |
| S09 | Developer-Diagnosen ausschalten, normal weiterskaten | Extra Inputs/Debugtext/VerboseLog aus; normales HUD bleibt. Kein dauerhaftes Flackern. |
| S10 | Shop G/E oeffnen, eine Vorschau/Kauf/Fahrt pruefen, V/R zurueck, normale GTA-Steuerung und SelfRadio pruefen | Kein bestehender Ablauf kaputt; Kauf nicht auf eine exakte Auslagen-Identitaet erweitern, die noch Backlog ist. |
| S11 | Mindestens10 weitere Minuten fahren, zwei Reloads nach Moeglichkeit wiederholen | Kein neuer Crash, deutlicher dauerhafter Leistungseinbruch oder Zustandsverlust. FPS/Frametimes und Logzeitpunkte notieren; ohne Messung keinen Speicher-Leak behaupten. |

## FiveM - eigener Freigabeblock (etwa15 Minuten nach Einrichtung)

Voraussetzung: neuer lokaler Server, Cfx-Schluessel nur lokal, Windows-Regeln
und Routerzugang eingerichtet. Vollstaendige Anleitung:
[FIVEM-FRIENDS-TEST.md](FIVEM-FRIENDS-TEST.md). Aktuell ist dies ein experimenteller
SK8M-Lua-Fallback mit vier Multiplikatoren, keine Skate3/WASM-Physik.

| ID | Handlung | Erwartung |
|---|---|---|
| F01 | Lokal verbinden, F6, W/S/A/D/Space; F8 skatedev und skaterules | Drei Resources laufen, Board/Spieler nutzbar, gueltige Regeln mit vier unterstuetzten Feldern. |
| F02 | Server-Developer-Policy, `skateset PushSpeed 1.1`, danach `skateset PushSpeed 99` und `skateset Pop 2` versuchen |1.1 bestaetigt;99/unsupported Pop abgelehnt. Darstellungen nicht als Physik-Anti-Cheat ausgeben. |
| F03 | Gesicherte locked-Policy laden, serverseitig skaterules_reload; Versuch erneut | Fahrt wird beendet, neue Anpassung gesperrt. Nur Admin kann Policy wechseln. |
| F04 | Rule-/Adapter-/Skateboard-Resource waehrend Fahrt stoppen/neustarten | Keine dauerhaft eingefrorene Figur/Kamera/Altboards; ohne gueltige Regeln kein Aufsteigen. |
| F05 | Freund von anderem Internetanschluss verbinden lassen | Gegenseitige Fahrer/Boards/Tricks sichtbar. Spawnen, Bail, Distanz/Streaming, spaeter Beitritt, Disconnect/Reconnect pruefen. Lokales Hairpin ist kein externer Netztest. |

Nicht eingerichteter Server oder nicht ausgefuehrter Zwei-Spieler-Test = NOT RUN,
niemals PASS. FiveM darf dann nicht als bestaetigter Mehrspieler-Releaseumfang
beworben werden. Fuer main muss der tatsaechlich enthaltene Freigabeumfang
vorher feststehen; ungepruefte Features werden weiter dev/experimentell gehalten.

## Ergebnis fuer die Freigabe

| Feld | Eintrag |
|---|---|
| Tester, Datum/Uhrzeit | |
| Build-ID, Host-/Runtimehash | siehe manifest.json, nicht schaetzen |
| Controller / Eingabesoftware | |
| S01-S11 | pro ID PASS / FAIL / NOT RUN |
| F01-F05 | pro ID PASS / FAIL / NOT RUN |
| Fehler | ID, Zeitstempel, erwartet/tatsaechlich, reproduzierbare Schritte |
| Video/Logs | host/runtime/ScriptHookV bzw. FiveMclient+server; Schluessel/Token/personenbezogene Daten vorher entfernen |
| main-Freigabe | explizite Entscheidung nach Auswertung, kein automatisches Ja |

main bleibt gesperrt bei Crash, reproduzierbarem Controller-Kamerafehler#1,
HUD-/Audioausfall nach Reload, Install-/Rollbackverlust oder einem neuen
blockierenden Gameplayfehler. Vor Promotion: CI gruen fuer exakt den geprueften
Commit, Testbinaerhashes zugeordnet, Befunde bewertet, bekannte Grenzen und
Release-Assets kontrolliert. Keine alte ungetestete ZIP als neuen Build ausgeben.
GitHub#1 bleibt bis zur echten Controller-Bestaetigung offen; eine C++-Fixture
ist noch kein gemeldeter Spieler-Erfolg.
