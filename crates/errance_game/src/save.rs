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

// ------------------------------------------------------------------
// Moteur graphique (choisi au lancement) : OpenGL ES ou Vulkan.
// ------------------------------------------------------------------

/// "gl" ou "vulkan". Par défaut : choix automatique (Vulkan sur Android ;
/// OpenGL plantait au démarrage sur le Redmi d'Adeh).
pub fn backend() -> String {
    if let Some(s) = black_box("moteur.txt").and_then(|p| std::fs::read_to_string(p).ok()) {
        let s = s.trim();
        if s == "gl" || s == "vulkan" {
            return s.to_string();
        }
    }
    "auto".into()
}

/// Bascule le moteur (pris en compte au prochain lancement). Renvoie le nouveau.
pub fn toggle_backend() -> String {
    let next = if backend() == "gl" { "vulkan" } else { "gl" };
    if let Some(p) = black_box("moteur.txt") {
        if let Some(d) = p.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let _ = std::fs::write(p, next);
    }
    next.to_string()
}

/// Garde-fou : si le lancement précédent n'a jamais atteint l'écran titre
/// (plantage du moteur graphique au démarrage), on change de moteur tout seul.
pub fn startup_guard() {
    let last = black_box("etape.txt").and_then(|p| std::fs::read_to_string(p).ok());
    if let Ok(mut prev) = PREV_STEP.lock() {
        *prev = last.clone();
    }
    if last.as_deref() == Some("démarrage") {
        let b = toggle_backend();
        if let Some(p) = black_box("plantage.txt") {
            let old = std::fs::read_to_string(&p).unwrap_or_default();
            let msg = format!("{old}\nLe jeu n'arrivait pas à démarrer : moteur graphique changé pour {}.", backend_label(&b));
            let _ = std::fs::write(p, msg.trim());
        }
    }
    mark("démarrage");
}

pub fn backend_label(b: &str) -> &'static str {
    match b {
        "gl" => "OpenGL ES",
        "vulkan" => "Vulkan",
        _ => "automatique",
    }
}

// ------------------------------------------------------------------
// Boîte noire : en cas de plantage sur le téléphone, on garde une trace
// (message de panique Rust + dernière étape atteinte) pour l'afficher au
// prochain lancement sur l'écran titre.
// ------------------------------------------------------------------

static STEP: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
/// L'étape où en était le lancement précédent (lue par `startup_guard`).
static PREV_STEP: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

fn black_box(name: &str) -> Option<PathBuf> {
    dir().map(|d| d.join(name))
}

/// Journal du moteur (erreurs Bevy / wgpu), réécrit à chaque nouvelle ligne.
pub fn write_journal(s: &str) {
    write_named("journal.txt", s);
}

/// Écrit un petit fichier texte dans le dossier de sauvegarde.
pub fn write_named(name: &str, s: &str) {
    if let Some(p) = black_box(name) {
        if let Some(d) = p.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let _ = std::fs::write(p, s);
    }
}

/// L'étape en cours (pour le rapport copié).
pub fn current_step() -> String {
    STEP.lock().map(|s| s.clone()).unwrap_or_default()
}

/// Note l'étape en cours (écrite sur disque : survit à un plantage du pilote).
pub fn mark(step: &str) {
    if let Ok(mut s) = STEP.lock() {
        if *s == step {
            return;
        }
        *s = step.to_string();
    }
    if let Some(p) = black_box("etape.txt") {
        if let Some(d) = p.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let _ = std::fs::write(p, step);
    }
}

/// Étapes après lesquelles un arrêt est normal.
const CALM: [&str; 2] = ["titre", "arrière-plan"];

/// À appeler au tout début : une panique Rust est écrite dans `plantage.txt`.
pub fn install_panic_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let step = STEP.lock().map(|s| s.clone()).unwrap_or_default();
        let msg = info.payload().downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| info.payload().downcast_ref::<String>().cloned()).unwrap_or_default();
        let at = info.location().map(|l| format!("{}:{}", l.file(), l.line())).unwrap_or_default();
        let thread = std::thread::current().name().unwrap_or("?").to_string();
        if let Some(p) = black_box("plantage.txt") {
            let _ = std::fs::write(p, format!("{msg}\n({at}, fil « {thread} », étape : {step})"));
        }
        default(info);
    }));
}

/// Le rapport du dernier plantage, s'il y en a eu un (puis on l'efface).
pub fn take_crash_report() -> Option<String> {
    let panic = black_box("plantage.txt").and_then(|p| {
        let s = std::fs::read_to_string(&p).ok();
        let _ = std::fs::remove_file(p);
        s
    });
    let step = PREV_STEP.lock().ok().and_then(|s| s.clone());
    if let Some(m) = panic {
        return Some(format!("Le jeu a planté la dernière fois : {m}"));
    }
    match step {
        Some(s) if !CALM.contains(&s.as_str()) => Some(format!("Le jeu s'est arrêté la dernière fois sans message, pendant l'étape : {s}. (Plantage du pilote graphique ou du son, ou fermeture forcée.)")),
        _ => None,
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
        let before = STEP.lock().map(|s| s.clone()).unwrap_or_default();
        mark("sauvegarde automatique");
        save(&game.0);
        mark(if now { "arrière-plan" } else { &before });
    }
}
