# Wie der Deadlock Mod Manager 2.3.0 funktioniert – und wie die Rogue-Core-Version davon abweicht

Zum Weitergeben an einen anderen Helfer. Alles unter "Original" wurde aus dem Installer gelesen
(`Deadlock.Mod.Manager_2.3.0_x64-setup.exe` → NSIS → `deadlock-mod-manager.exe`, eine Tauri-App:
Rust-Hülle + React/TypeScript-Oberfläche mit shadcn/ui und Tailwind; die Web-Dateien sind
brotli-komprimiert in der .exe eingebettet).

## 1. Original: Aufbau

### Fenster
- **Obere Leiste**: links Profil-Auswahl ("Default Profile", Profile = gespeicherte Sätze aktiver Mods);
  rechts "Launch without mods" (umrandet), "Launch modded (N mods)" (gefüllt, Akzentfarbe), "Sign In".
- **Seitenleiste links** (einklappbar, "Collapse menu"): Logo + Version, Gruppen
  - General: Dashboard, Downloads, Settings
  - Mods: Mods Library, Mods Store (deutsch: "Meine Mods", "Mods entdecken", "Mods hinzufügen")
  - Custom Servers: Server Browser · Customization: Crosshairs, Hero Skins, Mod Foundry, Performance, Stats
  - Plugins: Manage plugins · unten: Developer, Debug, Documentation, Experimental, Social-Links
- **Statusleiste unten**: "Status", Lautstärke, "Check for Updates".

### Routen (aus dem Router im JS-Bundle)
`/` Dashboard · `/my-mods` Bibliothek · `/mods` Store · `/mods/:id` Mod-Detail · `/authors/:id` ·
`/collections/:id` · `/maps` · `/favorites` · `/add-mods` · `/downloads` · `/servers` · `/settings` ·
`/settings/autoexec` · `/settings/experimental` · `/contributors` · `/developer` · `/plugins/manage` ·
`/plugins/:id` · `/debug` · `/crosshairs` · `/skins` · `/stats` · `/foundry` · `/performance`

### Bibliothek ("Mods Library")
- Kopf: Titel + Zähler "N addon files", Untertitel "Mods you have downloaded or installed",
  rechts Aktualisieren und "+ Add mods".
- Suchfeld "Search mods or authors", Filter Held/Kategorie, Reiter **All / Installed / Downloaded / Conflicts**
  mit Zahlen, Sortierung (Default, Last Updated, Download Count, Rating, Release Date, **Install Order**,
  Hero, Name, Recently Added), Raster/Liste.
- Karten: Bild oben, Name, "By <Autor>", unten Symbole + **An/Aus-Schalter**; Abzeichen oben rechts
  ("Needs an update for this patch", "Fixed for this patch", "Audio", "Custom").
- "Update All Mods"-Dialog (installierte vs. verfügbare Version, Varianten wählen).

### Store ("Mods Store")
- Quelle: **GameBanana** (für Deadlock). Filter: Kategorie, Held, Include/Exclude, NSFW zeigen/verbergen,
  nur Audio / nur Karten, veraltete verbergen, Zeitraum (24 h … 1 Jahr), Inhaltstyp (Mods, Sounds, Maps, WiPs,
  Collections); Sortierung wie oben.
- Detailseite: Galerie, Beschreibung, Dateien mit Download, Favorit-Stern.

### Mods hinzufügen ("Add Mods")
"Upload mod files. Drag & drop is supported." – eine .vpk oder ein .zip/.rar/.7z-Archiv ablegen.

### Installation im Original (Deadlock)
Mods sind `.vpk`-Dateien im Addon-Ordner des Spiels; der Manager schreibt die Suchpfade in `gameinfo.gi`
und schaltet Mods durch Umbenennen/Verschieben an und aus. "Launch modded" startet über Steam mit aktivem
Profil; "Launch without mods" ohne.

### Design (CSS-Variablen, Dunkelmodus)
background `hsl(20 9% 6%)` · card `hsl(20 8% 9%)` · foreground `hsl(38 65% 97%)` · primary `hsl(42 60% 84%)`
(beige/gold) · secondary `hsl(15 10% 16%)` · secondary-foreground `hsl(41.6 60% 63%)` · muted `hsl(0 0% 14.9%)`
· muted-foreground `hsl(0 0% 63.9%)` · border `hsl(0 0% 14.9%)` · destructive `hsl(0 62.8% 30.6%)` ·
Radius `0.5rem` · Schrift Geist (+ "Forevs Demo" für Überschriften) · 20 Sprachen inkl. Deutsch.

## 2. Rogue-Core-Version (diese App)

| Original | Rogue Core |
|---|---|
| GameBanana | Nexus Mods, Spiel-Domain `deeprockgalacticroguecore`, Spiel-ID 9293 |
| .vpk + gameinfo.gi | .pak/.utoc/.ucas direkt in `RogueCore\Content\Paks`; aus = Endung `.disabled` |
| Steam-Start Deadlock | `steam://rungameid/2605790` |
| Profile, Dashboard, Server, Skins, Foundry, Plugins, Stats | **nicht gebaut** (Deadlock-spezifisch oder später) |
| Reiter Installed/Downloaded/Conflicts | Alle / Aktiv / Deaktiviert / Updates |

### Nexus-Schnittstellen
- Liste (ohne Schlüssel): `POST https://api.nexusmods.com/v2/graphql`
  `mods(filter:{gameDomainName:[{value:"deeprockgalacticroguecore"}]}, count:100, offset:N)`
  → modId, name, summary, author, endorsements, downloads, version, updatedAt, thumbnailUrl, pictureUrl, modCategory.
- Details: `mod(modId, gameId:9293){description}` + `modFiles(modId, gameId:9293){fileId name version category sizeInBytes description}`.
- Download (mit API-Schlüssel im Header `apikey`):
  `GET https://api.nexusmods.com/v1/games/deeprockgalacticroguecore/mods/{mod}/files/{file}/download_link.json`
  – Premium direkt; ohne Premium nur mit `?key=…&expires=…` aus einem `nxm://deeprockgalacticroguecore/mods/{mod}/files/{file}?key=…&expires=…`-Link
  ("Mod Manager Download" auf Nexus). Die App trägt sich dafür unter `HKCU\Software\Classes\nxm` ein;
  ein zweiter Start reicht den Link über `127.0.0.1:47813` an das offene Fenster weiter.
- Konto prüfen: `GET /v1/users/validate.json` → name, is_premium.

### Noch offen / Ideen
Profile (mehrere Mod-Sets), Lade-Reihenfolge, .rar/.7z, Konflikt-Erkennung (gleiche Dateien in zwei Mods),
Auto-Update-Knopf, Favoriten, Seitenleiste einklappen, Englisch als zweite Sprache.
