//! Errance — le jeu (Bevy). Affichage 3D, tactile et interface ; toutes les
//! règles vivent dans `errance_core`.

pub mod audio;
pub mod field;
pub mod gather;
pub mod hud;
pub mod menus;
pub mod minigames;
pub mod models;
pub mod save;
pub mod town;
pub mod ui;
pub mod workshop;

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
#[cfg(target_os = "android")]
use bevy::window::WindowMode;
use errance_core::content::content;
use errance_core::profile::Profile;
use ui::{Fonts, UiAction, UiQueue, pal};

#[derive(States, Default, Clone, Eq, PartialEq, Hash, Debug)]
pub enum AppState {
    #[default]
    Title,
    Town,
    Field,
}

/// La partie en cours.
#[derive(Resource)]
pub struct Game(pub Profile);

/// Panneaux qui s'ouvrent par-dessus le jeu.
#[derive(Clone, Debug, PartialEq)]
pub enum Panel {
    Bag,
    Character,
    Atlas,
    Summary,
    Chores,
    Chore(String),
    Market,
    Tavern,
    Npc(u32),
    Gate,
    Inn,
    Company(u32),
    Intro,
    Workshop,
}

#[derive(Resource, Default)]
pub struct Overlay(pub Option<Panel>);

/// Petits messages en haut de l'écran.
#[derive(Resource, Default)]
pub struct Toasts {
    pub items: Vec<(String, f32)>,
}

impl Toasts {
    pub fn push(&mut self, s: String) {
        self.items.push((s, 0.0));
        if self.items.len() > 4 {
            self.items.remove(0);
        }
    }
}

#[derive(Component)]
struct ToastBox;

#[bevy_main]
pub fn main() {
    let window = Window {
        title: "Errance".into(),
        #[cfg(target_os = "android")]
        mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
        #[cfg(not(target_os = "android"))]
        resolution: (1280u32, 640u32).into(),
        ..default()
    };
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin { primary_window: Some(window), ..default() }))
        .insert_resource(ClearColor(Color::srgb(0.08, 0.07, 0.06)))
        .insert_resource(GlobalAmbientLight { brightness: 420.0, ..default() })
        .init_state::<AppState>()
        .init_resource::<Overlay>()
        .init_resource::<Toasts>()
        .add_plugins((audio::AudioPlugin, ui::UiPlugin, field::FieldPlugin, hud::HudPlugin, town::TownPlugin, menus::MenusPlugin, minigames::MiniGamesPlugin, gather::GatherPlugin))
        .add_systems(Startup, (setup_lib, setup_ui_camera))
        .add_systems(OnEnter(AppState::Title), build_title)
        .add_systems(Update, (title_actions.run_if(in_state(AppState::Title)), show_toasts, auto_screenshot, demo_driver, save::autosave))
        .run();
}

fn setup_lib(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.insert_resource(models::Lib::new(&mut meshes));
}

/// Caméra d'interface (au-dessus de tout, sans effacer la 3D).
fn setup_ui_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera { order: 10, clear_color: ClearColorConfig::None, ..default() },
        IsDefaultUiCamera,
    ));
}

#[derive(Component)]
struct TitleScreen;

fn build_title(mut commands: Commands, fonts: Res<Fonts>) {
    let has_save = save::load().is_some();
    commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: px(10),
                ..default()
            },
            BackgroundColor(Color::srgb(0.07, 0.06, 0.05)),
            TitleScreen,
            DespawnOnExit(AppState::Title),
        ))
        .with_children(|p| {
            p.spawn(ui::title(&fonts, "ERRANCE", 54.0, pal::GOLD));
            p.spawn(ui::label(&fonts, "Vous n'êtes personne. Pas encore.", 15.0, pal::DIM));
            p.spawn(Node { height: px(18), ..default() });
            if has_save {
                p.spawn(ui::button(&fonts, "Continuer", UiAction::Continue, px(220), 15.0));
            }
            p.spawn(ui::button(&fonts, "Nouvelle partie", UiAction::NewGame, px(220), 15.0));
            p.spawn(Node { height: px(18), ..default() });
            p.spawn(ui::label(&fonts, "Joystick à gauche · skills à droite · tenez le téléphone à l'horizontale", 11.0, pal::DIM));
        });
}

fn title_actions(mut commands: Commands, mut queue: ResMut<UiQueue>, mut next: ResMut<NextState<AppState>>) {
    for a in queue.take() {
        match a {
            UiAction::NewGame => {
                let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(7);
                let p = Profile::new_game(content(), "Vous", seed);
                commands.insert_resource(Game(p));
                next.set(AppState::Town);
            }
            UiAction::Continue => {
                if let Some(p) = save::load() {
                    commands.insert_resource(Game(p));
                    next.set(AppState::Town);
                }
            }
            _ => {}
        }
    }
}

fn show_toasts(mut commands: Commands, time: Res<Time>, mut toasts: ResMut<Toasts>, fonts: Res<Fonts>, boxes: Query<Entity, With<ToastBox>>, mut last_len: Local<usize>) {
    let dt = time.delta_secs();
    let before = toasts.items.len();
    for t in toasts.items.iter_mut() {
        t.1 += dt;
    }
    toasts.items.retain(|t| t.1 < 3.2);
    let changed = toasts.is_changed() && (toasts.items.len() != *last_len || before != toasts.items.len() || toasts.items.iter().any(|t| t.1 <= dt));
    if !changed && toasts.items.len() == *last_len {
        return;
    }
    *last_len = toasts.items.len();
    for e in boxes.iter() {
        commands.entity(e).despawn();
    }
    if toasts.items.is_empty() {
        return;
    }
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(62),
                left: percent(50),
                margin: UiRect::left(px(-160)),
                width: px(320),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(2),
                ..default()
            },
            ToastBox,
            GlobalZIndex(60),
        ))
        .with_children(|p| {
            for (t, _) in &toasts.items {
                p.spawn((
                    Node { padding: UiRect::axes(px(8), px(2)), border_radius: BorderRadius::all(px(4)), ..default() },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
                ))
                .with_child((Text::new(t.clone()), ui::font(&fonts.bold, 12.0), TextColor(pal::INK)));
            }
        });
}

/// Outil de test : ERRANCE_SCREENSHOT=chemin.png [ERRANCE_FRAMES=n] → capture puis quitte.
fn auto_screenshot(mut commands: Commands, mut frame: Local<u32>, mut exit: MessageWriter<AppExit>) {
    let Ok(path) = std::env::var("ERRANCE_SCREENSHOT") else { return };
    let at: u32 = std::env::var("ERRANCE_FRAMES").ok().and_then(|s| s.parse().ok()).unwrap_or(90);
    *frame += 1;
    if *frame == at {
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
    }
    if *frame == at + 20 {
        exit.write(AppExit::Success);
    }
}

/// Outil de test : ERRANCE_DEMO=town|field:<zone>|bag|... → saute directement à un écran.
#[allow(clippy::too_many_arguments)]
fn demo_driver(
    mut commands: Commands,
    mut frame: Local<u32>,
    state: Res<State<AppState>>,
    mut next: ResMut<NextState<AppState>>,
    game: Option<ResMut<Game>>,
    mut overlay: ResMut<Overlay>,
    field: Option<ResMut<field::Field>>,
    mut stick: ResMut<hud::Stick>,
    mut queue: ResMut<UiQueue>,
) {
    let Ok(demo) = std::env::var("ERRANCE_DEMO") else { return };
    *frame += 1;
    let parts: Vec<&str> = demo.split(':').collect();
    if *frame == 3 && *state.get() == AppState::Title {
        let mut p = Profile::new_game(content(), "Vous", 42);
        let c = content();
        if parts.contains(&"stuff") {
            // Un héros déjà équipé, pour voir des choses.
            for l in ["givre", "plaque", "labeur", "epee", "sacre"] {
                p.hero.masteries.add_fame(l, errance_core::mastery::total_fame_for(25) + 1.0);
            }
            for (b, t) in [("baton_givre", 3), ("casque_cornes", 3), ("cotte", 3), ("solerets", 3), ("cape_guerre", 3), ("potion_soin", 2)] {
                let it = p.new_item(b, t);
                let _ = p.hero.equip(c, it);
            }
            for b in ["epee_longue", "bouclier_rond", "robe", "capuche", "baton_sacre"] {
                let it = p.new_item(b, 2);
                p.add_to_bag(c, it);
            }
            p.gold = 137;
        }
        if parts.contains(&"res") {
            // De quoi forger quelques objets.
            for (r, t, n) in [("minerai", 1, 14), ("bois", 1, 9), ("peau", 1, 5), ("fibre", 2, 3), ("minerai", 2, 4)] {
                errance_core::craft::stack_add(&mut p.resources, r, t, n);
            }
            p.gold = p.gold.max(60);
        }
        if parts.contains(&"party") || parts.contains(&"companion") {
            let ids: Vec<u32> = p.world.tavern().iter().take(2).map(|n| n.id).collect();
            for id in &ids {
                if let Some(n) = p.world.npc_mut(*id) {
                    if parts.contains(&"companion") {
                        n.companion = true;
                    } else {
                        n.hired_for = 1;
                    }
                }
            }
        }
        commands.insert_resource(Game(p));
        next.set(AppState::Town);
    }
    if *frame == 6 {
        if let Some(g) = &game {
            match parts.first().copied() {
                Some("field") => {
                    let zone = parts.get(1).copied().unwrap_or("marches").to_string();
                    let party = g.0.world.party().iter().map(|n| n.id).collect();
                    commands.insert_resource(field::Expedition { zone, party, porter_for: None });
                    next.set(AppState::Field);
                }
                Some("bag") => overlay.0 = Some(Panel::Bag),
                Some("character") => overlay.0 = Some(Panel::Character),
                Some("atlas") => overlay.0 = Some(Panel::Atlas),
                Some("tavern") => overlay.0 = Some(Panel::Tavern),
                Some("gate") => overlay.0 = Some(Panel::Gate),
                Some("chores") => overlay.0 = Some(Panel::Chores),
                Some("market") => overlay.0 = Some(Panel::Market),
                Some("workshop") => overlay.0 = Some(Panel::Workshop),
                Some("company") => {
                    if let Some(co) = g.0.world.companies.first() {
                        queue.0.push(UiAction::OpenCompany(co.id));
                    }
                }
                Some("porter") => {
                    if let Some(co) = g.0.world.companies.first() {
                        queue.0.push(UiAction::JoinCompany(co.id));
                    }
                }
                Some("companion") => {
                    if let Some(n) = g.0.world.party().first() {
                        let id = n.id;
                        queue.0.push(UiAction::InspectNpc(id));
                        queue.0.push(UiAction::Tab(1));
                    }
                }
                Some("chore") => queue.0.push(UiAction::StartChore(parts.get(1).copied().unwrap_or("caisses_du_port").to_string())),
                _ => {}
            }
        }
    }
    // En expédition : le héros est piloté par l'IA (pour les captures).
    if let Some(mut f) = field {
        if parts.contains(&"brawl") && *frame == 12 {
            // Amène les ennemis les plus proches devant le héros.
            let hp = f.combat.get(f.hero).map(|h| h.pos).unwrap_or_default();
            let mut ids: Vec<(u32, f32)> = f.combat.fighters.iter().filter(|x| x.team == errance_core::combat::Team::Enemy).map(|x| (x.id, x.pos.distance(hp))).collect();
            ids.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            for (k, (id, _)) in ids.iter().take(3).enumerate() {
                if let Some(e) = f.combat.get_mut(*id) {
                    e.pos = hp + errance_core::math::Vec2::new(-2.0 + k as f32 * 2.0, -5.0);
                    e.home = e.pos;
                }
            }
        }
        if parts.contains(&"auto") {
            let hero = f.hero;
            if let Some(h) = f.combat.get_mut(hero) {
                if h.controller == errance_core::combat::Controller::Player {
                    h.controller = errance_core::combat::Controller::Companion { rules: errance_core::rules::default_rules() };
                }
            }
            // Va vers l'ennemi le plus proche.
            let hp = f.combat.get(hero).map(|h| h.pos).unwrap_or_default();
            let t = f.combat.fighters.iter().filter(|x| x.alive && x.team == errance_core::combat::Team::Enemy)
                .min_by(|a, b| a.pos.distance(hp).partial_cmp(&b.pos.distance(hp)).unwrap()).map(|x| x.id);
            f.combat.set_target(hero, t);
            if let Some(tid) = t {
                if let Some(e) = f.combat.get_mut(tid) {
                    if e.threat.is_empty() {
                        e.threat.push((hero, 1.0));
                    }
                }
            }
        }
    }
    let _ = &mut stick;
}
