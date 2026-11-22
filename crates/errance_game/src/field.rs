//! L'expédition : une zone en 3D où l'on se bat.
//!
//! Le moteur de combat (errance_core) décide de tout ; ici on dessine,
//! on transmet les gestes et on fabrique les effets visuels.

use crate::hud::Stick;
use crate::models::{AnimState, Finish, Lib, Limb, Painter, animate_limbs, col, spawn_look, spawn_rock, spawn_tree};
use crate::ui::{Fonts, UiAction, UiQueue, pal};
use crate::{AppState, Game, Overlay, Toasts};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use errance_core::character::item_name;
use errance_core::combat::{Combat, Controller, Event as CEvent, FighterId, Team, ZoneShape};
use errance_core::content::content;
use errance_core::expedition::{Decor, Layout, layout_zone, setup_combat};
use errance_core::item::Item;
use errance_core::look::Rgb;
use errance_core::loot::enemy_drop;
use errance_core::math::Vec2 as V2;
use errance_core::rng::Rng;
use errance_core::skill::{DamageKind, StatusKind, Tag};

/// Ce que le joueur a choisi en quittant la ville.
#[derive(Resource, Clone)]
pub struct Expedition {
    pub zone: String,
    /// Aventuriers qui vous accompagnent (ids de PNJ).
    pub party: Vec<u32>,
    /// Si vous êtes porteur pour une compagnie : son id.
    pub porter_for: Option<u32>,
}

#[derive(Resource)]
pub struct Field {
    pub combat: Combat,
    pub hero: FighterId,
    pub views: HashMap<FighterId, Entity>,
    pub npc_of: HashMap<FighterId, u32>,
    pub fame_seen: HashMap<FighterId, f32>,
    pub loot: Vec<Item>,
    pub gold: u32,
    pub ground: Vec<GroundLoot>,
    pub exit: V2,
    pub end: Option<End>,
    pub cleared_announced: bool,
    pub zone_name: String,
    pub tier: u8,
    pub kills: u32,
    pub rng: Rng,
    pub used_potion: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum End {
    Defeat,
    Return,
}

pub struct GroundLoot {
    pub pos: V2,
    pub gold: u32,
    pub items: Vec<Item>,
    pub entity: Entity,
}

#[derive(Component)]
pub struct MainCamera;

#[derive(Component)]
pub struct FighterView(pub FighterId);

#[derive(Component)]
struct ProjView(u32);

#[derive(Component)]
struct ZoneView {
    id: u32,
    fill: Entity,
}

#[derive(Component)]
struct Spark {
    vel: Vec3,
    life: f32,
}

#[derive(Component)]
struct Flash {
    life: f32,
    max: f32,
    grow: f32,
}

#[derive(Component)]
pub struct FloatText {
    pub world: Vec3,
    pub age: f32,
    pub life: f32,
}

#[derive(Component)]
struct TargetRing;

#[derive(Component)]
struct HeroRing;

#[derive(Component)]
struct LootView;

#[derive(Component)]
struct ExitMarker;

pub struct FieldPlugin;

impl Plugin for FieldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Field), setup_field)
            .add_systems(OnExit(AppState::Field), cleanup_field)
            .add_systems(
                Update,
                (field_input, step_combat, sync_views, update_vfx, camera_follow, pickup_loot, float_texts)
                    .chain()
                    .run_if(in_state(AppState::Field)),
            );
    }
}

fn v3(p: V2, y: f32) -> Vec3 {
    Vec3::new(p.x, y, p.y)
}

fn kind_color(k: Option<DamageKind>) -> Rgb {
    match k {
        Some(DamageKind::Fire) => (1.0, 0.5, 0.15),
        Some(DamageKind::Frost) => (0.5, 0.85, 1.0),
        Some(DamageKind::Arcane) => (0.75, 0.45, 1.0),
        Some(DamageKind::Holy) => (1.0, 0.9, 0.5),
        Some(DamageKind::Nature) => (0.45, 0.95, 0.4),
        Some(DamageKind::Shadow) => (0.6, 0.25, 0.75),
        Some(DamageKind::True) => (1.0, 1.0, 1.0),
        _ => (0.95, 0.9, 0.8),
    }
}

fn tags_color(tags: &[Tag]) -> Rgb {
    for t in tags {
        let c = match t {
            Tag::Fire | Tag::Burn => Some((1.0, 0.5, 0.15)),
            Tag::Frost => Some((0.5, 0.85, 1.0)),
            Tag::Arcane => Some((0.75, 0.45, 1.0)),
            Tag::Holy => Some((1.0, 0.9, 0.5)),
            Tag::Heal => Some((0.45, 1.0, 0.5)),
            Tag::Nature | Tag::Poison => Some((0.45, 0.9, 0.3)),
            Tag::Shadow => Some((0.6, 0.25, 0.75)),
            Tag::Taunt => Some((1.0, 0.3, 0.2)),
            Tag::Shield | Tag::Defense => Some((0.8, 0.85, 1.0)),
            _ => None,
        };
        if let Some(c) = c {
            return c;
        }
    }
    (1.0, 0.95, 0.85)
}

#[allow(clippy::too_many_arguments)]
pub fn setup_field(
    mut commands: Commands,
    exp: Res<Expedition>,
    mut game: ResMut<Game>,
    mut lib: ResMut<Lib>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut clear: ResMut<ClearColor>,
    mut toasts: ResMut<Toasts>,
) {
    let c = content();
    let zone = c.zone(&exp.zone).unwrap_or(&c.zones[0]).clone();
    let seed = game.0.rng.next_u64();
    let mut rng = Rng::new(seed);
    let layout: Layout = layout_zone(&zone, &mut rng);
    let p = &mut game.0;
    let hero_spec = p.hero.fighter_spec(c, Team::Ally, Controller::Player);
    let mut party_specs = vec![];
    let mut party_ids = vec![];
    for id in &exp.party {
        if let Some(n) = p.world.npc(*id) {
            party_specs.push(n.sheet.fighter_spec(c, Team::Ally, Controller::Companion { rules: n.sheet.rules.clone() }));
            party_ids.push(*id);
        }
    }
    let (combat, hero, fids) = setup_combat(c, &layout, hero_spec, party_specs, seed);
    let mut npc_of = HashMap::default();
    for (fid, nid) in fids.iter().zip(party_ids.iter()) {
        npc_of.insert(*fid, *nid);
    }
    p.counters.expeditions += 1;
    clear.0 = col(zone.sky);

    let mut painter = Painter { lib: &mut lib, mats: &mut mats };
    let scope = DespawnOnExit(AppState::Field);
    // Sol.
    let size = layout.half * 2.0 + 40.0;
    let ground = meshes.add(Plane3d::default().mesh().size(size, size));
    let gm = painter.mat(zone.ground, Finish::Matte);
    commands.spawn((Mesh3d(ground), MeshMaterial3d(gm), Transform::default(), scope.clone()));
    // Taches d'herbe / de terre pour casser l'uniformité.
    for _ in 0..120 {
        let x = rng.range_f32(-layout.half - 8.0, layout.half + 8.0);
        let z = rng.range_f32(-layout.half - 8.0, layout.half + 8.0);
        let k = rng.range_f32(0.85, 1.15);
        let cc = (zone.ground.0 * k, zone.ground.1 * k, zone.ground.2 * k);
        let disc = painter.lib.disc.clone();
        let m = painter.mat(cc, Finish::Matte);
        let r = rng.range_f32(1.0, 3.5);
        commands.spawn((
            Mesh3d(disc),
            MeshMaterial3d(m),
            Transform::from_xyz(x, 0.005 + rng.f32() * 0.004, z)
                .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
                .with_scale(Vec3::new(r, r * rng.range_f32(0.6, 1.0), 1.0)),
            scope.clone(),
        ));
    }
    // Touffes d'herbe.
    for _ in 0..260 {
        let x = rng.range_f32(-layout.half, layout.half);
        let z = rng.range_f32(-layout.half, layout.half);
        let cone = painter.lib.cone4.clone();
        let m = painter.mat(zone.foliage, Finish::Matte);
        let h = rng.range_f32(0.15, 0.35);
        commands.spawn((Mesh3d(cone), MeshMaterial3d(m), Transform::from_xyz(x, h / 2.0, z).with_scale(Vec3::new(0.12, h, 0.12)), scope.clone()));
    }
    // Décor.
    for (pos, _, kind, scale) in &layout.decor {
        let rot = rng.range_f32(0.0, 6.28);
        let e = match kind {
            Decor::Tree => spawn_tree(&mut painter, &mut commands, v3(*pos, 0.0), &zone.tree_kind, zone.foliage, *scale, rot),
            Decor::Rock => spawn_rock(&mut painter, &mut commands, v3(*pos, 0.0), zone.rock, Vec3::new(1.1, 0.8, 1.0) * *scale, rot),
        };
        commands.entity(e).insert(scope.clone());
    }
    // Sortie (poteau + feu de camp) au point de départ.
    let exit = layout.start + V2::new(2.5, 1.5);
    let post = commands.spawn((Transform::from_translation(v3(exit, 0.0)), Visibility::default(), ExitMarker, scope.clone())).id();
    painter.cube(&mut commands, post, (0.4, 0.28, 0.17), Finish::Matte, Vec3::new(0.0, 0.8, 0.0), Vec3::new(0.12, 1.6, 0.12));
    painter.cube(&mut commands, post, (0.6, 0.45, 0.25), Finish::Matte, Vec3::new(0.25, 1.35, 0.0), Vec3::new(0.6, 0.22, 0.06));
    for i in 0..5 {
        let a = i as f32 * 1.256;
        painter.cube(&mut commands, post, (0.35, 0.25, 0.15), Finish::Matte, Vec3::new(-0.9 + a.cos() * 0.3, 0.06, a.sin() * 0.3), Vec3::new(0.5, 0.08, 0.08));
    }
    painter.cone(&mut commands, post, (1.0, 0.55, 0.15), Finish::Glow, Transform::from_xyz(-0.9, 0.3, 0.0).with_scale(Vec3::new(0.35, 0.5, 0.35)));
    // Lumières.
    commands.spawn((
        DirectionalLight {
            illuminance: 11000.0,
            color: Color::srgb(1.0, 0.95, 0.85),
            #[cfg(not(target_os = "android"))]
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(8.0, 18.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
        scope.clone(),
    ));
    // Caméra.
    let start3 = v3(layout.start, 0.0);
    commands.spawn((
        Camera3d::default(),
        MainCamera,
        Projection::Perspective(PerspectiveProjection { fov: 38f32.to_radians(), ..default() }),
        Transform::from_translation(start3 + Vec3::new(0.0, 13.0, 9.5)).looking_at(start3, Vec3::Y),
        DistanceFog {
            color: col(zone.sky),
            falloff: FogFalloff::Linear { start: 22.0, end: 48.0 },
            ..default()
        },
        #[cfg(target_os = "android")]
        Msaa::Off,
        scope.clone(),
    ));
    // Anneaux de sélection.
    let ring = painter.lib.ring.clone();
    let red = painter.mat((1.0, 0.3, 0.25), Finish::Ghost);
    commands.spawn((
        Mesh3d(ring.clone()),
        MeshMaterial3d(red),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        TargetRing,
        Visibility::Hidden,
        scope.clone(),
    ));
    let gold = painter.mat((1.0, 0.85, 0.4), Finish::Ghost);
    commands.spawn((
        Mesh3d(ring),
        MeshMaterial3d(gold),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)).with_scale(Vec3::splat(0.55)),
        HeroRing,
        scope.clone(),
    ));
    // Combattants.
    let mut views = HashMap::default();
    for f in &combat.fighters {
        let root = spawn_look(&mut painter, &mut commands, &f.look, v3(f.pos, 0.0));
        commands.entity(root).insert((FighterView(f.id), scope.clone()));
        views.insert(f.id, root);
    }
    toasts.push(format!("{} — Tier {}", zone.name, zone.tier));
    if exp.porter_for.is_some() {
        toasts.push("Vous portez la torche. Restez derrière.".to_string());
    }
    commands.insert_resource(Field {
        combat,
        hero,
        views,
        npc_of,
        fame_seen: HashMap::default(),
        loot: vec![],
        gold: 0,
        ground: vec![],
        exit,
        end: None,
        cleared_announced: false,
        zone_name: zone.name.clone(),
        tier: zone.tier,
        kills: 0,
        rng,
        used_potion: false,
    });
}

fn cleanup_field(mut commands: Commands, mut clear: ResMut<ClearColor>) {
    commands.remove_resource::<Field>();
    clear.0 = Color::srgb(0.08, 0.07, 0.06);
}

/// Gestes : joystick, boutons, tap sur un ennemi.
#[allow(clippy::too_many_arguments)]
fn field_input(
    mut field: ResMut<Field>,
    stick: Res<Stick>,
    keys: Res<ButtonInput<KeyCode>>,
    mut queue: ResMut<UiQueue>,
    mut next: ResMut<NextState<AppState>>,
    mut ov: ResMut<Overlay>,
    touches: Res<bevy::input::touch::Touches>,
    windows: Query<&Window>,
    cam: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut toasts: ResMut<Toasts>,
    mut game: ResMut<Game>,
    mut menu: ResMut<crate::menus::MenuState>,
) {
    let hero = field.hero;
    let mut dir = stick.dir;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::KeyZ) || keys.pressed(KeyCode::ArrowUp) {
        dir.y -= 1.0;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        dir.y += 1.0;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::KeyQ) || keys.pressed(KeyCode::ArrowLeft) {
        dir.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        dir.x += 1.0;
    }
    let d = V2::new(dir.x, dir.y).clamp_length(1.0);
    field.combat.set_move(hero, if ov.0.is_some() { V2::ZERO } else { d });
    let key_slots = [
        (KeyCode::Digit1, errance_core::skill::SkillSlot::Weapon1),
        (KeyCode::Digit2, errance_core::skill::SkillSlot::Weapon2),
        (KeyCode::Digit3, errance_core::skill::SkillSlot::Weapon3),
        (KeyCode::Digit4, errance_core::skill::SkillSlot::Head),
        (KeyCode::Digit5, errance_core::skill::SkillSlot::Chest),
        (KeyCode::Digit6, errance_core::skill::SkillSlot::Feet),
        (KeyCode::Digit7, errance_core::skill::SkillSlot::Potion),
    ];
    for (k, s) in key_slots {
        if keys.just_pressed(k) {
            field.combat.request_skill(hero, s);
        }
    }
    if keys.just_pressed(KeyCode::Tab) {
        field.combat.cycle_target(hero);
    }
    // Tap / clic sur un ennemi : le cibler.
    let (camera, cam_t) = *cam;
    let mut taps: Vec<bevy::math::Vec2> = touches
        .iter_just_released()
        .filter(|t| t.position().distance(t.start_position()) < 14.0 && Some(t.id()) != stick.touch)
        .map(|t| t.position())
        .collect();
    if mouse.just_pressed(MouseButton::Right) {
        if let Some(p) = windows.single().ok().and_then(|w| w.cursor_position()) {
            taps.push(p);
        }
    }
    for tap in taps {
        if let Ok(ray) = camera.viewport_to_world(cam_t, tap) {
            if let Some(dist) = ray.intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y)) {
                let w = ray.get_point(dist);
                let pt = V2::new(w.x, w.z);
                let best = field
                    .combat
                    .fighters
                    .iter()
                    .filter(|f| f.alive && f.team == Team::Enemy)
                    .map(|f| (f.id, f.pos.distance(pt)))
                    .filter(|x| x.1 < 1.8)
                    .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
                if let Some((id, _)) = best {
                    field.combat.set_target(hero, Some(id));
                }
            }
        }
    }
    let actions = queue.take();
    let mut keep = vec![];
    for a in actions {
        match a {
            UiAction::Skill(s) => field.combat.request_skill(hero, s),
            UiAction::CycleTarget => field.combat.cycle_target(hero),
            UiAction::SetOrder(o) => {
                field.combat.order = o;
                toasts.push(o.label().to_string());
            }
            UiAction::LeaveField => {
                let hero_f = field.combat.get(hero);
                let in_fight = hero_f.map(|h| h.in_combat()).unwrap_or(false);
                let near_exit = hero_f.map(|h| h.pos.distance(field.exit) < 6.0).unwrap_or(false);
                if field.end == Some(End::Defeat) || field.end == Some(End::Return) {
                    ov.0 = None;
                    next.set(AppState::Town);
                } else if in_fight {
                    toasts.push("Impossible de fuir en plein combat !".into());
                } else if !near_exit {
                    toasts.push("Retournez au feu de camp pour rentrer.".into());
                } else {
                    field.end = Some(End::Return);
                    menu.summary = finish_expedition(&mut game, &mut field);
                    menu.dirty = true;
                    ov.0 = Some(crate::Panel::Summary);
                }
            }
            other => keep.push(other),
        }
    }
    queue.0.extend(keep);
}

/// Fait avancer le combat, applique la gloire et le butin.
fn step_combat(
    mut commands: Commands,
    time: Res<Time>,
    mut field: ResMut<Field>,
    mut game: ResMut<Game>,
    fonts: Res<Fonts>,
    mut lib: ResMut<Lib>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut toasts: ResMut<Toasts>,
    mut ov: ResMut<Overlay>,
    mut menu: ResMut<crate::menus::MenuState>,
) {
    if ov.0.is_some() || field.end.is_some() {
        return;
    }
    let c = content();
    let dt = time.delta_secs();
    field.combat.tick(dt);
    let events = field.combat.drain_events();
    let mut painter = Painter { lib: &mut lib, mats: &mut mats };
    let scope = DespawnOnExit(AppState::Field);
    let hero = field.hero;
    for ev in events {
        match ev {
            CEvent::Damage { dst, amount, kind, crit, src, .. } => {
                if let Some(f) = field.combat.get(dst) {
                    let color = if f.team == Team::Ally {
                        Color::srgb(1.0, 0.35, 0.3)
                    } else if crit {
                        Color::srgb(1.0, 0.85, 0.2)
                    } else {
                        col(kind_color(Some(kind)))
                    };
                    let size = if crit { 20.0 } else if src == hero || dst == hero { 15.0 } else { 12.0 };
                    let txt = if crit { format!("{:.0}!", amount) } else { format!("{:.0}", amount) };
                    spawn_float(&mut commands, &fonts, v3(f.pos, 2.0), txt, color, size);
                    spawn_sparks(&mut commands, &mut painter, v3(f.pos, 1.0), kind_color(Some(kind)), if crit { 8 } else { 4 });
                }
            }
            CEvent::Heal { dst, amount, .. } => {
                if let Some(f) = field.combat.get(dst) {
                    if amount >= 1.0 {
                        spawn_float(&mut commands, &fonts, v3(f.pos, 2.1), format!("+{:.0}", amount), Color::srgb(0.45, 1.0, 0.45), 13.0);
                    }
                }
            }
            CEvent::Shield { dst, amount } => {
                if let Some(f) = field.combat.get(dst) {
                    spawn_float(&mut commands, &fonts, v3(f.pos, 2.2), format!("+{:.0}", amount), Color::srgb(0.8, 0.88, 1.0), 12.0);
                    spawn_flash(&mut commands, &mut painter, v3(f.pos, 0.05), 0.9, (0.8, 0.88, 1.0), 0.4, 1.4);
                }
            }
            CEvent::Dodge { dst } => {
                if let Some(f) = field.combat.get(dst) {
                    spawn_float(&mut commands, &fonts, v3(f.pos, 2.0), "Esquive".into(), pal::DIM, 12.0);
                }
            }
            CEvent::Block { dst } => {
                if let Some(f) = field.combat.get(dst) {
                    spawn_float(&mut commands, &fonts, v3(f.pos, 2.3), "Bloqué".into(), pal::DIM, 11.0);
                }
            }
            CEvent::StatusOn { dst, kind } => {
                if matches!(kind, StatusKind::Frozen | StatusKind::Stun | StatusKind::Fear | StatusKind::Root | StatusKind::Silence | StatusKind::Taunted) {
                    if let Some(f) = field.combat.get(dst) {
                        spawn_float(&mut commands, &fonts, v3(f.pos, 2.5), kind.label().to_string(), Color::srgb(0.85, 0.75, 1.0), 12.0);
                    }
                }
            }
            CEvent::Say { who, text } => {
                if who == hero {
                    toasts.push(text);
                }
            }
            CEvent::SkillUsed { from, to, tags, slot, who, .. } => {
                let color = tags_color(&tags);
                if slot != errance_core::skill::SkillSlot::Basic {
                    if let Some(f) = field.combat.get(who) {
                        if let Some(s) = f.skill(slot) {
                            if f.team == Team::Ally {
                                spawn_float(&mut commands, &fonts, v3(f.pos, 2.6), s.name.clone(), Color::srgb(1.0, 0.95, 0.8), 11.0);
                            }
                            let radius = match s.targeting {
                                errance_core::skill::Targeting::AreaAroundSelf { radius }
                                | errance_core::skill::Targeting::AlliesAroundSelf { radius } => Some((from, radius)),
                                errance_core::skill::Targeting::AreaAtTarget { radius, .. }
                                | errance_core::skill::Targeting::AreaAtAlly { radius, .. }
                                    if s.delivery == errance_core::skill::Delivery::Instant => Some((to, radius)),
                                errance_core::skill::Targeting::Dash { radius, .. } | errance_core::skill::Targeting::Leap { radius, .. } if radius > 0.0 => Some((to, radius)),
                                _ => None,
                            };
                            if let Some((center, r)) = radius {
                                spawn_flash(&mut commands, &mut painter, v3(center, 0.06), r, color, 0.45, 1.0);
                            } else {
                                spawn_flash(&mut commands, &mut painter, v3(f.pos, 0.06), 0.8, color, 0.35, 1.6);
                            }
                        }
                    }
                }
            }
            CEvent::ProjectileSpawned { id } => {
                if let Some(pj) = field.combat.projectiles.iter().find(|p| p.id == id) {
                    let c = if pj.friendly { (0.5, 1.0, 0.5) } else { kind_color(pj.kind) };
                    let m = painter.mat(c, Finish::Glow);
                    commands.spawn((
                        Mesh3d(painter.lib.ico.clone()),
                        MeshMaterial3d(m),
                        Transform::from_translation(v3(pj.pos, 1.1)).with_scale(Vec3::splat(0.22)),
                        ProjView(id),
                        scope.clone(),
                    ));
                }
            }
            CEvent::ProjectileHit { at, .. } => {
                spawn_flash(&mut commands, &mut painter, v3(at, 0.06), 0.6, (1.0, 0.9, 0.7), 0.25, 1.3);
            }
            CEvent::ZoneSpawned { id } => {
                if let Some(z) = field.combat.zones.iter().find(|z| z.id == id) {
                    let hostile_to_player = z.team == Team::Enemy;
                    let color = if hostile_to_player { (1.0, 0.25, 0.15) } else { tags_color(&z.skill.all_tags()) };
                    let mesh = zone_mesh(&mut meshes, &z.shape);
                    let edge = painter.mat(color, Finish::Ghost);
                    let fillc = (color.0 * 1.1, color.1 * 0.9, color.2 * 0.9);
                    let fill_m = painter.mat(fillc, Finish::Ghost);
                    let base = commands
                        .spawn((Mesh3d(mesh.clone()), MeshMaterial3d(edge), Transform::from_translation(v3(z.center, 0.04)), scope.clone()))
                        .id();
                    let fill = commands
                        .spawn((Mesh3d(mesh), MeshMaterial3d(fill_m), Transform::from_translation(v3(z.center, 0.05)).with_scale(Vec3::new(0.01, 1.0, 0.01)), scope.clone()))
                        .id();
                    commands.entity(base).insert(ZoneView { id, fill });
                }
            }
            CEvent::ZoneResolved { center, .. } => {
                spawn_sparks(&mut commands, &mut painter, v3(center, 0.4), (1.0, 0.6, 0.3), 10);
            }
            CEvent::Death { who, .. } => {
                let (team, pos, key, name) = match field.combat.get(who) {
                    Some(f) => (f.team, f.pos, f.key.clone(), f.name.clone()),
                    None => continue,
                };
                if team == Team::Enemy {
                    field.kills += 1;
                    game.0.counters.kills += 1;
                    if let Some(e) = c.enemy(&key) {
                        let mut uid = game.0.next_uid;
                        let (gold, items) = enemy_drop(c, &mut field.rng, e, &mut uid);
                        game.0.next_uid = uid;
                        if gold > 0 || !items.is_empty() {
                            let entity = spawn_loot_bag(&mut commands, &mut painter, v3(pos, 0.0), !items.is_empty());
                            field.ground.push(GroundLoot { pos, gold, items, entity });
                        }
                        if e.boss {
                            toasts.push(format!("{name} est vaincu !"));
                        }
                    }
                } else if who == hero {
                    field.end = Some(End::Defeat);
                    game.0.counters.deaths += 1;
                    menu.summary = finish_expedition(&mut game, &mut field);
                    menu.dirty = true;
                    ov.0 = Some(crate::Panel::Summary);
                } else if let Some(nid) = field.npc_of.get(&who).copied() {
                    toasts.push(format!("{name} est à terre !"));
                    if let Some(n) = game.0.world.npc_mut(nid) {
                        n.affinity -= 5;
                    }
                }
            }
            CEvent::Aggro { .. } | CEvent::Reset { .. } | CEvent::Taunt { .. } | CEvent::CastStart { .. } | CEvent::CastInterrupted { .. } => {}
        }
    }
    // Potion bue ?
    if let Some(h) = field.combat.get(hero) {
        let used = h.cooldown(errance_core::skill::SkillSlot::Potion) > 0.0;
        if used && !field.used_potion {
            game.0.consume_potion();
            if game.0.hero.item(errance_core::item::EquipSlot::Potion).is_none() {
                if let Some(f) = field.combat.get_mut(hero) {
                    f.skills[errance_core::skill::SkillSlot::Potion.bar_index()] = None;
                }
            }
        }
        field.used_potion = used;
    }
    // Gloire : appliquée en direct, avec célébration.
    let ids: Vec<(FighterId, f32)> = field.combat.fighters.iter().filter(|f| f.team == Team::Ally).map(|f| (f.id, f.tally.fame_earned)).collect();
    for (fid, fame) in ids {
        let seen = field.fame_seen.get(&fid).copied().unwrap_or(0.0);
        if fame <= seen {
            continue;
        }
        field.fame_seen.insert(fid, fame);
        let delta = fame - seen;
        if fid == hero {
            let ups = game.0.hero.award_fame(c, delta);
            for u in &ups {
                let name = c.line(&u.line).map(|l| l.name.clone()).unwrap_or(u.line.clone());
                toasts.push(format!("* {name} : niveau {} !", u.to));
            }
            if !ups.is_empty() {
                let spec = game.0.hero.fighter_spec(c, Team::Ally, Controller::Player);
                field.combat.refresh(hero, &spec);
            }
        } else if let Some(nid) = field.npc_of.get(&fid).copied() {
            if let Some(n) = game.0.world.npc_mut(nid) {
                let ups = n.sheet.award_fame(c, delta);
                if !ups.is_empty() {
                    let spec = n.sheet.fighter_spec(c, Team::Ally, Controller::Companion { rules: n.sheet.rules.clone() });
                    field.combat.refresh(fid, &spec);
                }
            }
        }
    }
    // Zone nettoyée ?
    if !field.cleared_announced && !field.combat.fighters.iter().any(|f| f.alive && f.team == Team::Enemy) {
        field.cleared_announced = true;
        toasts.push("Zone nettoyée ! Retournez au feu de camp pour rentrer.".into());
    }
    let _ = item_name;
}

fn zone_mesh(meshes: &mut Assets<Mesh>, shape: &ZoneShape) -> Handle<Mesh> {
    use bevy::mesh::{Indices, PrimitiveTopology};
    use bevy::asset::RenderAssetUsages;
    let mut pos: Vec<[f32; 3]> = vec![];
    let mut idx: Vec<u32> = vec![];
    match *shape {
        ZoneShape::Circle { radius } => {
            pos.push([0.0, 0.0, 0.0]);
            let n = 32;
            for i in 0..=n {
                let a = i as f32 / n as f32 * std::f32::consts::TAU;
                pos.push([a.cos() * radius, 0.0, a.sin() * radius]);
            }
            for i in 1..=n as u32 {
                idx.extend([0, i + 1, i]);
            }
        }
        ZoneShape::Cone { dir, range, half_angle } => {
            pos.push([0.0, 0.0, 0.0]);
            let base = dir.angle();
            let n = 16;
            for i in 0..=n {
                let a = base - half_angle + 2.0 * half_angle * i as f32 / n as f32;
                pos.push([a.cos() * range, 0.0, a.sin() * range]);
            }
            for i in 1..=n as u32 {
                idx.extend([0, i + 1, i]);
            }
        }
        ZoneShape::Line { dir, length, half_width } => {
            let d = dir.normalize_or_zero();
            let p = d.perp() * half_width;
            let e = d * length;
            pos.extend([[p.x, 0.0, p.y], [-p.x, 0.0, -p.y], [e.x - p.x, 0.0, e.y - p.y], [e.x + p.x, 0.0, e.y + p.y]]);
            idx.extend([0, 1, 2, 0, 2, 3, 0, 2, 1, 0, 3, 2]);
        }
    }
    let n = pos.len();
    let mut m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; n]);
    m.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; n]);
    m.insert_indices(Indices::U32(idx));
    meshes.add(m)
}

pub fn spawn_float(commands: &mut Commands, fonts: &Fonts, world: Vec3, text: String, color: Color, size: f32) {
    commands.spawn((
        Text::new(text),
        crate::ui::font(&fonts.bold, size),
        TextColor(color),
        TextShadow { offset: Vec2::new(1.0, 1.0), color: Color::srgba(0.0, 0.0, 0.0, 0.85) },
        Node { position_type: PositionType::Absolute, left: px(-100), top: px(-100), ..default() },
        FloatText { world, age: 0.0, life: 1.0 },
        GlobalZIndex(5),
        DespawnOnExit(AppState::Field),
    ));
}

fn spawn_sparks(commands: &mut Commands, p: &mut Painter, at: Vec3, c: Rgb, n: usize) {
    let m = p.mat(c, Finish::Glow);
    for i in 0..n {
        let a = i as f32 * 2.4 + at.x * 3.0;
        let vel = Vec3::new(a.cos() * 2.5, 2.5 + (i % 3) as f32, a.sin() * 2.5);
        commands.spawn((
            Mesh3d(p.lib.cube.clone()),
            MeshMaterial3d(m.clone()),
            Transform::from_translation(at).with_scale(Vec3::splat(0.09)),
            Spark { vel, life: 0.45 },
            DespawnOnExit(AppState::Field),
        ));
    }
}

fn spawn_flash(commands: &mut Commands, p: &mut Painter, at: Vec3, radius: f32, c: Rgb, life: f32, grow: f32) {
    let m = p.mat(c, Finish::Ghost);
    commands.spawn((
        Mesh3d(p.lib.ring.clone()),
        MeshMaterial3d(m),
        Transform::from_translation(at).with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)).with_scale(Vec3::splat(radius * 0.6)),
        Flash { life, max: life, grow: radius * grow },
        DespawnOnExit(AppState::Field),
    ));
}

fn spawn_loot_bag(commands: &mut Commands, p: &mut Painter, at: Vec3, has_item: bool) -> Entity {
    let root = commands.spawn((Transform::from_translation(at), Visibility::default(), LootView, DespawnOnExit(AppState::Field))).id();
    if has_item {
        p.cube(commands, root, (0.45, 0.3, 0.15), Finish::Matte, Vec3::new(0.0, 0.18, 0.0), Vec3::new(0.5, 0.35, 0.35));
        p.cube(commands, root, (0.85, 0.7, 0.25), Finish::Metal, Vec3::new(0.0, 0.38, 0.0), Vec3::new(0.52, 0.06, 0.37));
        p.ball(commands, root, (1.0, 0.85, 0.35), Finish::Glow, Vec3::new(0.0, 0.8, 0.0), Vec3::splat(0.12));
    } else {
        p.ball(commands, root, (0.95, 0.8, 0.3), Finish::Metal, Vec3::new(0.0, 0.12, 0.0), Vec3::new(0.3, 0.2, 0.3));
    }
    root
}

/// Place les modèles selon le combat et les anime.
fn sync_views(
    time: Res<Time>,
    field: Res<Field>,
    mut roots: Query<(Entity, &FighterView, &mut Transform), Without<Limb>>,
    mut limbs: Query<(&Limb, &mut Transform)>,
    mut rings: ParamSet<(
        Query<(&mut Transform, &mut Visibility), (With<TargetRing>, Without<FighterView>, Without<Limb>)>,
        Query<&mut Transform, (With<HeroRing>, Without<FighterView>, Without<Limb>, Without<TargetRing>)>,
    )>,
    mut dead_timer: Local<HashMap<FighterId, f32>>,
) {
    let dt = time.delta_secs();
    let mut states: HashMap<Entity, AnimState> = HashMap::default();
    for (e, view, mut t) in roots.iter_mut() {
        let Some(f) = field.combat.get(view.0) else { continue };
        let target = v3(f.pos, 0.0);
        if !f.alive {
            let k = dead_timer.entry(f.id).or_insert(0.0);
            *k += dt;
            t.translation.x = target.x;
            t.translation.z = target.z;
            let fall = (*k * 3.0).min(1.0);
            t.rotation = Quat::from_rotation_y(-f.facing.angle() + std::f32::consts::FRAC_PI_2) * Quat::from_rotation_z(fall * 1.45);
            if *k > 2.5 {
                t.translation.y = -(*k - 2.5) * 0.5;
            }
            states.insert(e, AnimState { dead: true, ..default() });
            continue;
        }
        t.translation = t.translation.lerp(target, (dt * 20.0).min(1.0));
        let yaw = -f.facing.angle() + std::f32::consts::FRAC_PI_2;
        let want = Quat::from_rotation_y(yaw);
        t.rotation = t.rotation.slerp(want, (dt * 12.0).min(1.0));
        states.insert(e, AnimState {
            moving: f.moving,
            speed: f.move_speed(),
            swing: f.swing,
            casting: f.cast.is_some(),
            dead: false,
            stunned: f.incapacitated(),
        });
    }
    animate_limbs(time.elapsed_secs(), &states, &mut limbs);
    // Anneau de cible.
    let hero = field.combat.get(field.hero);
    let target = hero.and_then(|h| h.target).and_then(|t| field.combat.get(t)).filter(|t| t.alive);
    if let Ok((mut t, mut vis)) = rings.p0().single_mut() {
        match target {
            Some(tg) => {
                *vis = Visibility::Visible;
                t.translation = v3(tg.pos, 0.03);
                t.scale = Vec3::splat(tg.radius * 1.5 + 0.2);
            }
            None => *vis = Visibility::Hidden,
        }
    }
    if let (Some(h), Ok(mut t)) = (hero, rings.p1().single_mut()) {
        t.translation = v3(h.pos, 0.02);
    }
}

#[allow(clippy::type_complexity)]
fn update_vfx(
    mut commands: Commands,
    time: Res<Time>,
    field: Res<Field>,
    mut projs: Query<(Entity, &ProjView, &mut Transform), (Without<Spark>, Without<Flash>, Without<ZoneView>)>,
    zones: Query<(Entity, &ZoneView)>,
    mut fills: Query<&mut Transform, (Without<ProjView>, Without<Spark>, Without<Flash>, Without<ZoneView>)>,
    mut sparks: Query<(Entity, &mut Spark, &mut Transform), (Without<ProjView>, Without<Flash>)>,
    mut flashes: Query<(Entity, &mut Flash, &mut Transform), (Without<ProjView>, Without<Spark>)>,
) {
    let dt = time.delta_secs();
    for (e, pv, mut t) in projs.iter_mut() {
        match field.combat.projectiles.iter().find(|p| p.id == pv.0) {
            Some(p) => t.translation = t.translation.lerp(v3(p.pos, 1.1), (dt * 30.0).min(1.0)),
            None => commands.entity(e).despawn(),
        }
    }
    for (e, zv) in zones.iter() {
        match field.combat.zones.iter().find(|z| z.id == zv.id) {
            Some(z) => {
                let k = 1.0 - (z.remaining / z.total.max(0.01)).clamp(0.0, 1.0);
                if let Ok(mut ft) = fills.get_mut(zv.fill) {
                    ft.scale = Vec3::new(k.max(0.01), 1.0, k.max(0.01));
                }
            }
            None => {
                commands.entity(zv.fill).despawn();
                commands.entity(e).despawn();
            }
        }
    }
    for (e, mut s, mut t) in sparks.iter_mut() {
        s.life -= dt;
        s.vel.y -= 12.0 * dt;
        t.translation += s.vel * dt;
        t.scale = Vec3::splat(0.09 * (s.life / 0.45).max(0.0));
        if s.life <= 0.0 {
            commands.entity(e).despawn();
        }
    }
    for (e, mut f, mut t) in flashes.iter_mut() {
        f.life -= dt;
        let k = 1.0 - (f.life / f.max).max(0.0);
        t.scale = Vec3::splat(f.grow * (0.6 + 0.4 * k)) * (1.0 - k * 0.2);
        if f.life <= 0.0 {
            commands.entity(e).despawn();
        }
    }
}

fn camera_follow(time: Res<Time>, field: Res<Field>, mut cam: Query<&mut Transform, With<MainCamera>>, zoom: Res<crate::hud::Zoom>) {
    let Some(h) = field.combat.get(field.hero) else { return };
    let Ok(mut t) = cam.single_mut() else { return };
    let target = v3(h.pos, 0.0);
    let offset = Vec3::new(0.0, 13.0, 9.5) * zoom.0;
    let want = target + offset;
    let k = (time.delta_secs() * 5.0).min(1.0);
    t.translation = t.translation.lerp(want, k);
    let look = t.translation - offset;
    t.look_at(look, Vec3::Y);
}

fn pickup_loot(mut commands: Commands, mut field: ResMut<Field>, fonts: Res<Fonts>, mut toasts: ResMut<Toasts>) {
    let c = content();
    let Some(h) = field.combat.get(field.hero) else { return };
    if !h.alive {
        return;
    }
    let hp = h.pos;
    let mut picked = vec![];
    for (i, g) in field.ground.iter().enumerate() {
        if g.pos.distance(hp) < 1.4 {
            picked.push(i);
        }
    }
    for i in picked.into_iter().rev() {
        let g = field.ground.remove(i);
        commands.entity(g.entity).despawn();
        field.gold += g.gold;
        if g.gold > 0 {
            spawn_float(&mut commands, &fonts, v3(g.pos, 1.5), format!("+{} po", g.gold), pal::GOLD, 13.0);
        }
        for it in g.items {
            toasts.push(format!("Butin : {}", item_name(c, &it)));
            field.loot.push(it);
        }
    }
}

fn float_texts(
    mut commands: Commands,
    time: Res<Time>,
    cam: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    scale: Res<UiScale>,
    mut q: Query<(Entity, &mut FloatText, &mut Node, &mut TextColor)>,
) {
    let Ok((camera, ct)) = cam.single() else { return };
    let dt = time.delta_secs();
    for (e, mut f, mut node, mut color) in q.iter_mut() {
        f.age += dt;
        if f.age >= f.life {
            commands.entity(e).despawn();
            continue;
        }
        let w = f.world + Vec3::Y * f.age * 1.2;
        if let Ok(p) = camera.world_to_viewport(ct, w) {
            node.left = px(p.x / scale.0 - 12.0);
            node.top = px(p.y / scale.0 - 8.0);
        }
        let a = 1.0 - (f.age / f.life).powi(2);
        color.0 = color.0.with_alpha(a);
    }
}

/// Résumé de fin d'expédition : applique ou perd le butin.
pub fn finish_expedition(game: &mut Game, field: &mut Field) -> Vec<String> {
    let c = content();
    let mut lines = vec![];
    let p = &mut game.0;
    match field.end {
        Some(End::Defeat) => {
            lines.push("Vous êtes tombé. On vous a ramené en ville, couvert de boue.".into());
            if !field.loot.is_empty() || field.gold > 0 {
                lines.push(format!("Butin perdu : {} objet(s), {} po.", field.loot.len(), field.gold));
            }
            let lost = p.gold / 10;
            if lost > 0 {
                p.gold -= lost;
                lines.push(format!("Le guérisseur a pris {lost} po."));
            }
            field.loot.clear();
            field.gold = 0;
        }
        _ => {
            lines.push(format!("Retour à Gué-Rouge. Ennemis vaincus : {}.", field.kills));
            if field.gold > 0 {
                lines.push(format!("+{} pièces d'or.", field.gold));
                p.earn(field.gold);
            }
            for it in field.loot.drain(..) {
                let name = item_name(c, &it);
                if p.add_to_bag(c, it) {
                    lines.push(format!("+ {name}"));
                } else {
                    lines.push(format!("Sac plein : {name} abandonné."));
                }
            }
            field.gold = 0;
        }
    }
    // Les mercenaires ont fini leur contrat.
    for nid in field.npc_of.values() {
        if let Some(n) = p.world.npc_mut(*nid) {
            if n.hired_for > 0 {
                n.hired_for -= 1;
                if n.hired_for == 0 && !n.companion {
                    lines.push(format!("{} a rempli son contrat.", n.sheet.name));
                }
            }
            n.affinity += 2;
        }
    }
    p.day += 1;
    let mut wrng = p.rng.fork();
    p.world.advance_day(c, &mut wrng);
    lines
}
