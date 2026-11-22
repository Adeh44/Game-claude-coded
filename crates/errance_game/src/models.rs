//! Modèles 3D low-poly fabriqués par le code (aucun fichier à charger).
//!
//! Style « figurines peintes » : formes simples, faces plates, couleurs
//! franches. Un personnage est assemblé de pièces ; ce qu'il porte se voit.

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use errance_core::look::{Body, Look, Rgb};

/// Rôle d'une pièce animée.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LimbRole {
    Body,
    LegL,
    LegR,
    ArmL,
    ArmR,
    /// Pattes de créature : avant gauche/droite, arrière gauche/droite.
    PawFL,
    PawFR,
    PawBL,
    PawBR,
    Tail,
    Head,
    /// Pattes d'araignée (avec un décalage de phase).
    SpiderLeg(i8),
    /// Flamme (torche) : vacille.
    Flame,
}

#[derive(Component)]
pub struct Limb {
    pub role: LimbRole,
    pub owner: Entity,
    pub rest: Transform,
}

/// Bibliothèque de formes et de couleurs partagées.
#[derive(Resource)]
pub struct Lib {
    pub cube: Handle<Mesh>,
    pub ico: Handle<Mesh>,
    pub ico2: Handle<Mesh>,
    pub cyl6: Handle<Mesh>,
    pub cyl10: Handle<Mesh>,
    pub cone6: Handle<Mesh>,
    pub cone4: Handle<Mesh>,
    pub disc: Handle<Mesh>,
    pub ring: Handle<Mesh>,
    pub plane: Handle<Mesh>,
    mats: HashMap<(u32, u8), Handle<StandardMaterial>>,
}

fn flat(mut m: Mesh) -> Mesh {
    m.duplicate_vertices();
    m.compute_flat_normals();
    m
}

impl Lib {
    pub fn new(meshes: &mut Assets<Mesh>) -> Self {
        Self {
            cube: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
            ico: meshes.add(flat(Sphere::new(0.5).mesh().ico(1).unwrap())),
            ico2: meshes.add(flat(Sphere::new(0.5).mesh().ico(2).unwrap())),
            cyl6: meshes.add(flat(Cylinder::new(0.5, 1.0).mesh().resolution(6).build())),
            cyl10: meshes.add(flat(Cylinder::new(0.5, 1.0).mesh().resolution(10).build())),
            cone6: meshes.add(flat(Cone::new(0.5, 1.0).mesh().resolution(6).build())),
            cone4: meshes.add(flat(Cone::new(0.5, 1.0).mesh().resolution(4).build())),
            disc: meshes.add(Circle::new(1.0).mesh().resolution(40).build()),
            ring: meshes.add(Annulus::new(0.85, 1.0).mesh().resolution(40).build()),
            plane: meshes.add(Plane3d::default().mesh().size(1.0, 1.0)),
            mats: HashMap::default(),
        }
    }
}

/// Type de matériau.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Finish {
    Matte,
    Metal,
    Glow,
    Ghost,
}

pub fn key(c: Rgb) -> u32 {
    let q = |v: f32| ((v.clamp(0.0, 1.0) * 63.0).round() as u32) & 63;
    (q(c.0) << 12) | (q(c.1) << 6) | q(c.2)
}

pub fn col(c: Rgb) -> Color {
    Color::srgb(c.0, c.1, c.2)
}

/// Fabrique des pièces de modèles (meshes + matériaux en cache).
pub struct Painter<'a> {
    pub lib: &'a mut Lib,
    pub mats: &'a mut Assets<StandardMaterial>,
}

impl Painter<'_> {
    pub fn mat(&mut self, c: Rgb, finish: Finish) -> Handle<StandardMaterial> {
        let k = (key(c), finish as u8);
        if let Some(h) = self.lib.mats.get(&k) {
            return h.clone();
        }
        let base = col(c);
        let m = match finish {
            Finish::Matte => StandardMaterial { base_color: base, perceptual_roughness: 0.92, ..default() },
            Finish::Metal => StandardMaterial { base_color: base, perceptual_roughness: 0.45, metallic: 0.5, ..default() },
            Finish::Glow => StandardMaterial {
                base_color: base,
                emissive: LinearRgba::from(base) * 4.0,
                unlit: false,
                ..default()
            },
            Finish::Ghost => StandardMaterial {
                base_color: base.with_alpha(0.45),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                double_sided: true,
                cull_mode: None,
                ..default()
            },
        };
        let h = self.mats.add(m);
        self.lib.mats.insert(k, h.clone());
        h
    }

    /// Un morceau : forme + couleur + transform, enfant de `parent`.
    pub fn part(&mut self, cmd: &mut Commands, parent: Entity, mesh: &Handle<Mesh>, c: Rgb, finish: Finish, t: Transform) -> Entity {
        let m = self.mat(c, finish);
        let e = cmd.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(m), t)).id();
        cmd.entity(parent).add_child(e);
        e
    }

    /// Un pivot vide (pour articuler un membre).
    pub fn pivot(&mut self, cmd: &mut Commands, parent: Entity, t: Transform, role: Option<(LimbRole, Entity)>) -> Entity {
        let mut ec = cmd.spawn((t, Visibility::default()));
        if let Some((role, owner)) = role {
            ec.insert(Limb { role, owner, rest: t });
        }
        let e = ec.id();
        cmd.entity(parent).add_child(e);
        e
    }

    pub fn cube(&mut self, cmd: &mut Commands, p: Entity, c: Rgb, f: Finish, pos: Vec3, size: Vec3) -> Entity {
        let m = self.lib.cube.clone();
        self.part(cmd, p, &m, c, f, Transform::from_translation(pos).with_scale(size))
    }

    pub fn ball(&mut self, cmd: &mut Commands, p: Entity, c: Rgb, f: Finish, pos: Vec3, size: Vec3) -> Entity {
        let m = self.lib.ico.clone();
        self.part(cmd, p, &m, c, f, Transform::from_translation(pos).with_scale(size))
    }

    pub fn ball2(&mut self, cmd: &mut Commands, p: Entity, c: Rgb, f: Finish, pos: Vec3, size: Vec3) -> Entity {
        let m = self.lib.ico2.clone();
        self.part(cmd, p, &m, c, f, Transform::from_translation(pos).with_scale(size))
    }

    pub fn cyl(&mut self, cmd: &mut Commands, p: Entity, c: Rgb, f: Finish, t: Transform) -> Entity {
        let m = self.lib.cyl6.clone();
        self.part(cmd, p, &m, c, f, t)
    }

    pub fn cone(&mut self, cmd: &mut Commands, p: Entity, c: Rgb, f: Finish, t: Transform) -> Entity {
        let m = self.lib.cone6.clone();
        self.part(cmd, p, &m, c, f, t)
    }
}

fn darker(c: Rgb, k: f32) -> Rgb {
    (c.0 * k, c.1 * k, c.2 * k)
}

const WOOD: Rgb = (0.42, 0.29, 0.17);
const LEATHER: Rgb = (0.35, 0.23, 0.14);

/// Couleur de l'élément d'un bâton.
fn gem_color(model: &str) -> Rgb {
    match model {
        m if m.contains("feu") => (1.0, 0.45, 0.1),
        m if m.contains("givre") => (0.45, 0.85, 1.0),
        m if m.contains("arcane") => (0.75, 0.4, 1.0),
        m if m.contains("sacre") => (1.0, 0.88, 0.45),
        m if m.contains("nature") => (0.35, 0.9, 0.35),
        m if m.contains("maudit") => (0.6, 0.2, 0.7),
        _ => (0.8, 0.8, 1.0),
    }
}

/// Arme dans la main (repère : la main est à l'origine, l'arme pointe vers +Y puis on la couche vers l'avant).
pub fn build_weapon(p: &mut Painter, cmd: &mut Commands, hand: Entity, model: &str, c: Rgb) {
    let f = Finish::Metal;
    let w = Finish::Matte;
    match model {
        "epee" | "epee_longue" | "claymore" => {
            let len = match model {
                "epee" => 0.6,
                "epee_longue" => 0.8,
                _ => 1.05,
            };
            p.cube(cmd, hand, LEATHER, w, Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.05, 0.16, 0.05));
            p.cube(cmd, hand, (0.75, 0.6, 0.25), f, Vec3::new(0.0, 0.1, 0.0), Vec3::new(0.24, 0.04, 0.06));
            p.cube(cmd, hand, c, f, Vec3::new(0.0, 0.12 + len / 2.0, 0.0), Vec3::new(0.07, len, 0.02));
        }
        "hache" | "grande_hache" => {
            let len = if model == "hache" { 0.6 } else { 1.0 };
            p.cube(cmd, hand, WOOD, w, Vec3::new(0.0, len / 2.0 - 0.1, 0.0), Vec3::new(0.05, len, 0.05));
            let s = if model == "hache" { 1.0 } else { 1.5 };
            p.cube(cmd, hand, c, f, Vec3::new(0.1 * s, len - 0.15, 0.0), Vec3::new(0.2 * s, 0.22 * s, 0.03));
            if model == "grande_hache" {
                p.cube(cmd, hand, c, f, Vec3::new(-0.12 * s, len - 0.15, 0.0), Vec3::new(0.16 * s, 0.18 * s, 0.03));
            }
        }
        "gourdin" => {
            p.cyl(cmd, hand, c, w, Transform::from_xyz(0.0, 0.28, 0.0).with_scale(Vec3::new(0.08, 0.6, 0.08)));
            p.ball(cmd, hand, c, w, Vec3::new(0.0, 0.58, 0.0), Vec3::splat(0.16));
        }
        "masse" | "morgenstern" => {
            let len = if model == "masse" { 0.55 } else { 0.85 };
            p.cyl(cmd, hand, WOOD, w, Transform::from_xyz(0.0, len / 2.0 - 0.05, 0.0).with_scale(Vec3::new(0.05, len, 0.05)));
            p.ball(cmd, hand, c, f, Vec3::new(0.0, len, 0.0), Vec3::splat(if model == "masse" { 0.18 } else { 0.26 }));
            if model == "morgenstern" {
                for i in 0..4 {
                    let a = i as f32 * std::f32::consts::FRAC_PI_2;
                    p.cone(cmd, hand, c, f, Transform::from_xyz(a.cos() * 0.13, len, a.sin() * 0.13)
                        .with_rotation(Quat::from_rotation_arc(Vec3::Y, Vec3::new(a.cos(), 0.0, a.sin())))
                        .with_scale(Vec3::new(0.06, 0.12, 0.06)));
                }
            }
        }
        "marteau" => {
            p.cyl(cmd, hand, WOOD, w, Transform::from_xyz(0.0, 0.45, 0.0).with_scale(Vec3::new(0.05, 1.1, 0.05)));
            p.cube(cmd, hand, c, f, Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.38, 0.2, 0.2));
        }
        "lance" | "hallebarde" | "fourche" => {
            p.cyl(cmd, hand, WOOD, w, Transform::from_xyz(0.0, 0.5, 0.0).with_scale(Vec3::new(0.045, 1.6, 0.045)));
            match model {
                "fourche" => {
                    for dx in [-0.08, 0.0, 0.08] {
                        p.cube(cmd, hand, (0.5, 0.5, 0.5), f, Vec3::new(dx, 1.38, 0.0), Vec3::new(0.02, 0.22, 0.02));
                    }
                    p.cube(cmd, hand, (0.5, 0.5, 0.5), f, Vec3::new(0.0, 1.27, 0.0), Vec3::new(0.2, 0.025, 0.025));
                }
                "hallebarde" => {
                    p.cone(cmd, hand, c, f, Transform::from_xyz(0.0, 1.42, 0.0).with_scale(Vec3::new(0.08, 0.3, 0.04)));
                    p.cube(cmd, hand, c, f, Vec3::new(0.1, 1.22, 0.0), Vec3::new(0.2, 0.22, 0.03));
                }
                _ => {
                    p.cone(cmd, hand, c, f, Transform::from_xyz(0.0, 1.42, 0.0).with_scale(Vec3::new(0.09, 0.28, 0.04)));
                }
            }
        }
        "dague" | "dagues_jumelles" => {
            p.cube(cmd, hand, LEATHER, w, Vec3::ZERO, Vec3::new(0.04, 0.12, 0.04));
            p.cube(cmd, hand, c, f, Vec3::new(0.0, 0.24, 0.0), Vec3::new(0.05, 0.32, 0.015));
        }
        "arc" | "arc_long" => {
            let h = if model == "arc" { 0.55 } else { 0.75 };
            // L'arc est tenu verticalement, les branches courbées vers l'arrière.
            p.cube(cmd, hand, c, w, Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.05, 0.18, 0.05));
            for s in [-1.0f32, 1.0] {
                p.part(cmd, hand, &p.lib.cube.clone(), c, w,
                    Transform::from_xyz(0.0, s * h * 0.5, -0.07)
                        .with_rotation(Quat::from_rotation_x(s * 0.35))
                        .with_scale(Vec3::new(0.04, h, 0.04)));
            }
            p.cube(cmd, hand, (0.9, 0.9, 0.85), w, Vec3::new(0.0, 0.0, -0.17), Vec3::new(0.008, h * 1.75, 0.008));
        }
        "arbalete" | "arbalete_lourde" => {
            let s = if model == "arbalete" { 1.0 } else { 1.3 };
            p.cube(cmd, hand, WOOD, w, Vec3::new(0.0, 0.25 * s, 0.0), Vec3::new(0.07, 0.6 * s, 0.07));
            p.cube(cmd, hand, c, f, Vec3::new(0.0, 0.5 * s, 0.0), Vec3::new(0.6 * s, 0.05, 0.05));
        }
        "baguette" | "sceptre" => {
            let mc = if model == "sceptre" { c } else { WOOD };
            p.cyl(cmd, hand, mc, if model == "sceptre" { f } else { w }, Transform::from_xyz(0.0, 0.2, 0.0).with_scale(Vec3::new(0.04, 0.5, 0.04)));
            p.ball(cmd, hand, gem_color(model), Finish::Glow, Vec3::new(0.0, 0.48, 0.0), Vec3::splat(0.1));
        }
        m if m.starts_with("baton") => {
            p.cyl(cmd, hand, c, w, Transform::from_xyz(0.0, 0.5, 0.0).with_scale(Vec3::new(0.05, 1.6, 0.05)));
            let g = gem_color(m);
            p.ball(cmd, hand, g, Finish::Glow, Vec3::new(0.0, 1.38, 0.0), Vec3::splat(0.17));
            p.cube(cmd, hand, darker(c, 0.8), w, Vec3::new(0.0, 1.27, 0.0), Vec3::new(0.16, 0.05, 0.05));
        }
        _ => {
            p.cube(cmd, hand, c, f, Vec3::new(0.0, 0.3, 0.0), Vec3::new(0.06, 0.6, 0.06));
        }
    }
}

pub fn build_offhand(p: &mut Painter, cmd: &mut Commands, hand: Entity, model: &str, c: Rgb, owner: Entity) {
    match model {
        "bouclier_rond" => {
            p.part(cmd, hand, &p.lib.cyl10.clone(), c, Finish::Matte,
                Transform::from_xyz(0.0, 0.05, 0.08).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)).with_scale(Vec3::new(0.5, 0.05, 0.5)));
            p.ball(cmd, hand, (0.6, 0.6, 0.62), Finish::Metal, Vec3::new(0.0, 0.05, 0.12), Vec3::splat(0.1));
        }
        "ecu" => {
            p.cube(cmd, hand, c, Finish::Metal, Vec3::new(0.0, 0.0, 0.09), Vec3::new(0.42, 0.6, 0.05));
            p.cube(cmd, hand, (0.7, 0.15, 0.12), Finish::Matte, Vec3::new(0.0, 0.0, 0.12), Vec3::new(0.08, 0.45, 0.02));
        }
        "tome" => {
            p.cube(cmd, hand, c, Finish::Matte, Vec3::new(0.0, 0.0, 0.08), Vec3::new(0.25, 0.32, 0.08));
            p.cube(cmd, hand, (0.95, 0.92, 0.8), Finish::Matte, Vec3::new(0.0, 0.0, 0.08), Vec3::new(0.23, 0.3, 0.085));
        }
        "torche" => {
            p.cyl(cmd, hand, WOOD, Finish::Matte, Transform::from_xyz(0.0, 0.2, 0.0).with_scale(Vec3::new(0.05, 0.5, 0.05)));
            let flame = p.pivot(cmd, hand, Transform::from_xyz(0.0, 0.5, 0.0), Some((LimbRole::Flame, owner)));
            p.cone(cmd, flame, (1.0, 0.6, 0.15), Finish::Glow, Transform::from_xyz(0.0, 0.08, 0.0).with_scale(Vec3::new(0.14, 0.25, 0.14)));
            p.ball(cmd, flame, (1.0, 0.85, 0.3), Finish::Glow, Vec3::new(0.0, 0.02, 0.0), Vec3::splat(0.1));
        }
        "orbe" => {
            p.ball2(cmd, hand, (0.6, 0.95, 0.95), Finish::Glow, Vec3::new(0.0, 0.12, 0.05), Vec3::splat(0.2));
        }
        _ => {
            p.cube(cmd, hand, c, Finish::Matte, Vec3::new(0.0, 0.0, 0.08), Vec3::new(0.3, 0.3, 0.05));
        }
    }
}

/// Construit le modèle d'un combattant. Renvoie la racine.
pub fn spawn_look(p: &mut Painter, cmd: &mut Commands, look: &Look, at: Vec3) -> Entity {
    let root = cmd.spawn((Transform::from_translation(at).with_scale(Vec3::splat(look.scale)), Visibility::default())).id();
    match &look.body {
        Body::Humanoid { skin, hair, hair_style } => build_humanoid(p, cmd, root, look, *skin, *hair, *hair_style, false),
        Body::Creature { model } => match model.as_str() {
            "squelette" => build_humanoid(p, cmd, root, look, (0.88, 0.86, 0.78), (0.88, 0.86, 0.78), 9, true),
            "troll" => build_troll(p, cmd, root, look.color),
            "araignee" => build_spider(p, cmd, root, look.color),
            "crapaud" => build_toad(p, cmd, root, look.color),
            m => build_quadruped(p, cmd, root, m, look.color),
        },
    }
    root
}

#[allow(clippy::too_many_arguments)]
fn build_humanoid(p: &mut Painter, cmd: &mut Commands, root: Entity, look: &Look, skin: Rgb, hair: Rgb, hair_style: u8, skeleton: bool) {
    let body = p.pivot(cmd, root, Transform::default(), Some((LimbRole::Body, root)));
    let thin = if skeleton { 0.55 } else { 1.0 };
    let chest = look.part("chest");
    let chest_model = chest.map(|c| c.model.as_str()).unwrap_or("");
    let chest_c = chest.map(|c| c.color).unwrap_or(look.color);
    let pants = if skeleton { skin } else if chest_model == "robe" || chest_model == "haillons" { darker(chest_c, 0.7) } else { (0.3, 0.25, 0.2) };
    let feet = look.part("feet");
    let boot_c = feet.map(|f| f.color).unwrap_or(if skeleton { skin } else { (0.25, 0.18, 0.12) });
    // Jambes.
    for (role, x) in [(LimbRole::LegL, -0.11), (LimbRole::LegR, 0.11)] {
        let hip = p.pivot(cmd, body, Transform::from_xyz(x, 0.74, 0.0), Some((role, root)));
        p.cube(cmd, hip, pants, Finish::Matte, Vec3::new(0.0, -0.3, 0.0), Vec3::new(0.16 * thin, 0.6, 0.17 * thin));
        let boot_size = match feet.map(|f| f.model.as_str()) {
            Some("solerets") => Vec3::new(0.2, 0.2, 0.28),
            Some("sandales") => Vec3::new(0.17, 0.06, 0.24),
            _ => Vec3::new(0.18, 0.16, 0.25),
        };
        let fin = if feet.map(|f| f.model == "solerets").unwrap_or(false) { Finish::Metal } else { Finish::Matte };
        p.cube(cmd, hip, boot_c, fin, Vec3::new(0.0, -0.66, 0.03), boot_size);
    }
    // Torse.
    let torso_c = if skeleton { skin } else { chest_c };
    let torso_fin = if matches!(chest_model, "cotte" | "plastron") { Finish::Metal } else { Finish::Matte };
    p.cube(cmd, body, torso_c, torso_fin, Vec3::new(0.0, 1.05, 0.0), Vec3::new(0.46 * thin.max(0.75), 0.6, 0.26 * thin.max(0.7)));
    match chest_model {
        "cotte" | "plastron" => {
            for x in [-0.27, 0.27] {
                p.ball(cmd, body, darker(chest_c, 0.9), Finish::Metal, Vec3::new(x, 1.3, 0.0), Vec3::new(0.24, 0.16, 0.26));
            }
            p.cube(cmd, body, LEATHER, Finish::Matte, Vec3::new(0.0, 0.8, 0.0), Vec3::new(0.48, 0.07, 0.28));
        }
        "veste_cuir" | "gambison" => {
            p.cube(cmd, body, darker(chest_c, 0.7), Finish::Matte, Vec3::new(0.0, 0.82, 0.0), Vec3::new(0.48, 0.06, 0.28));
            p.cube(cmd, body, darker(chest_c, 0.85), Finish::Matte, Vec3::new(0.0, 1.2, 0.0), Vec3::new(0.08, 0.36, 0.27));
        }
        "robe" => {
            p.cone(cmd, body, chest_c, Finish::Matte, Transform::from_xyz(0.0, 0.5, 0.0).with_scale(Vec3::new(0.62, 0.75, 0.5)));
            p.cube(cmd, body, (0.75, 0.6, 0.3), Finish::Matte, Vec3::new(0.0, 0.82, 0.0), Vec3::new(0.47, 0.05, 0.27));
        }
        "haillons" => {
            p.cube(cmd, body, darker(chest_c, 0.8), Finish::Matte, Vec3::new(0.1, 0.72, 0.1), Vec3::new(0.14, 0.14, 0.04));
            p.cube(cmd, body, darker(chest_c, 0.75), Finish::Matte, Vec3::new(-0.12, 0.7, -0.1), Vec3::new(0.12, 0.18, 0.04));
        }
        _ => {}
    }
    if skeleton {
        for y in [0.95, 1.05, 1.15] {
            p.cube(cmd, body, darker(skin, 0.6), Finish::Matte, Vec3::new(0.0, y, 0.13), Vec3::new(0.3, 0.03, 0.02));
        }
    }
    // Cape.
    if let Some(cape) = look.part("cape") {
        p.cube(cmd, body, cape.color, Finish::Matte, Vec3::new(0.0, 0.92, -0.17), Vec3::new(0.44, 0.78, 0.04));
    }
    // Tête.
    let head = p.pivot(cmd, body, Transform::from_xyz(0.0, 1.52, 0.0), Some((LimbRole::Head, root)));
    p.ball2(cmd, head, skin, Finish::Matte, Vec3::ZERO, Vec3::new(0.34, 0.36, 0.34));
    if skeleton {
        for x in [-0.06, 0.06] {
            p.cube(cmd, head, (0.1, 0.08, 0.08), Finish::Matte, Vec3::new(x, 0.02, 0.15), Vec3::splat(0.07));
        }
    } else {
        for x in [-0.06, 0.06] {
            p.cube(cmd, head, (0.12, 0.1, 0.1), Finish::Matte, Vec3::new(x, 0.03, 0.16), Vec3::new(0.04, 0.05, 0.02));
        }
    }
    let helmet = look.part("head");
    match helmet.map(|h| h.model.as_str()) {
        Some("casque") | Some("casque_cornes") => {
            let h = helmet.unwrap();
            p.part(cmd, head, &p.lib.cyl10.clone(), h.color, Finish::Metal, Transform::from_xyz(0.0, 0.06, 0.0).with_scale(Vec3::new(0.4, 0.3, 0.4)));
            p.ball2(cmd, head, h.color, Finish::Metal, Vec3::new(0.0, 0.2, 0.0), Vec3::new(0.4, 0.22, 0.4));
            p.cube(cmd, head, darker(h.color, 0.8), Finish::Metal, Vec3::new(0.0, 0.0, 0.2), Vec3::new(0.04, 0.18, 0.03));
            if h.model == "casque_cornes" {
                for s in [-1.0f32, 1.0] {
                    p.cone(cmd, head, (0.92, 0.88, 0.75), Finish::Matte,
                        Transform::from_xyz(s * 0.24, 0.2, 0.0)
                            .with_rotation(Quat::from_rotation_z(-s * 0.9))
                            .with_scale(Vec3::new(0.08, 0.26, 0.08)));
                }
            }
        }
        Some("capuche") => {
            let h = helmet.unwrap();
            p.ball2(cmd, head, h.color, Finish::Matte, Vec3::new(0.0, 0.05, -0.04), Vec3::new(0.42, 0.42, 0.42));
            p.cone(cmd, head, h.color, Finish::Matte, Transform::from_xyz(0.0, 0.08, -0.22).with_rotation(Quat::from_rotation_x(-1.9)).with_scale(Vec3::new(0.16, 0.22, 0.16)));
        }
        Some("chapeau_mage") => {
            let h = helmet.unwrap();
            p.part(cmd, head, &p.lib.cyl10.clone(), h.color, Finish::Matte, Transform::from_xyz(0.0, 0.14, 0.0).with_scale(Vec3::new(0.62, 0.03, 0.62)));
            p.cone(cmd, head, h.color, Finish::Matte, Transform::from_xyz(0.0, 0.4, -0.03).with_rotation(Quat::from_rotation_x(-0.15)).with_scale(Vec3::new(0.32, 0.55, 0.32)));
        }
        _ if !skeleton => {
            // Cheveux.
            match hair_style % 3 {
                0 => {
                    p.ball2(cmd, head, hair, Finish::Matte, Vec3::new(0.0, 0.07, -0.03), Vec3::new(0.36, 0.3, 0.36));
                }
                1 => {
                    p.ball2(cmd, head, hair, Finish::Matte, Vec3::new(0.0, 0.08, -0.02), Vec3::new(0.37, 0.26, 0.37));
                    p.cube(cmd, head, hair, Finish::Matte, Vec3::new(0.0, -0.1, -0.15), Vec3::new(0.26, 0.3, 0.08));
                }
                _ => {
                    p.ball2(cmd, head, hair, Finish::Matte, Vec3::new(0.0, 0.12, -0.02), Vec3::new(0.3, 0.16, 0.3));
                    p.cube(cmd, head, hair, Finish::Matte, Vec3::new(0.0, -0.13, 0.12), Vec3::new(0.2, 0.12, 0.1));
                }
            }
        }
        _ => {}
    }
    // Bras.
    let sleeve = if skeleton { skin } else if chest.is_some() { darker(chest_c, 0.92) } else { look.color };
    for (role, x) in [(LimbRole::ArmL, -0.31), (LimbRole::ArmR, 0.31)] {
        let sh = p.pivot(cmd, body, Transform::from_xyz(x, 1.3, 0.0), Some((role, root)));
        p.cube(cmd, sh, sleeve, Finish::Matte, Vec3::new(0.0, -0.22, 0.0), Vec3::new(0.12 * thin.max(0.7), 0.44, 0.13 * thin.max(0.7)));
        p.cube(cmd, sh, skin, Finish::Matte, Vec3::new(0.0, -0.5, 0.0), Vec3::new(0.11, 0.12, 0.11));
        // Main : l'arme est tenue vers l'avant.
        let hand = p.pivot(cmd, sh, Transform::from_xyz(0.0, -0.52, 0.04).with_rotation(Quat::from_rotation_x(1.35)), None);
        if role == LimbRole::ArmR {
            if let Some(w) = look.part("main") {
                build_weapon(p, cmd, hand, &w.model, w.color);
            }
        } else if let Some(o) = look.part("off") {
            let off = p.pivot(cmd, sh, Transform::from_xyz(-0.06, -0.45, 0.06), None);
            build_offhand(p, cmd, off, &o.model, o.color, root);
        } else if let Some(w) = look.part("main") {
            if w.model == "dagues_jumelles" {
                build_weapon(p, cmd, hand, "dague", w.color);
            }
        }
    }
}

fn build_quadruped(p: &mut Painter, cmd: &mut Commands, root: Entity, model: &str, c: Rgb) {
    // (longueur, hauteur, largeur du corps, hauteur des pattes)
    let (len, h, w, leg) = match model {
        "rat" => (0.62, 0.26, 0.28, 0.12),
        "sanglier" => (0.95, 0.5, 0.5, 0.3),
        "ours" => (1.25, 0.72, 0.7, 0.38),
        _ => (0.95, 0.38, 0.32, 0.42), // loup
    };
    let body = p.pivot(cmd, root, Transform::default(), Some((LimbRole::Body, root)));
    let y = leg + h / 2.0;
    p.ball2(cmd, body, c, Finish::Matte, Vec3::new(0.0, y, 0.0), Vec3::new(w, h, len));
    // Tête.
    let head = p.pivot(cmd, body, Transform::from_xyz(0.0, y + h * 0.15, len * 0.5), Some((LimbRole::Head, root)));
    let hs = match model {
        "rat" => 0.2,
        "ours" => 0.42,
        "sanglier" => 0.36,
        _ => 0.3,
    };
    p.ball2(cmd, head, c, Finish::Matte, Vec3::new(0.0, 0.0, hs * 0.3), Vec3::new(hs, hs * 0.9, hs * 1.1));
    p.cube(cmd, head, darker(c, 0.85), Finish::Matte, Vec3::new(0.0, -hs * 0.12, hs * 0.85), Vec3::new(hs * 0.45, hs * 0.4, hs * 0.7));
    p.cube(cmd, head, (0.08, 0.06, 0.06), Finish::Matte, Vec3::new(0.0, -hs * 0.05, hs * 1.2), Vec3::splat(hs * 0.16));
    for s in [-1.0f32, 1.0] {
        p.cube(cmd, head, (0.9, 0.7, 0.2), Finish::Glow, Vec3::new(s * hs * 0.25, hs * 0.15, hs * 0.62), Vec3::splat(hs * 0.08));
        match model {
            "rat" => {
                p.ball(cmd, head, (0.75, 0.55, 0.55), Finish::Matte, Vec3::new(s * hs * 0.4, hs * 0.45, hs * 0.1), Vec3::new(hs * 0.35, hs * 0.35, hs * 0.1));
            }
            "ours" => {
                p.ball(cmd, head, darker(c, 0.8), Finish::Matte, Vec3::new(s * hs * 0.38, hs * 0.42, 0.0), Vec3::splat(hs * 0.25));
            }
            _ => {
                p.cone(cmd, head, darker(c, 0.85), Finish::Matte, Transform::from_xyz(s * hs * 0.28, hs * 0.5, 0.0).with_scale(Vec3::new(hs * 0.22, hs * 0.4, hs * 0.15)));
            }
        }
        if model == "sanglier" {
            p.cone(cmd, head, (0.95, 0.92, 0.8), Finish::Matte, Transform::from_xyz(s * hs * 0.25, -hs * 0.15, hs * 1.05).with_rotation(Quat::from_rotation_x(0.6)).with_scale(Vec3::new(0.05, 0.16, 0.05)));
        }
    }
    if model == "sanglier" {
        p.cube(cmd, body, darker(c, 0.6), Finish::Matte, Vec3::new(0.0, y + h * 0.5, 0.0), Vec3::new(0.08, 0.12, len * 0.8));
    }
    // Pattes.
    let legw = (w * 0.3).max(0.07);
    for (role, sx, sz) in [
        (LimbRole::PawFL, -1.0, 1.0),
        (LimbRole::PawFR, 1.0, 1.0),
        (LimbRole::PawBL, -1.0, -1.0),
        (LimbRole::PawBR, 1.0, -1.0),
    ] {
        let hip = p.pivot(cmd, body, Transform::from_xyz(sx * w * 0.3, leg, sz * len * 0.3), Some((role, root)));
        p.cube(cmd, hip, darker(c, 0.85), Finish::Matte, Vec3::new(0.0, -leg / 2.0, 0.0), Vec3::new(legw, leg + 0.04, legw));
    }
    // Queue.
    let tail = p.pivot(cmd, body, Transform::from_xyz(0.0, y + h * 0.1, -len * 0.48), Some((LimbRole::Tail, root)));
    let (tl, tc) = match model {
        "rat" => (0.55, (0.75, 0.55, 0.55)),
        "ours" => (0.12, c),
        "sanglier" => (0.2, darker(c, 0.7)),
        _ => (0.45, c),
    };
    p.cube(cmd, tail, tc, Finish::Matte, Vec3::new(0.0, 0.0, -tl / 2.0), Vec3::new(0.05 + (model == "loup") as u8 as f32 * 0.06, 0.05 + (model == "loup") as u8 as f32 * 0.06, tl));
}

fn build_spider(p: &mut Painter, cmd: &mut Commands, root: Entity, c: Rgb) {
    let body = p.pivot(cmd, root, Transform::default(), Some((LimbRole::Body, root)));
    p.ball2(cmd, body, c, Finish::Matte, Vec3::new(0.0, 0.45, -0.3), Vec3::new(0.65, 0.5, 0.75));
    p.ball2(cmd, body, darker(c, 1.3), Finish::Matte, Vec3::new(0.0, 0.4, 0.2), Vec3::new(0.38, 0.3, 0.4));
    p.cube(cmd, body, (0.7, 0.15, 0.1), Finish::Matte, Vec3::new(0.0, 0.62, -0.3), Vec3::new(0.12, 0.05, 0.4));
    for s in [-1.0f32, 1.0] {
        p.cube(cmd, body, (0.9, 0.1, 0.1), Finish::Glow, Vec3::new(s * 0.07, 0.48, 0.38), Vec3::splat(0.05));
    }
    for i in 0..8 {
        let side = if i < 4 { -1.0 } else { 1.0 };
        let k = (i % 4) as f32;
        let leg = p.pivot(cmd, body, Transform::from_xyz(side * 0.15, 0.42, 0.25 - k * 0.15)
            .with_rotation(Quat::from_rotation_y(side * (0.6 - k * 0.35))), Some((LimbRole::SpiderLeg(i as i8), root)));
        p.cube(cmd, leg, darker(c, 0.8), Finish::Matte, Vec3::new(side * 0.3, 0.08, 0.0), Vec3::new(0.55, 0.05, 0.05));
        p.cube(cmd, leg, darker(c, 0.8), Finish::Matte, Vec3::new(side * 0.58, -0.17, 0.0), Vec3::new(0.05, 0.5, 0.05));
    }
}

fn build_toad(p: &mut Painter, cmd: &mut Commands, root: Entity, c: Rgb) {
    let body = p.pivot(cmd, root, Transform::default(), Some((LimbRole::Body, root)));
    p.ball2(cmd, body, c, Finish::Matte, Vec3::new(0.0, 0.35, 0.0), Vec3::new(0.95, 0.6, 0.9));
    p.ball2(cmd, body, darker(c, 1.25), Finish::Matte, Vec3::new(0.0, 0.25, 0.15), Vec3::new(0.7, 0.35, 0.65));
    for s in [-1.0f32, 1.0] {
        p.ball(cmd, body, (0.95, 0.8, 0.2), Finish::Glow, Vec3::new(s * 0.22, 0.65, 0.25), Vec3::splat(0.16));
        let leg = p.pivot(cmd, body, Transform::from_xyz(s * 0.4, 0.2, -0.2), Some((if s < 0.0 { LimbRole::PawBL } else { LimbRole::PawBR }, root)));
        p.ball(cmd, leg, darker(c, 0.85), Finish::Matte, Vec3::ZERO, Vec3::new(0.3, 0.25, 0.45));
    }
}

fn build_troll(p: &mut Painter, cmd: &mut Commands, root: Entity, c: Rgb) {
    let body = p.pivot(cmd, root, Transform::default(), Some((LimbRole::Body, root)));
    for (role, x) in [(LimbRole::LegL, -0.16), (LimbRole::LegR, 0.16)] {
        let hip = p.pivot(cmd, body, Transform::from_xyz(x, 0.55, 0.0), Some((role, root)));
        p.cube(cmd, hip, darker(c, 0.85), Finish::Matte, Vec3::new(0.0, -0.27, 0.0), Vec3::new(0.22, 0.55, 0.24));
    }
    p.ball2(cmd, body, c, Finish::Matte, Vec3::new(0.0, 0.95, 0.05), Vec3::new(0.75, 0.8, 0.6));
    p.cube(cmd, body, (0.35, 0.25, 0.15), Finish::Matte, Vec3::new(0.0, 0.62, 0.0), Vec3::new(0.62, 0.18, 0.5));
    let head = p.pivot(cmd, body, Transform::from_xyz(0.0, 1.35, 0.2), Some((LimbRole::Head, root)));
    p.ball2(cmd, head, c, Finish::Matte, Vec3::ZERO, Vec3::new(0.38, 0.32, 0.38));
    p.cube(cmd, head, darker(c, 0.8), Finish::Matte, Vec3::new(0.0, -0.02, 0.18), Vec3::new(0.14, 0.12, 0.12));
    for s in [-1.0f32, 1.0] {
        p.cube(cmd, head, (1.0, 0.75, 0.2), Finish::Glow, Vec3::new(s * 0.09, 0.06, 0.17), Vec3::splat(0.05));
        p.cone(cmd, head, (0.95, 0.92, 0.8), Finish::Matte, Transform::from_xyz(s * 0.08, -0.12, 0.17).with_scale(Vec3::new(0.04, 0.1, 0.04)));
    }
    for (role, x) in [(LimbRole::ArmL, -0.45), (LimbRole::ArmR, 0.45)] {
        let sh = p.pivot(cmd, body, Transform::from_xyz(x, 1.2, 0.05), Some((role, root)));
        p.cube(cmd, sh, c, Finish::Matte, Vec3::new(0.0, -0.4, 0.0), Vec3::new(0.2, 0.8, 0.22));
        p.ball(cmd, sh, darker(c, 0.85), Finish::Matte, Vec3::new(0.0, -0.85, 0.0), Vec3::splat(0.26));
    }
}

/// Arbre (décor). `kind` : feuillu, sapin, mort, saule.
pub fn spawn_tree(p: &mut Painter, cmd: &mut Commands, at: Vec3, kind: &str, foliage: Rgb, scale: f32, rot: f32) -> Entity {
    let root = cmd.spawn((Transform::from_translation(at).with_rotation(Quat::from_rotation_y(rot)).with_scale(Vec3::splat(scale)), Visibility::default())).id();
    let trunk = (0.38, 0.27, 0.18);
    match kind {
        "sapin" => {
            p.cyl(cmd, root, trunk, Finish::Matte, Transform::from_xyz(0.0, 0.5, 0.0).with_scale(Vec3::new(0.25, 1.0, 0.25)));
            for (i, (y, r)) in [(1.2, 1.2), (2.0, 0.95), (2.7, 0.65)].iter().enumerate() {
                let c = darker(foliage, 0.85 + i as f32 * 0.08);
                p.cone(cmd, root, c, Finish::Matte, Transform::from_xyz(0.0, *y, 0.0).with_scale(Vec3::new(*r * 2.0, 1.3, *r * 2.0)));
            }
        }
        "mort" => {
            p.cyl(cmd, root, (0.35, 0.3, 0.25), Finish::Matte, Transform::from_xyz(0.0, 1.0, 0.0).with_scale(Vec3::new(0.22, 2.0, 0.22)));
            for (a, y, l) in [(0.8f32, 1.5, 0.9), (-0.9, 1.9, 0.7), (2.4, 1.2, 0.6)] {
                p.part(cmd, root, &p.lib.cube.clone(), (0.35, 0.3, 0.25), Finish::Matte,
                    Transform::from_xyz(a.cos() * l * 0.4, y, a.sin() * l * 0.4)
                        .with_rotation(Quat::from_rotation_y(-a) * Quat::from_rotation_z(0.9))
                        .with_scale(Vec3::new(0.08, l, 0.08)));
            }
        }
        "saule" => {
            p.cyl(cmd, root, trunk, Finish::Matte, Transform::from_xyz(0.0, 0.8, 0.0).with_scale(Vec3::new(0.32, 1.6, 0.32)));
            p.ball2(cmd, root, foliage, Finish::Matte, Vec3::new(0.0, 2.0, 0.0), Vec3::new(2.4, 1.3, 2.4));
            for i in 0..6 {
                let a = i as f32 * 1.047;
                p.cube(cmd, root, darker(foliage, 0.85), Finish::Matte, Vec3::new(a.cos() * 1.0, 1.3, a.sin() * 1.0), Vec3::new(0.35, 1.3, 0.35));
            }
        }
        _ => {
            p.cyl(cmd, root, trunk, Finish::Matte, Transform::from_xyz(0.0, 0.7, 0.0).with_scale(Vec3::new(0.28, 1.4, 0.28)));
            p.ball(cmd, root, foliage, Finish::Matte, Vec3::new(0.0, 1.9, 0.0), Vec3::new(1.9, 1.6, 1.9));
            p.ball(cmd, root, darker(foliage, 1.12), Finish::Matte, Vec3::new(0.45, 2.4, 0.2), Vec3::new(1.1, 1.0, 1.1));
            p.ball(cmd, root, darker(foliage, 0.9), Finish::Matte, Vec3::new(-0.4, 2.2, -0.3), Vec3::new(1.2, 1.0, 1.2));
        }
    }
    root
}

pub fn spawn_rock(p: &mut Painter, cmd: &mut Commands, at: Vec3, c: Rgb, size: Vec3, rot: f32) -> Entity {
    let root = cmd.spawn((Transform::from_translation(at).with_rotation(Quat::from_rotation_y(rot)), Visibility::default())).id();
    p.ball(cmd, root, c, Finish::Matte, Vec3::new(0.0, size.y * 0.3, 0.0), size);
    p.ball(cmd, root, darker(c, 0.88), Finish::Matte, Vec3::new(size.x * 0.3, size.y * 0.15, size.z * 0.2), size * 0.6);
    root
}

/// Animation procédurale de tous les membres.
#[derive(Clone, Copy, Default)]
pub struct AnimState {
    pub moving: bool,
    pub speed: f32,
    pub swing: f32,
    pub casting: bool,
    pub dead: bool,
    pub stunned: bool,
}

pub fn animate_limbs(time: f32, states: &HashMap<Entity, AnimState>, limbs: &mut Query<(&Limb, &mut Transform)>) {
    for (limb, mut t) in limbs.iter_mut() {
        let Some(s) = states.get(&limb.owner) else { continue };
        let rest = limb.rest;
        let phase = time * (6.0 + s.speed * 1.6);
        let walk = if s.moving && !s.stunned { 1.0 } else { 0.0 };
        let sw = (s.swing / 0.35).clamp(0.0, 1.0);
        let mut rot = Quat::IDENTITY;
        let mut offset = Vec3::ZERO;
        match limb.role {
            LimbRole::LegL | LimbRole::PawFL | LimbRole::PawBR => rot = Quat::from_rotation_x(phase.sin() * 0.6 * walk),
            LimbRole::LegR | LimbRole::PawFR | LimbRole::PawBL => rot = Quat::from_rotation_x(-phase.sin() * 0.6 * walk),
            LimbRole::ArmL => {
                rot = Quat::from_rotation_x(-phase.sin() * 0.4 * walk);
                if s.casting {
                    rot = Quat::from_rotation_x(-1.2 + (time * 10.0).sin() * 0.1);
                }
            }
            LimbRole::ArmR => {
                let swing = if sw > 0.0 { -2.2 * (sw * std::f32::consts::PI).sin() } else { 0.0 };
                rot = Quat::from_rotation_x(phase.sin() * 0.4 * walk + swing);
                if s.casting {
                    rot = Quat::from_rotation_x(-1.6 + (time * 9.0).cos() * 0.1);
                }
            }
            LimbRole::Body => {
                offset.y = (phase * 2.0).sin().abs() * 0.05 * walk;
                if sw > 0.0 {
                    rot = Quat::from_rotation_x(0.15 * (sw * std::f32::consts::PI).sin());
                }
                if s.stunned {
                    rot = Quat::from_rotation_z((time * 8.0).sin() * 0.08);
                }
            }
            LimbRole::Head => {
                if sw > 0.0 && limb.rest.translation.z > 0.1 {
                    // Créatures : coup de tête quand elles mordent.
                    offset.z = 0.15 * (sw * std::f32::consts::PI).sin();
                    rot = Quat::from_rotation_x(0.3 * (sw * std::f32::consts::PI).sin());
                }
            }
            LimbRole::Tail => rot = Quat::from_rotation_y((time * 5.0).sin() * 0.4),
            LimbRole::SpiderLeg(i) => {
                let ph = phase * 1.6 + i as f32 * 1.3;
                rot = Quat::from_rotation_z(ph.sin() * 0.25 * walk);
            }
            LimbRole::Flame => {
                let f = 1.0 + (time * 13.0 + (limb.owner.to_bits() % 97) as f32).sin() * 0.15;
                t.scale = Vec3::new(1.0, f, 1.0);
            }
        }
        if limb.role != LimbRole::Flame {
            t.translation = rest.translation + offset;
            t.rotation = rest.rotation * rot;
        }
    }
}
