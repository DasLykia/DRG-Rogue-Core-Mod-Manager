# Behaviour sheet

This is an app (a mod manager), not a game, so there is no movement or weapons. These are the numbers and rules that shape how it works. Sources: evidence ids from the Craft Anything ledger, the original's embedded CSS/JS (`assets/index-qfzktzOi.css`, `assets/index-DHbMZv7z.js`, extracted from `deadlock-mod-manager.exe`), the Nexus API and the game folder.

## Layout
| Value | Number | Source |
|---|---|---|
| Window start size | 1280 × 800 | estimated (original's preview image is 960×540 scaled) |
| Sidebar width | 214 px | estimated from `assets/preview-ngAZ7C1A.png` |
| Top bar height | 44 px | estimated from preview |
| Status bar height | 26 px | estimated from preview |
| Corner radius | 8 px | CSS `--radius: .5rem` |
| Library card | 196 × 238 px, picture 110 px | estimated from preview (6–9 cards per row) |
| Store card | 230 × 272 px, picture 128 px | estimated |
| Store page size | 48 cards, "Mehr laden" adds 48 | estimated |

## Colours (CSS dark theme → RGB)
background 17,15,14 · card 25,22,21 · foreground 252,249,242 · primary 239,224,190 · secondary 45,39,37 · secondary-fg 217,183,104 · muted 38,38,38 · muted-fg 163,163,163 · border 38,38,38 · destructive 127,29,29 · toast 37,33,31 — from `:root`/`.dark` in the CSS.

## Navigation and labels
Sidebar groups "Allgemein" (Downloads, Einstellungen) and "Mods" (Meine Mods, Mods entdecken, Mods hinzufügen): German strings from the original's own German translation object (`myMods:"Meine Mods", getMods:"Mods entdecken", addMods:"Mods hinzufügen"`). Library tabs: original All/Installed/Downloaded/Conflicts → here Alle/Aktiv/Deaktiviert/Updates. Library sort: Installationsreihenfolge (original "Install Order", default in its preview), Name, Zuletzt hinzugefügt.

## Mod rules (Rogue Core)
| Rule | Value | Source |
|---|---|---|
| Mod folder | `<game>\RogueCore\Content\Paks` | Nexus mod 4 install instructions; game folder listing on D: |
| Mod file types | .pak, .utoc, .ucas, .sig | Unreal Engine pak/IoStore conventions; game folder |
| Base game files never touched | `RogueCore-*`, `pakchunk*`, `global.*` | game folder listing |
| Switched off | file renamed to `<name>.disabled` | design choice (Unreal mounts only *.pak) |
| Steam app id | 2605790 | `appmanifest_2605790.acf` |
| Nexus game id / domain | 9293 / deeprockgalacticroguecore | Nexus GraphQL answer |
| Mods on Nexus at build time | 137 | Nexus GraphQL totalCount |
| Page size for the list | 100 | Nexus GraphQL (count) |
| Update available | Nexus `updatedAt` later than install time | design choice |
| Local mod ↔ Nexus link | names equal after dropping versions, spaces and punctuation | design choice |
| Thumbnail downloads in parallel | 6 | estimated |
| Installed game build | `buildid` and `LastUpdated` in `steamapps\appmanifest_2605790.acf` | game folder on D: (buildid 26783375) |
| Latest public build and its date | steamcmd.net `depots.branches.public` (buildid, timeupdated) | live answer: 26783375, 2026-11-19 |
| Game version name | newest Steam announcement titled "PATCH NOTES"/"HOT FIX", e.g. "RC: EA:03:04:00" → "EA 03.04.00" | Steam ISteamNews feed |
| "✔ Aktuell" / "⚠ Evtl. veraltet" | Nexus `updatedAt` on/after vs. before the last public patch date (fallback: local LastUpdated) | design choice; Nexus has no target-game-version field |
| "Neu" label | created less than 7 days ago | design choice |
| Bug/comment counts | not available: GraphQL has no such fields, nexusmods.com answers 403 to the app | checked 2026-11-25 |
| Export file | .zip with `modpack.json` (the library entries) + `files/<mod key>/<file>` | design choice |
| UE4SS on/off | vanilla launch renames `Binaries\Win64\dwmapi.dll` / `xinput1_3.dll` to `*.disabled` (only when `UE4SS.dll` is in `Win64` or `Win64\ue4ss`); modded launch renames back | game folder: UE4SS loads via dwmapi.dll, mods use `enabled.txt` / `mods.txt` |
| Second start | hands nxm link to the open window via 127.0.0.1:47813 | design choice (original uses Tauri single-instance) |
