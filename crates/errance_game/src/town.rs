//! Gué-Rouge : la ville de départ. Un diorama 3D derrière un menu tactile.

use crate::models::{Finish, Lib, Painter, spawn_look, spawn_tree};
use crate::ui::{self, Fonts, UiAction, UiQueue, pal};
use crate::{AppState, Game, Overlay, Panel};
use bevy::prelude::*;
use errance_core::content::content;

pub struct TownPlugin;

impl Plugin for TownPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Town), (build_town_scene, build_town_ui))
            .add_systems(Update, (town_actions, orbit_camera, refresh_town_ui, hide_under_overlay).run_if(in_state(AppState::Town)));
    }
}

#[derive(Component)]
struct TownCamera;

#[derive(Component)]
struct TownUi;

#[derive(Component)]
struct TownInfo;

#[derive(Component)]
struct TownHint;

fn house(p: &mut Painter, cmd: &mut Commands, at: Vec3, rot: f32, wall: (f32, f32, f32), roof: (f32, f32, f32), size: Vec3) -> Entity {
    let root = cmd.spawn((Transform::from_translation(at).with_rotation(Quat::from_rotation_y(rot)), Visibility::default(), DespawnOnExit(AppState::Town))).id();
    p.cube(cmd, root, wall, Finish::Matte, Vec3::new(0.0, size.y / 2.0, 0.0), size);
    // Colombages.
    for x in [-0.5, 0.0, 0.5] {
        p.cube(cmd, root, (0.35, 0.22, 0.12), Finish::Matte, Vec3::new(x * size.x * 0.9, size.y / 2.0, size.z / 2.0 + 0.01), Vec3::new(0.12, size.y, 0.04));
    }
    p.cube(cmd, root, (0.35, 0.22, 0.12), Finish::Matte, Vec3::new(0.0, size.y * 0.55, size.z / 2.0 + 0.02), Vec3::new(size.x, 0.12, 0.04));
    // Toit (prisme approché par deux pans).
    for s in [-1.0f32, 1.0] {
        p.part(cmd, root, &p.lib.cube.clone(), roof, Finish::Matte,
            Transform::from_xyz(s * size.x * 0.27, size.y + size.x * 0.22, 0.0)
                .with_rotation(Quat::from_rotation_z(-s * 0.75))
                .with_scale(Vec3::new(size.x * 0.72, 0.14, size.z * 1.12)));
    }
    // Porte et fenêtre.
    p.cube(cmd, root, (0.3, 0.2, 0.12), Finish::Matte, Vec3::new(0.0, 0.55, size.z / 2.0 + 0.03), Vec3::new(0.6, 1.1, 0.05));
    p.cube(cmd, root, (1.0, 0.8, 0.45), Finish::Glow, Vec3::new(size.x * 0.3, size.y * 0.7, size.z / 2.0 + 0.03), Vec3::new(0.35, 0.35, 0.03));
    root
}

fn build_town_scene(mut commands: Commands, mut lib: ResMut<Lib>, mut mats: ResMut<Assets<StandardMaterial>>, mut meshes: ResMut<Assets<Mesh>>, mut clear: ResMut<ClearColor>, game: Res<Game>) {
    clear.0 = Color::srgb(0.55, 0.62, 0.72);
    let scope = DespawnOnExit(AppState::Town);
    let mut p = Painter { lib: &mut lib, mats: &mut mats };
    let ground = meshes.add(Plane3d::default().mesh().size(80.0, 80.0));
    let gm = p.mat((0.38, 0.47, 0.28), Finish::Matte);
    commands.spawn((Mesh3d(ground), MeshMaterial3d(gm), Transform::default(), scope.clone()));
    // Place pavée.
    let disc = p.lib.disc.clone();
    let pm = p.mat((0.55, 0.52, 0.47), Finish::Matte);
    commands.spawn((Mesh3d(disc), MeshMaterial3d(pm), Transform::from_xyz(0.0, 0.01, 0.0).with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)).with_scale(Vec3::splat(7.5)), scope.clone()));
    // Rivière (le « gué rouge »).
    let river = p.mat((0.35, 0.5, 0.62), Finish::Metal);
    commands.spawn((Mesh3d(p.lib.cube.clone()), MeshMaterial3d(river), Transform::from_xyz(0.0, 0.0, -14.0).with_scale(Vec3::new(80.0, 0.05, 4.0)), scope.clone()));
    // Maisons en cercle.
    let walls = [(0.88, 0.82, 0.7), (0.82, 0.75, 0.62), (0.9, 0.86, 0.78), (0.78, 0.72, 0.6)];
    let roofs = [(0.6, 0.25, 0.18), (0.45, 0.3, 0.2), (0.55, 0.2, 0.15), (0.35, 0.35, 0.4)];
    for i in 0..9 {
        let a = i as f32 / 9.0 * std::f32::consts::TAU + 0.3;
        let r = 11.0 + (i % 2) as f32 * 2.0;
        let at = Vec3::new(a.cos() * r, 0.0, a.sin() * r);
        let size = Vec3::new(3.2 + (i % 3) as f32 * 0.6, 2.4 + (i % 2) as f32 * 0.5, 2.8);
        let rot = -a - std::f32::consts::FRAC_PI_2;
        house(&mut p, &mut commands, at, rot, walls[i % 4], roofs[i % 4], size);
    }
    // Puits au centre.
    let well = commands.spawn((Transform::default(), Visibility::default(), scope.clone())).id();
    p.part(&mut commands, well, &p.lib.cyl10.clone(), (0.6, 0.58, 0.55), Finish::Matte, Transform::from_xyz(0.0, 0.4, 0.0).with_scale(Vec3::new(1.4, 0.8, 1.4)));
    p.part(&mut commands, well, &p.lib.cyl10.clone(), (0.2, 0.3, 0.4), Finish::Metal, Transform::from_xyz(0.0, 0.79, 0.0).with_scale(Vec3::new(1.1, 0.04, 1.1)));
    for x in [-0.6, 0.6] {
        p.cube(&mut commands, well, (0.4, 0.28, 0.17), Finish::Matte, Vec3::new(x, 1.3, 0.0), Vec3::new(0.1, 1.2, 0.1));
    }
    p.cube(&mut commands, well, (0.55, 0.25, 0.15), Finish::Matte, Vec3::new(0.0, 2.0, 0.0), Vec3::new(1.6, 0.12, 0.9));
    // Arbres autour.
    for i in 0..14 {
        let a = i as f32 * 0.45;
        let at = Vec3::new(a.cos() * 20.0, 0.0, a.sin() * 20.0);
        let e = spawn_tree(&mut p, &mut commands, at, if i % 3 == 0 { "sapin" } else { "feuillu" }, (0.3, 0.5, 0.25), 1.2, a);
        commands.entity(e).insert(scope.clone());
    }
    // Le héros, au milieu, avec ce qu'il porte.
    let look = game.0.hero.look(content());
    let hero = spawn_look(&mut p, &mut commands, &look, Vec3::new(1.5, 0.0, 2.0));
    commands.entity(hero).insert((scope.clone(), Transform::from_xyz(1.5, 0.0, 2.0).with_rotation(Quat::from_rotation_y(0.4))));
    // Quelques aventuriers de la taverne qui traînent sur la place.
    for (k, n) in game.0.world.tavern().iter().take(4).enumerate() {
        let look = n.sheet.look(content());
        let a = k as f32 * 1.4 + 2.0;
        let at = Vec3::new(a.cos() * 4.5, 0.0, a.sin() * 4.5);
        let e = spawn_look(&mut p, &mut commands, &look, at);
        commands.entity(e).insert((scope.clone(), Transform::from_translation(at).with_rotation(Quat::from_rotation_y(-a))));
    }
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            color: Color::srgb(1.0, 0.93, 0.8),
            #[cfg(not(target_os = "android"))]
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(6.0, 14.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
        scope.clone(),
    ));
    commands.spawn((
        Camera3d::default(),
        TownCamera,
        Projection::Perspective(PerspectiveProjection { fov: 40f32.to_radians(), ..default() }),
        Transform::from_xyz(0.0, 12.0, 16.0).looking_at(Vec3::ZERO, Vec3::Y),
        // MSAA coupé partout, comme la caméra de l'interface (voir setup_ui_camera).
        Msaa::Off,
        scope,
    ));
}

fn orbit_camera(time: Res<Time>, mut q: Query<&mut Transform, With<TownCamera>>) {
    let Ok(mut t) = q.single_mut() else { return };
    let a = time.elapsed_secs() * 0.04;
    t.translation = Vec3::new(a.sin() * 17.0, 11.0, a.cos() * 17.0);
    t.look_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y);
}

fn build_town_ui(mut commands: Commands, fonts: Res<Fonts>, game: Res<Game>) {
    let p = &game.0;
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(10),
                top: px(10),
                bottom: px(10),
                width: px(230),
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                padding: UiRect::all(px(10)),
                border_radius: BorderRadius::all(px(8)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.06, 0.05, 0.86)),
            TownUi,
            GlobalZIndex(5),
            DespawnOnExit(AppState::Town),
        ))
        .with_children(|c| {
            c.spawn(ui::title(&fonts, "Gué-Rouge", 22.0, pal::GOLD));
            c.spawn((Text::new(town_info(p)), ui::font(&fonts.regular, 12.0), TextColor(pal::DIM), TownInfo));
            for (label, b) in [
                ("Tableau des corvées", "corvees"),
                ("Marché", "marche"),
                ("Atelier du forgeron", "atelier"),
                ("Taverne du Pichet Fêlé", "taverne"),
                ("Porte de la ville", "porte"),
                ("Auberge (repos)", "auberge"),
            ] {
                c.spawn(ui::button(&fonts, label, UiAction::Building(b.into()), percent(100), 13.0));
            }
            c.spawn(Node { flex_grow: 1.0, ..default() });
            c.spawn(ui::row()).with_children(|r| {
                r.spawn(ui::button(&fonts, "Sac", UiAction::OpenBag, px(62), 12.0));
                r.spawn(ui::button(&fonts, "Fiche", UiAction::OpenCharacter, px(62), 12.0));
                r.spawn(ui::button(&fonts, "Atlas", UiAction::OpenAtlas, px(62), 12.0));
            });
        });
    // Conseil du moment (en bas à droite).
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: px(12),
            bottom: px(12),
            max_width: px(300),
            padding: UiRect::all(px(8)),
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.08, 0.06, 0.05, 0.8)),
        GlobalZIndex(5),
        TownUi,
        DespawnOnExit(AppState::Town),
        children![(Text::new(hint(p)), ui::font(&fonts.regular, 12.0), TextColor(pal::INK), TownHint)],
    ));
}

fn town_info(p: &errance_core::profile::Profile) -> String {
    let c = content();
    format!(
        "Jour {} · {} po\n{}\nPV {:.0} · Rôle : {}",
        p.day,
        p.gold,
        c.line(&p.hero.weapon_line(c)).map(|l| l.name.clone()).unwrap_or_default(),
        p.hero.stats(c).get(errance_core::stats::Stat::MaxHp),
        p.hero.role(c).title
    )
}

fn hint(p: &errance_core::profile::Profile) -> String {
    let c = content();
    if p.hero.item(errance_core::item::EquipSlot::MainHand).is_none() && p.gold < 12 {
        "Vous n'avez rien, pas même une arme. Le tableau des corvées paie mal... mais il paie.".into()
    } else if p.hero.item(errance_core::item::EquipSlot::MainHand).is_none() {
        "Vous avez de quoi vous offrir un gourdin au marché. Ce serait plus sage que vos poings.".into()
    } else if p.counters.expeditions == 0 {
        "Les Marches de Brume, juste derrière la porte : des rats, des loups maigres. Un bon début.".into()
    } else if p.world.party().is_empty() && p.gold > 30 {
        "Seul, c'est dur. À la taverne, des mercenaires se louent pour une expédition.".into()
    } else if p.hero.masteries.total() > 40 && p.flags.iter().all(|f| f != "compagnie") {
        "Les compagnies cherchent des porteurs. Pas glorieux, mais on apprend en regardant les meilleurs.".into()
    } else {
        let _ = c;
        "Chaque arme et chaque armure a sa Voie. Ouvrez l'Atlas pour voir ce que vous débloquerez.".into()
    }
}

fn refresh_town_ui(game: Res<Game>, mut info: Query<&mut Text, (With<TownInfo>, Without<TownHint>)>, mut h: Query<&mut Text, (With<TownHint>, Without<TownInfo>)>) {
    if !game.is_changed() {
        return;
    }
    if let Ok(mut t) = info.single_mut() {
        **t = town_info(&game.0);
    }
    if let Ok(mut t) = h.single_mut() {
        **t = hint(&game.0);
    }
}

fn town_actions(mut queue: ResMut<UiQueue>, mut overlay: ResMut<Overlay>, mut game: ResMut<Game>) {
    if !game.0.has_flag("intro") && std::env::var("ERRANCE_DEMO").is_err() {
        game.0.set_flag("intro");
        overlay.0 = Some(Panel::Intro);
    }
    let mut keep = vec![];
    for a in queue.take() {
        match a {
            UiAction::Building(b) => {
                overlay.0 = Some(match b.as_str() {
                    "corvees" => Panel::Chores,
                    "marche" => Panel::Market,
                    "taverne" => Panel::Tavern,
                    "porte" => Panel::Gate,
                    "atelier" => Panel::Workshop,
                    _ => Panel::Inn,
                });
            }
            other => keep.push(other),
        }
    }
    queue.0.extend(keep);
}

fn hide_under_overlay(ov: Res<Overlay>, mut q: Query<&mut Visibility, With<TownUi>>) {
    if !ov.is_changed() {
        return;
    }
    for mut v in q.iter_mut() {
        *v = if ov.0.is_some() { Visibility::Hidden } else { Visibility::Inherited };
    }
}
