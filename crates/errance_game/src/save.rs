//! Sauvegarde sur le téléphone (ou le PC) : un seul fichier texte.

use crate::Game;
use bevy::prelude::*;
use errance_core::profile::Profile;
use std::path::PathBuf;

fn dir() -> Option<PathBuf> {
    #[cfg(target_os = "android")]
    {
        bevy::android::ANDROID_APP.get().and_then(|a| a.internal_data_path())
    }
    #[cfg(not(target_os = "android"))]
    {
        if let Ok(d) = std::env::var("ERRANCE_SAVE_DIR") {
            return Some(PathBuf::from(d));
        }
        let home = std::env::var("HOME").ok()?;
        Some(PathBuf::from(home).join(".local/share/errance"))
    }
}

fn path() -> Option<PathBuf> {
    dir().map(|d| d.join("partie.ron"))
}

pub fn save(p: &Profile) {
    if std::env::var("ERRANCE_DEMO").is_ok() {
        return;
    }
    let Some(path) = path() else { return };
    if let Some(d) = path.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let tmp = path.with_extension("tmp");
    if std::fs::write(&tmp, p.to_ron()).is_ok() {
        let _ = std::fs::rename(&tmp, &path);
    }
}

pub fn load() -> Option<Profile> {
    if std::env::var("ERRANCE_DEMO").is_ok() {
        return None;
    }
    let s = std::fs::read_to_string(path()?).ok()?;
    Profile::from_ron(&s)
}

/// Sauvegarde automatique toutes les 20 secondes (et quand le jeu passe en arrière-plan).
pub fn autosave(time: Res<Time>, game: Option<Res<Game>>, mut last: Local<f32>, mut lifecycle: MessageReader<bevy::window::AppLifecycle>) {
    let Some(game) = game else { return };
    let mut now = false;
    for e in lifecycle.read() {
        if matches!(e, bevy::window::AppLifecycle::WillSuspend | bevy::window::AppLifecycle::Suspended) {
            now = true;
        }
    }
    *last += time.delta_secs();
    if now || *last > 20.0 {
        *last = 0.0;
        save(&game.0);
    }
}
