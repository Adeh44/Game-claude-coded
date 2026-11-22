//! Errance — le jeu (Bevy). Squelette v0.1 : un perso qui bouge.

use bevy::input::touch::Touches;
use bevy::prelude::*;
#[cfg(target_os = "android")]
use bevy::window::WindowMode;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};

#[bevy_main]
pub fn main() {
    let window = Window {
        title: "Errance".into(),
        #[cfg(target_os = "android")]
        mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
        #[cfg(not(target_os = "android"))]
        resolution: (1280u32, 720u32).into(),
        ..default()
    };
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin { primary_window: Some(window), ..default() }))
        .add_systems(Startup, setup)
        .add_systems(Update, (move_player, auto_screenshot))
        .run();
}

#[derive(Component)]
struct Player;

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(40.0, 40.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.32, 0.45, 0.25))),
    ));
    commands.spawn((
        Player,
        Mesh3d(meshes.add(Capsule3d::new(0.35, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.6, 0.4))),
        Transform::from_xyz(0.0, 0.85, 0.0),
    ));
    commands.spawn((
        DirectionalLight { illuminance: 9000.0, ..default() },
        Transform::from_xyz(4.0, 10.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 12.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        #[cfg(target_os = "android")]
        Msaa::Off,
    ));
}

fn move_player(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    touches: Res<Touches>,
    mut player: Single<&mut Transform, With<Player>>,
) {
    let mut dir = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) { dir.y -= 1.0; }
    if keys.pressed(KeyCode::KeyS) { dir.y += 1.0; }
    if keys.pressed(KeyCode::KeyA) { dir.x -= 1.0; }
    if keys.pressed(KeyCode::KeyD) { dir.x += 1.0; }
    for t in touches.iter() {
        let d = t.position() - t.start_position();
        if d.length() > 10.0 { dir += d.normalize(); }
    }
    let d = dir.normalize_or_zero() * 5.0 * time.delta_secs();
    player.translation.x += d.x;
    player.translation.z += d.y;
}

/// Outil de test : ERRANCE_SCREENSHOT=chemin.png → capture puis quitte.
fn auto_screenshot(mut commands: Commands, mut frame: Local<u32>, mut exit: MessageWriter<AppExit>) {
    let Ok(path) = std::env::var("ERRANCE_SCREENSHOT") else { return };
    *frame += 1;
    if *frame == 60 {
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
    }
    if *frame == 90 {
        exit.write(AppExit::Success);
    }
}
