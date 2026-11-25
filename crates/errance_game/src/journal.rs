//! Journal du moteur, visible dans le jeu.
//!
//! Sur le téléphone, les erreurs de Bevy / wgpu partent dans le journal
//! système Android (illisible pour Adeh). Ce module les capte (avertissements,
//! erreurs, et la fiche du GPU), les écrit dans `journal.txt` et les affiche
//! en petit en bas de l'écran : une capture suffit pour diagnostiquer.

use bevy::log::BoxedLayer;
use bevy::prelude::*;
use std::fmt::Write as _;
use std::sync::Mutex;
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::{Context, Layer};

/// Messages retenus (sans doublons), dans l'ordre d'arrivée.
static LINES: Mutex<Vec<String>> = Mutex::new(Vec::new());
const MAX_LINES: usize = 10;

/// Bruits connus sans intérêt.
const IGNORE: [&str; 3] = ["XSETTINGS", "only supports software rendering", "ALSA"];

/// Le texte du message, et la vraie cible quand il vient de la crate `log`.
struct Grab(String, Option<String>);

impl Visit for Grab {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        match field.name() {
            "message" => {
                let _ = write!(self.0, "{value:?}");
            }
            "log.target" => self.1 = Some(format!("{value:?}").trim_matches('"').to_string()),
            n if n.starts_with("log.") => {}
            n => {
                let _ = write!(self.0, " {n}={value:?}");
            }
        }
    }
    fn record_str(&mut self, field: &Field, value: &str) {
        match field.name() {
            "message" => self.0.push_str(value),
            "log.target" => self.1 = Some(value.to_string()),
            n if n.starts_with("log.") => {}
            n => {
                let _ = write!(self.0, " {n}={value}");
            }
        }
    }
}

struct JournalLayer;

impl<S: Subscriber> Layer<S> for JournalLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let meta = event.metadata();
        let level = *meta.level();
        let mut g = Grab(String::new(), None);
        event.record(&mut g);
        // La fiche du GPU (niveau INFO) nous intéresse aussi.
        let adapter = g.0.contains("AdapterInfo");
        if !(adapter || level == Level::ERROR || level == Level::WARN) {
            return;
        }
        if IGNORE.iter().any(|x| g.0.contains(x)) {
            return;
        }
        let mut msg = if adapter {
            // On garde l'essentiel : nom, pilote, moteur.
            let pick = |key: &str| g.0.split(&format!("{key}: ")).nth(1).map(|r| r.split(", ").next().unwrap_or("").to_string()).unwrap_or_default();
            format!("GPU {} · pilote {} {} · {}", pick("name"), pick("driver"), pick("driver_info"), pick("backend"))
        } else {
            let tag = if level == Level::ERROR { "ERREUR" } else { "alerte" };
            format!("{tag} [{}] {}", g.1.as_deref().unwrap_or(meta.target()), g.0)
        };
        if msg.chars().count() > 260 {
            msg = msg.chars().take(260).collect::<String>() + "…";
        }
        let Ok(mut lines) = LINES.lock() else { return };
        if lines.iter().any(|l| *l == msg) || lines.len() >= MAX_LINES {
            return;
        }
        lines.push(msg);
        crate::save::write_journal(&lines.join("\n"));
    }
}

/// À brancher dans `LogPlugin::custom_layer`.
pub fn layer(_app: &mut App) -> Option<BoxedLayer> {
    Some(Box::new(JournalLayer))
}

/// Le rapport du dernier plantage (affiché sur l'écran titre).
static CRASH: Mutex<Option<String>> = Mutex::new(None);

pub fn set_crash(s: &str) {
    if let Ok(mut c) = CRASH.lock() {
        *c = Some(s.to_string());
    }
}

/// Tout ce qu'il faut à Claude, en texte brut.
pub fn report() -> String {
    let mut r = format!("Rapport Errance (version {})\nÉtape : {}\n", env!("CARGO_PKG_VERSION"), crate::save::current_step());
    if let Some(c) = CRASH.lock().ok().and_then(|c| c.clone()) {
        let _ = writeln!(r, "Dernier plantage : {c}");
    }
    let _ = writeln!(r, "Moteur choisi : {}", crate::save::backend_label(&crate::save::backend()));
    if let Ok(lines) = LINES.lock() {
        if lines.is_empty() {
            r.push_str("(aucune erreur ni alerte)\n");
        }
        for l in lines.iter() {
            let _ = writeln!(r, "{l}");
        }
    }
    r
}

#[derive(Component)]
struct JournalText;

pub struct JournalPlugin;

impl Plugin for JournalPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_journal).add_systems(Update, (show_journal, copy_action));
    }
}

fn spawn_journal(mut commands: Commands, fonts: Res<crate::ui::Fonts>) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(4),
                bottom: px(2),
                max_width: percent(70),
                padding: UiRect::all(px(3)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexStart,
                row_gap: px(2),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.75)),
            GlobalZIndex(200),
            Visibility::Hidden,
            JournalText,
            // Ne bloque pas les doigts : les boutons dessous restent utilisables.
            bevy::picking::Pickable::IGNORE,
        ))
        .with_children(|p| {
            p.spawn(crate::ui::button_colored(&fonts, "Copier le journal pour Claude", crate::ui::UiAction::CopyJournal, px(190), 10.0, Color::srgb(0.25, 0.35, 0.55)));
            p.spawn((Text::new(""), crate::ui::font(&fonts.regular, 8.0), TextColor(Color::srgb(1.0, 0.75, 0.6)), bevy::picking::Pickable::IGNORE));
        });
}

fn copy_action(mut queue: ResMut<crate::ui::UiQueue>, mut toasts: ResMut<crate::Toasts>) {
    if !queue.0.contains(&crate::ui::UiAction::CopyJournal) {
        return;
    }
    queue.0.retain(|a| *a != crate::ui::UiAction::CopyJournal);
    toasts.push(crate::clipboard::copy(&report()));
}

fn show_journal(mut seen: Local<usize>, mut q: Query<(&mut Visibility, &Children), With<JournalText>>, mut texts: Query<&mut Text>) {
    let Ok(lines) = LINES.lock() else { return };
    // Toujours visible dès qu'il y a une ligne : même « rien que le GPU » est une info.
    let interesting = !lines.is_empty();
    if lines.len() == *seen {
        return;
    }
    *seen = lines.len();
    let Ok((mut v, children)) = q.single_mut() else { return };
    *v = if interesting { Visibility::Inherited } else { Visibility::Hidden };
    if let Some(mut t) = children.last().and_then(|c| texts.get_mut(*c).ok()) {
        **t = lines.join("\n");
    }
}
