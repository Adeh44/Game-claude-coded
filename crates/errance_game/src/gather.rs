//! Récolte en expédition : nœuds visibles, jauge, gains.
//!
//! Les règles (portée, durée, rendement) sont dans `errance_core::craft` ;
//! ici on dessine les arbres marqués, filons, plantes et carcasses, et on
//! affiche la jauge quand le héros reste à côté.

use crate::audio::{PlaySfx, Sfx};
use crate::field::{Field, spawn_float};
use crate::models::{Finish, Painter, spawn_rock, spawn_tree};
use crate::ui::{Fonts, font, pal};
use crate::{AppState, Game, Overlay, Toasts};
use bevy::prelude::*;
use errance_core::content::content;
use errance_core::craft::{GatherTick, LINE_GATHER, Node as ResNode, NodeKind, gather_fame, stack_add};
use errance_core::look::Rgb;

/// Un nœud dessiné : la racine (tout le modèle) et son repère lumineux.
pub struct NodeView {
    pub root: Entity,
    pub marker: Entity,
}

#[derive(Component)]
struct Marker {
    phase: f32,
    y: f32,
}

#[derive(Component)]
struct GatherBar;
#[derive(Component)]
struct GatherFill;
#[derive(Component)]
struct GatherLabel;

pub struct GatherPlugin;

impl Plugin for GatherPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Field), build_gather_bar.after(crate::field::setup_field)).add_systems(
            Update,
            (gather_step, bob_markers).chain().after(crate::field::step_combat).run_if(in_state(AppState::Field)),
        );
    }
}

fn v3(p: errance_core::math::Vec2, y: f32) -> Vec3 {
    Vec3::new(p.x, y, p.y)
}

fn res_color(kind: NodeKind) -> Rgb {
    content().resource_of(kind).map(|r| r.color).unwrap_or((0.8, 0.8, 0.8))
}

/// Dessine un nœud de récolte.
pub fn spawn_node(p: &mut Painter, cmd: &mut Commands, n: &ResNode, zone_foliage: Rgb, zone_rock: Rgb, tree_kind: &str, rot: f32) -> NodeView {
    let at = v3(n.pos, 0.0);
    let rc = res_color(n.kind);
    let root = match n.kind {
        NodeKind::Tree => {
            let e = spawn_tree(p, cmd, at, tree_kind, zone_foliage, 1.05, rot);
            // Entaille claire sur le tronc.
            p.cube(cmd, e, (0.92, 0.82, 0.6), Finish::Matte, Vec3::new(0.0, 0.55, 0.2), Vec3::new(0.22, 0.28, 0.08));
            e
        }
        NodeKind::Vein => {
            let e = spawn_rock(p, cmd, at, zone_rock, Vec3::new(1.2, 1.0, 1.1), rot);
            // Éclats de minerai qui brillent.
            for k in 0..4 {
                let a = k as f32 * 1.7 + rot;
                p.ball(cmd, e, rc, Finish::Metal, Vec3::new(a.cos() * 0.45, 0.35 + (k % 2) as f32 * 0.25, a.sin() * 0.45), Vec3::splat(0.2));
            }
            e
        }
        NodeKind::Herb => {
            let e = cmd.spawn((Transform::from_translation(at).with_rotation(Quat::from_rotation_y(rot)), Visibility::default())).id();
            for k in 0..7 {
                let a = k as f32 * 0.9;
                let r = 0.15 + (k % 3) as f32 * 0.12;
                let h = 0.45 + (k % 2) as f32 * 0.2;
                p.cone(cmd, e, zone_foliage, Finish::Matte, Transform::from_xyz(a.cos() * r, h / 2.0, a.sin() * r).with_scale(Vec3::new(0.1, h, 0.1)));
            }
            // Fleurs / épis de la couleur de la fibre.
            for k in 0..4 {
                let a = k as f32 * 1.6 + 0.4;
                p.ball(cmd, e, rc, Finish::Matte, Vec3::new(a.cos() * 0.22, 0.62, a.sin() * 0.22), Vec3::splat(0.12));
            }
            e
        }
        NodeKind::Carcass => {
            let e = cmd.spawn((Transform::from_translation(at).with_rotation(Quat::from_rotation_y(rot)), Visibility::default())).id();
            p.ball(cmd, e, rc, Finish::Matte, Vec3::new(0.0, 0.18, 0.0), Vec3::new(0.9, 0.35, 0.5));
            p.ball(cmd, e, (rc.0 * 0.8, rc.1 * 0.8, rc.2 * 0.8), Finish::Matte, Vec3::new(0.5, 0.15, 0.0), Vec3::new(0.35, 0.25, 0.3));
            for x in [-0.25, 0.2] {
                p.cube(cmd, e, (rc.0 * 0.7, rc.1 * 0.7, rc.2 * 0.7), Finish::Matte, Vec3::new(x, 0.1, 0.3), Vec3::new(0.08, 0.08, 0.35));
            }
            e
        }
    };
    // Repère : un petit losange qui flotte au-dessus.
    let h = match n.kind {
        NodeKind::Tree => 4.3,
        NodeKind::Vein => 1.6,
        _ => 1.2,
    };
    let marker = cmd
        .spawn((
            Transform::from_translation(at + Vec3::Y * h).with_scale(Vec3::new(0.18, 0.3, 0.18)),
            Visibility::default(),
            Marker { phase: rot, y: h },
        ))
        .id();
    let m = p.lib.ico.clone();
    p.part(cmd, marker, &m, (1.0, 0.9, 0.5), Finish::Glow, Transform::default());
    let scope = DespawnOnExit(AppState::Field);
    cmd.entity(root).insert(scope.clone());
    cmd.entity(marker).insert(scope);
    NodeView { root, marker }
}

fn bob_markers(time: Res<Time>, mut q: Query<(&Marker, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (m, mut tr) in q.iter_mut() {
        tr.rotation = Quat::from_rotation_y(t * 1.5 + m.phase);
        tr.translation.y = m.y + (t * 2.0 + m.phase).sin() * 0.08;
    }
}

fn build_gather_bar(mut commands: Commands, fonts: Res<Fonts>) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: px(168),
                left: percent(50),
                margin: UiRect::left(px(-90)),
                width: px(180),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(2),
                ..default()
            },
            Visibility::Hidden,
            GatherBar,
            GlobalZIndex(8),
            DespawnOnExit(AppState::Field),
        ))
        .with_children(|p| {
            p.spawn((Text::new(""), font(&fonts.bold, 11.0), TextColor(pal::INK), GatherLabel));
            p.spawn((
                Node { width: px(170), height: px(9), border: UiRect::all(px(1)), border_radius: BorderRadius::all(px(3)), overflow: Overflow::clip(), ..default() },
                BorderColor::all(Color::srgba(0.0, 0.0, 0.0, 0.8)),
                BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.8)),
            ))
            .with_children(|b| {
                b.spawn((
                    Node { position_type: PositionType::Absolute, left: px(0), top: px(0), bottom: px(0), width: percent(0), ..default() },
                    BackgroundColor(Color::srgb(0.5, 0.85, 0.4)),
                    GatherFill,
                ));
            });
        });
}

#[allow(clippy::too_many_arguments)]
fn gather_step(
    mut commands: Commands,
    time: Res<Time>,
    field: Option<ResMut<Field>>,
    mut game: ResMut<Game>,
    fonts: Res<Fonts>,
    ov: Res<Overlay>,
    mut toasts: ResMut<Toasts>,
    mut sfx: MessageWriter<PlaySfx>,
    mut bar: Query<&mut Visibility, With<GatherBar>>,
    mut fill: Query<&mut Node, With<GatherFill>>,
    mut label: Query<(&mut Text, &mut TextColor), With<GatherLabel>>,
) {
    let Some(mut field) = field else { return };
    let c = content();
    let mut show = false;
    let mut frac = 0.0;
    let mut text = String::new();
    let mut ok = true;
    let alive = field.combat.get(field.hero).map(|h| (h.alive, h.pos));
    if ov.0.is_none() && field.end.is_none() {
        if let Some((true, pos)) = alive {
            if std::mem::take(&mut field.gather_hit) {
                field.gathering.interrupt();
            }
            let lvl = game.0.hero.masteries.level(LINE_GATHER);
            let dt = time.delta_secs();
            let f = &mut *field;
            let tick = f.gathering.tick(&mut f.nodes, pos, dt, lvl, &mut f.rng);
            if std::env::var("ERRANCE_DEBUG_GATHER").is_ok() {
                let d = f.nodes.iter().map(|n| n.pos.distance(pos)).fold(f32::MAX, f32::min);
                eprintln!("GATHER {tick:?} nearest={d:.2}");
            }
            let name_of = |n: &ResNode| c.resource_of(n.kind).map(|r| r.tier_name(n.tier).to_string()).unwrap_or_default();
            match tick {
                GatherTick::Idle => {}
                GatherTick::Working { node, frac: fr } => {
                    let n = &field.nodes[node];
                    show = true;
                    frac = fr;
                    text = format!("{} : {} (T{})", n.kind.verb(), name_of(n), n.tier);
                }
                GatherTick::TooHard { node, need } => {
                    let n = &field.nodes[node];
                    show = true;
                    ok = false;
                    text = format!("{} T{} : Récolte niv. {need} requise", name_of(n), n.tier);
                }
                GatherTick::Got { node, kind, tier, count } => {
                    let n = field.nodes[node].clone();
                    let Some(res) = c.resource_of(kind) else { return };
                    stack_add(&mut field.resources, &res.id, tier, count);
                    game.0.counters.gathered += count;
                    spawn_float(&mut commands, &fonts, v3(n.pos, 1.8), format!("+{count} {}", res.tier_name(tier)), crate::ui::rgb(res.color), 13.0);
                    sfx.write(PlaySfx(Sfx::Swing));
                    if let Some(up) = game.0.hero.masteries.add_fame(LINE_GATHER, gather_fame(tier) * count as f64) {
                        sfx.write(PlaySfx(Sfx::LevelUp));
                        toasts.push(format!("* Récolte : niveau {} !", up.to));
                        game.0.hero.fix_loadout(c);
                    }
                    if n.left == 0 {
                        if let Some(v) = field.node_views.get(node) {
                            commands.entity(v.marker).despawn();
                            // Les plantes et les carcasses disparaissent ; arbres et rochers restent.
                            if matches!(kind, NodeKind::Herb | NodeKind::Carcass) {
                                commands.entity(v.root).despawn();
                            }
                        }
                    }
                    show = true;
                    frac = 1.0;
                    text = format!("{} : {}", n.kind.verb(), res.tier_name(tier));
                }
            }
        }
    }
    if let Ok(mut v) = bar.single_mut() {
        let want = if show { Visibility::Inherited } else { Visibility::Hidden };
        if *v != want {
            *v = want;
        }
    }
    if show {
        if let Ok(mut n) = fill.single_mut() {
            n.width = percent(frac * 100.0);
        }
        if let Ok((mut t, mut cl)) = label.single_mut() {
            **t = text;
            cl.0 = if ok { pal::INK } else { pal::RED };
        }
    }
}

/// Une bête abattue devient une carcasse à dépecer.
pub fn add_carcass(field: &mut Field, p: &mut Painter, cmd: &mut Commands, n: ResNode) {
    let rot = field.rng.range_f32(0.0, std::f32::consts::TAU);
    let v = spawn_node(p, cmd, &n, (0.3, 0.5, 0.25), (0.5, 0.5, 0.5), "", rot);
    field.nodes.push(n);
    field.node_views.push(v);
}
