//! App updates with Velopack, from this app's GitHub releases.
use crate::ui::t;
use velopack::{sources::GithubSource, UpdateCheck, UpdateInfo, UpdateManager};

pub const REPO_URL: &str = "https://github.com/DasLykia/DPG-Rogue-Core-Mod-Manager";

fn manager() -> Result<UpdateManager, String> {
    UpdateManager::new(GithubSource::new(REPO_URL, None, false), None, None).map_err(|e| match e {
        velopack::Error::NotInstalled(_) => t(
            "Automatische Updates gibt es nur in der installierten Version (Setup von GitHub).",
            "Automatic updates only work in the installed version (Setup from GitHub).",
        )
        .to_string(),
        e => e.to_string(),
    })
}

/// A newer release on GitHub, if there is one.
pub fn check() -> Result<Option<UpdateInfo>, String> {
    match manager()?.check_for_updates().map_err(|e| e.to_string())? {
        UpdateCheck::UpdateAvailable(u) => Ok(Some(*u)),
        _ => Ok(None),
    }
}

/// Downloads the update, then closes the app, installs it and starts the new version.
pub fn install(update: &UpdateInfo) -> Result<(), String> {
    let um = manager()?;
    um.download_updates(update, None).map_err(|e| e.to_string())?;
    um.apply_updates_and_restart(update).map_err(|e| e.to_string())
}
