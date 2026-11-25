This build contains the original game's own files, for the private use of the person who owns the game, on their own computer. It must not be shared or published.

(In this build no file from the original program or the game is actually bundled: the app draws everything itself and loads mod pictures from Nexus Mods while it runs. See CREDITS.md.)

# DPG: Rogue Core Mod Manager

**DRG: RG Mod Manager** – Mod-Manager für Deep Rock Galactic: Rogue Core.

Ein eigener Mod-Manager im Stil des Deadlock Mod Managers (dunkles Design, Seitenleiste, Mod-Karten mit Schaltern), nur für Rogue Core und für Mods von https://www.nexusmods.com/games/deeprockgalacticroguecore/mods.

## Was die App kann

- **Sprache und Design**: in den Einstellungen – Deutsch/Englisch und vier Designs (Standard, Mitternacht, Magma, Hell); beides wird gespeichert.
- **Exportieren / Importieren** (Meine Mods): speichert alle Mods samt Dateien, Namen und An/Aus-Zustand in eine .zip und installiert sie aus so einer .zip wieder (z. B. für Freunde oder einen anderen PC).
- **Spielversion und "Evtl. veraltet"**: die App liest die installierte Build-Nummer aus Steam (appmanifest) und die neueste Version aus Steams Patch-Notes (z. B. "EA 03.04.00"). Mods, die auf Nexus seit dem letzten Spiel-Patch nicht aktualisiert wurden, bekommen "⚠ Evtl. veraltet", die anderen "✔ Aktuell"; neue Mods (unter 7 Tagen) "Neu". Filter dafür im Store und als Reiter in Meine Mods. Nexus sagt nicht, für welche Spielversion ein Mod gebaut ist, und die Nexus-Webseite lässt keine automatischen Abrufe zu – Kommentare und Bug-Meldungen können deshalb nicht gezählt werden.
- **Updates erkennen**: beim Start fragt die App Nexus nach den Dateien jedes installierten Mods. Für Mods, die über die App installiert wurden, kennt sie die genaue Datei und meldet eine neuere Datei derselben Reihe (z. B. V2.2 → V2.3) als "⬆ Update verfügbar" (Reiter "Updates", Hinweis oben in Meine Mods, Markierung "⬆ Update für dich" auf der Mod-Seite). Bei Mods, die schon im Paks-Ordner lagen, vergleicht sie die Version im Dateinamen (z. B. "…V2.3_P.pak") mit der neuesten Nexus-Version; steht im Dateinamen keine Version, kann sie es nicht sagen – einmal über die App neu installieren, dann wird der Mod genau verfolgt.
- **Fenster**: eigene Titelleiste wie beim Deadlock Mod Manager (Minimieren, Fenstermodus/Maximieren, Schließen; ziehen zum Verschieben, Doppelklick maximiert, Ränder zum Größe-Ändern). Maus-Seitentasten (und Alt+←/→ sowie die Pfeile oben) gehen zurück/vor wie im Browser.
- **Einstellungen**: Anwendung (Sprache, automatische App-Updates, Mods beim Start wieder anwenden, nach Spielstart minimieren, Standard-Sortierung), Darstellung, Spiel, Nexus, App-Updates, Mod-Backups (erstellen, automatisch vor Updates/Importen/Wiederherstellen, Anzahl behalten, wiederherstellen, löschen) und Daten (Download-Cache leeren, Mod-Liste zurücksetzen, alle Mods entfernen).
- **Wiki**: eigene Seite mit Suche im Rogue-Core-Wiki (deeprockgalactic.wiki.gg), Schnellzugriffen und "Wiki öffnen".

- **Meine Mods**: zeigt alle installierten Mods als Karten; ein Schalter schaltet jeden Mod an/aus, der Mülleimer entfernt ihn. Mods, die schon im Paks-Ordner des Spiels liegen, werden automatisch gefunden und – wenn der Name passt – mit ihrer Nexus-Seite verknüpft (Bild, Autor). Reiter: Alle / Aktiv / Deaktiviert / Updates, Suche, Sortierung.
- **Mods entdecken**: alle Rogue-Core-Mods von Nexus (öffentliche Nexus-API, ohne Anmeldung), mit Suche, Kategorie und Sortierung (Beliebteste, Downloads, Zuletzt aktualisiert, Neueste, Name). Klick öffnet die Detailseite mit Bild, Infos, Dateien und Beschreibung.
- **Zips und UE4SS-Mods**: Downloads kommen meist als .zip; die App packt die .pak (auch aus Unterordnern) selbst aus. Zips ohne .pak, die ein UE4SS-Mod sind (enabled.txt, Scripts/main.lua oder dlls/main.dll), landen in UE4SS' Mods-Ordner (der zuletzt von UE4SS benutzte) und lassen sich wie andere Mods an-/ausschalten (über enabled.txt), entfernen, exportieren und importieren. UE4SS-Mods, die schon im Spiel liegen, werden erkannt; UE4SS' eigene Mods aus mods.txt bleiben außen vor.
- **Ohne API-Schlüssel installieren**: "⬇ Herunterladen" öffnet die Datei auf Nexus; dort (mit kostenlosem Nexus-Konto angemeldet) "Manual download" klicken. Die App beobachtet bis zu 30 Minuten den Downloads-Ordner, erkennt die Datei am Nexus-Dateinamen und installiert sie automatisch mit allen Infos (Bild, Autor, Version, Update-Verfolgung).
- **Herunterladen**: mit Nexus Premium direkt in der App. Ohne Premium öffnet der Knopf die Datei auf Nexus; dort "Mod Manager Download" klicken, dann lädt und installiert die App die Datei (dafür einmal in den Einstellungen "Als Nexus-Download-Programm eintragen"). Für beides braucht man den persönlichen API-Schlüssel von Nexus (Einstellungen → "Schlüssel holen").
- **Mods hinzufügen**: .zip oder .pak per Klick oder Ziehen & Ablegen.
- **Downloads**: Fortschritt und Ergebnis jedes Downloads.
- **Oben**: "Ohne Mods starten" schaltet alle Mods kurz aus und startet das Spiel über Steam; "Mit Mods starten" schaltet die gewählten wieder ein und startet. Ist UE4SS installiert, wird dabei auch UE4SS selbst aus- bzw. eingeschaltet (sein Lader `dwmapi.dll` bzw. `xinput1_3.dll` in `RogueCore\Binaries\Win64` bekommt die Endung `.disabled`) – damit laufen auch keine UE4SS-Mods wie NegotiationMoreChoices. Der Zustand steht unten in der Statusleiste ("UE4SS aktiv/pausiert").

## Wie Mods installiert werden

Rogue Core lädt Mods als `.pak`-Dateien (manchmal mit `.utoc`/`.ucas`) aus `…\Deep Rock Galactic RogueCore\RogueCore\Content\Paks` (so steht es in den Installationsanleitungen auf Nexus). Die App kopiert die Dateien dorthin. "Aus" heißt: die Datei bekommt die Endung `.disabled` (das Spiel lädt nur `*.pak`). Die Originaldatei des Spiels (`RogueCore-Windows.pak`) wird nie angefasst. Eine Kopie jedes Mods liegt in `%APPDATA%\CoreModManager\mods`, die Bibliothek in `library.json`.

## So installierst du die App

1. Auf https://github.com/DasLykia/DPG-Rogue-Core-Mod-Manager/releases die neueste **`…-win-Setup.exe`** herunterladen und starten. Sie installiert die App (Startmenü + Desktop) und richtet automatische Updates ein.
2. Neue Versionen erkennt die App beim Start selbst: unten links erscheint "⬆ App-Update …"; ein Klick lädt es und startet die neue Version. In **Einstellungen → App-Updates** kann man auch von Hand suchen.
3. Für Downloads ohne Premium einmal in den Einstellungen "Als Nexus-Download-Programm eintragen" drücken (in der installierten App).

## Neue Version veröffentlichen (Velopack)

`.\release.ps1 0.2.0` – setzt die Version in Cargo.toml, baut die App, packt sie mit Velopack (`vpk pack`, inkl. Setup.exe und kleinen Delta-Updates) und lädt sie als GitHub-Release hoch (`vpk upload github`). Danach bieten alle installierten Apps das Update an. Nötig: Rust (GNU-Toolchain), `vpk` (`dotnet tool install -g vpk`) und angemeldete GitHub-CLI (`gh auth login`).

## Starten / Bauen

`cargo build` in diesem Ordner (nutzt die GNU-Rust-Toolchain, siehe rust-toolchain.toml), dann `target\debug\core-mod-manager.exe`. Automatische Updates funktionieren nur in der über Setup.exe installierten Version.
Prüf-Optionen: `--screenshot datei.png`, `--self-test datei.json`, `--page store|settings|downloads|add|wiki|detail:<id>`, `--lang de|en`, `--theme standard|midnight|magma|light`.

SPEC.md beschreibt, wie das Original (Deadlock Mod Manager 2.3.0) aufgebaut ist – zum Weitergeben an einen anderen Helfer.
