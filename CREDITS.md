# Credits and sources

## Files from the original used in this build

None. The app bundles no image, font, sound or code file from the original program or the game.

What was taken from the original Deadlock Mod Manager 2.3.0 (read from `deadlock-mod-manager.exe`, unpacked from the `Deadlock.Mod.Manager_2.3.0_x64-setup.exe` installer, embedded `assets/index-qfzktzOi.css` and `assets/index-DHbMZv7z.js`) is only *values*, re-implemented in Rust:

- theme colours (`--background`, `--card`, `--primary`, `--secondary`, `--muted`, `--border`, `--destructive`, `--toast-background`, `--sidebar-border`) and the 0.5 rem corner radius;
- layout of the main screen (sidebar groups, top launch bar, library cards with on/off switch, status bar), seen in the embedded `assets/preview-ngAZ7C1A.png`;
- the German menu wording from its own German translation ("Meine Mods", "Mods entdecken", "Mods hinzufügen", "Downloads", "Einstellungen").

The original's logo, name, Geist font and artwork are not used.

## App icon

ssets/icon.png – the logo the person provided for this app (cut to its round badge in code; ssets/app.ico is made from it at build time).

## Data at run time

- Mod list, pictures, descriptions and files: Nexus Mods (https://www.nexusmods.com), public GraphQL API v2 and REST API v1. Pictures are cached in `%APPDATA%\CoreModManager\thumbs`.
- CJK text is drawn with fonts already installed in Windows (Microsoft YaHei, Yu Gothic, Malgun Gothic) when present; they are read from `C:\Windows\Fonts`, not copied.

## Libraries

eframe/egui (MIT/Apache-2.0), image, ureq, native-tls, serde, serde_json, zip, rfd.
