//! Habillage du sol des expéditions : herbe, fleurs, buissons, cailloux,
//! chemin de terre. Purement visuel (aucune collision).
//!
//! Performances (Mali du téléphone) : peu de couleurs différentes et des
//! formes partagées, pour que Bevy dessine des centaines d'objets d'un coup.

use crate::models::{Finish, Painter};
use bevy::prelude::*;
use errance_core::content::ZoneDef;
use errance_core::expedition::Layout;
use errance_core::look::Rgb;
use errance_core::math::Vec2 as V2;
use errance_core::rng::Rng;

fn shade(c: Rgb, k: f32) -> Rgb {
    ((c.0 * k).min(1.0), (c.1 * k).min(1.0), (c.2 * k).min(1.0))
}

/// Le style de chaque zone : ce qui pousse (ou traîne) par terre.
struct Style {
    /// Densité d'herbe (1 = prairie).
    grass: f32,
    /// Couleurs de fleurs (vide = pas de fleurs).
    flowers: Vec<Rgb>,
    /// Buissons par 100 m².
    bushes: f32,
    /// Monticules de neige (Givrecol).
    snow: bool,
    /// Braises qui luisent (Cratère).
    embers: bool,
    /// Champignons (marais, forêt).
    mushrooms: bool,
}

fn style(z: &ZoneDef) -> Style {
    match z.id.as_str() {
        "marches" => Style { grass: 1.2, flowers: vec![(0.95, 0.9, 0.4), (0.9, 0.45, 0.55), (0.85, 0.85, 0.95)], bushes: 0.35, snow: false, embers: false, mushrooms: false },
        "vergues" => Style { grass: 0.9, flowers: vec![(0.7, 0.5, 0.9)], bushes: 0.6, snow: false, embers: false, mushrooms: true },
        "collines" => Style { grass: 0.6, flowers: vec![(0.95, 0.75, 0.3)], bushes: 0.2, snow: false, embers: false, mushrooms: false },
        "morne_eau" => Style { grass: 1.0, flowers: vec![], bushes: 0.4, snow: false, embers: false, mushrooms: true },
        "givrecol" => Style { grass: 0.06, flowers: vec![], bushes: 0.15, snow: true, embers: false, mushrooms: false },
        "cratere" => Style { grass: 0.1, flowers: vec![], bushes: 0.05, snow: false, embers: true, mushrooms: false },
        _ => Style { grass: 0.5, flowers: vec![], bushes: 0.15, snow: false, embers: false, mushrooms: false },
    }
}

/// Habille le sol d'une zone. `scope` : marqueur de nettoyage à la sortie.
pub fn dress(p: &mut Painter, cmd: &mut Commands, meshes: &mut Assets<Mesh>, layout: &Layout, zone: &ZoneDef, rng: &mut Rng, scope: impl Bundle + Clone) {
    let light = cfg!(target_os = "android");
    let st = style(zone);
    let half = layout.half;
    let area = (2.0 * half) * (2.0 * half);
    let start = layout.start;
    let clear_of_camp = |x: f32, z: f32| V2::new(x, z).distance(start) > 3.5;

    // --- Chemin de terre : du camp vers le nord, en serpentant. ---
    let path_col = shade(zone.ground, 0.88);
    let path_mat = p.mat(path_col, Finish::Matte);
    let disc = p.lib.disc.clone();
    let mut pos = start;
    let mut dir_x = 0.0f32;
    let end_y = layout.boss.as_ref().map(|b| b.0.y).unwrap_or(-half + 4.0);
    while pos.y > end_y {
        dir_x = (dir_x + rng.range_f32(-0.25, 0.25)).clamp(-0.6, 0.6);
        pos = V2::new((pos.x + dir_x).clamp(-half + 3.0, half - 3.0), pos.y - 0.9);
        let r = rng.range_f32(0.9, 1.25);
        cmd.spawn((
            Mesh3d(disc.clone()),
            MeshMaterial3d(path_mat.clone()),
            Transform::from_xyz(pos.x, 0.012, pos.y).with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)).with_scale(Vec3::new(r, r * 0.8, 1.0)),
            scope.clone(),
        ));
    }

    // --- Herbe : des milliers de brins fusionnés en 16 objets (un par carré de terrain). ---
    let density = 6.0 * st.grass * if light { 0.6 } else { 1.0 };
    let grass_mat = p.mats.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.95,
        // Visible des deux côtés, mais sans retourner l'éclairage (sinon brins noirs vus de dos).
        cull_mode: None,
        double_sided: false,
        ..default()
    });
    let chunks = 4;
    let span = (2.0 * half + 8.0) / chunks as f32;
    for cx in 0..chunks {
        for cz in 0..chunks {
            let x0 = -half - 4.0 + cx as f32 * span;
            let z0 = -half - 4.0 + cz as f32 * span;
            let n = (span * span * density) as usize;
            let mut pos: Vec<[f32; 3]> = Vec::with_capacity(n * 3);
            let mut col: Vec<[f32; 4]> = Vec::with_capacity(n * 3);
            for _ in 0..n {
                let x = x0 + rng.f32() * span;
                let z = z0 + rng.f32() * span;
                if !clear_of_camp(x, z) {
                    continue;
                }
                let h = rng.range_f32(0.15, 0.42);
                let w = rng.range_f32(0.035, 0.06);
                let a = rng.range_f32(0.0, std::f32::consts::TAU);
                let (dx, dz) = (a.cos() * w, a.sin() * w);
                let lean = (rng.range_f32(-0.12, 0.12), rng.range_f32(-0.12, 0.12));
                pos.push([x - dx, 0.0, z - dz]);
                pos.push([x + dx, 0.0, z + dz]);
                pos.push([x + lean.0, h, z + lean.1]);
                // Pied plus sombre, pointe plus claire, teinte variable.
                let k = rng.range_f32(0.8, 1.15);
                let base = Color::srgb(zone.foliage.0 * k * 0.85, zone.foliage.1 * k * 0.85, zone.foliage.2 * k * 0.85).to_linear();
                let tip = Color::srgb((zone.foliage.0 * k * 1.25).min(1.0), (zone.foliage.1 * k * 1.25).min(1.0), (zone.foliage.2 * k * 1.25).min(1.0)).to_linear();
                col.push([base.red, base.green, base.blue, 1.0]);
                col.push([base.red, base.green, base.blue, 1.0]);
                col.push([tip.red, tip.green, tip.blue, 1.0]);
            }
            if pos.is_empty() {
                continue;
            }
            let normals = vec![[0.0f32, 1.0, 0.0]; pos.len()];
            let mesh = Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList, bevy::asset::RenderAssetUsages::default())
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
                .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
                .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, col);
            cmd.spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(grass_mat.clone()),
                Transform::default(),
                // Des milliers de petites ombres : chères et sombres, on s'en passe.
                bevy::light::NotShadowCaster,
                scope.clone(),
            ));
        }
    }

    // --- Fleurs : une tige et une tête colorée, en petits massifs. ---
    if !st.flowers.is_empty() {
        let stem = p.mat(shade(zone.foliage, 0.8), Finish::Matte);
        let heads: Vec<_> = st.flowers.iter().map(|c| p.mat(*c, Finish::Matte)).collect();
        let cube = p.lib.cube.clone();
        let ico = p.lib.ico.clone();
        let massifs = (area / 100.0 * 0.9 * if light { 0.7 } else { 1.0 }) as usize;
        for _ in 0..massifs {
            let cx = rng.range_f32(-half, half);
            let cz = rng.range_f32(-half, half);
            let head = heads[rng.index(heads.len())].clone();
            for _ in 0..rng.range_u32(3, 6) {
                let x = cx + rng.range_f32(-0.8, 0.8);
                let z = cz + rng.range_f32(-0.8, 0.8);
                if !clear_of_camp(x, z) {
                    continue;
                }
                let h = rng.range_f32(0.22, 0.38);
                cmd.spawn((Mesh3d(cube.clone()), MeshMaterial3d(stem.clone()), Transform::from_xyz(x, h / 2.0, z).with_scale(Vec3::new(0.025, h, 0.025)), scope.clone()));
                cmd.spawn((Mesh3d(ico.clone()), MeshMaterial3d(head.clone()), Transform::from_xyz(x, h, z).with_scale(Vec3::splat(0.09)), scope.clone()));
            }
        }
    }

    // --- Buissons : boules de feuillage écrasées. ---
    let bush_cols = [shade(zone.foliage, 0.75), shade(zone.foliage, 0.92)];
    let bush_mats: Vec<_> = bush_cols.iter().map(|c| p.mat(*c, Finish::Matte)).collect();
    let ico2 = p.lib.ico2.clone();
    let n_bush = (area / 100.0 * st.bushes) as usize;
    for _ in 0..n_bush {
        let x = rng.range_f32(-half, half);
        let z = rng.range_f32(-half, half);
        if !clear_of_camp(x, z) {
            continue;
        }
        let s = rng.range_f32(0.45, 0.85);
        let m = bush_mats[rng.index(2)].clone();
        let b = cmd.spawn((Transform::from_xyz(x, 0.0, z), Visibility::default(), scope.clone())).id();
        for k in 0..3 {
            let a = k as f32 * 2.0 + rng.f32();
            let part = cmd
                .spawn((Mesh3d(ico2.clone()), MeshMaterial3d(m.clone()), Transform::from_xyz(a.cos() * 0.3 * s, 0.25 * s, a.sin() * 0.3 * s).with_scale(Vec3::new(0.8, 0.6, 0.8) * s)))
                .id();
            cmd.entity(b).add_child(part);
        }
    }

    // --- Cailloux (sur la neige : des rochers sombres qui dépassent). ---
    let pebble = p.mat(shade(zone.rock, if st.snow { 0.7 } else { 0.9 }), Finish::Matte);
    let ico = p.lib.ico.clone();
    for _ in 0..(area / 100.0 * if st.snow { 3.0 } else { 1.2 }) as usize {
        let x = rng.range_f32(-half, half);
        let z = rng.range_f32(-half, half);
        let s = rng.range_f32(0.1, 0.28);
        cmd.spawn((
            Mesh3d(ico.clone()),
            MeshMaterial3d(pebble.clone()),
            Transform::from_xyz(x, s * 0.25, z).with_rotation(Quat::from_rotation_y(rng.range_f32(0.0, 6.3))).with_scale(Vec3::new(s * 1.3, s * 0.6, s)),
            scope.clone(),
        ));
    }

    // --- Spécialités de zone. ---
    if st.snow {
        let snow = p.mat((0.95, 0.96, 0.99), Finish::Matte);
        for _ in 0..(area / 100.0 * 1.5) as usize {
            let x = rng.range_f32(-half - 3.0, half + 3.0);
            let z = rng.range_f32(-half - 3.0, half + 3.0);
            let s = rng.range_f32(0.5, 1.6);
            cmd.spawn((Mesh3d(ico2.clone()), MeshMaterial3d(snow.clone()), Transform::from_xyz(x, 0.0, z).with_scale(Vec3::new(s * 1.4, s * 0.35, s)), scope.clone()));
        }
    }
    if st.embers {
        let ember = p.mat((1.0, 0.45, 0.1), Finish::Glow);
        for _ in 0..(area / 100.0 * 1.0) as usize {
            let x = rng.range_f32(-half, half);
            let z = rng.range_f32(-half, half);
            let s = rng.range_f32(0.06, 0.16);
            cmd.spawn((Mesh3d(ico.clone()), MeshMaterial3d(ember.clone()), Transform::from_xyz(x, 0.02, z).with_scale(Vec3::new(s, s * 0.5, s)), scope.clone()));
        }
    }
    if st.mushrooms {
        let cap = p.mat((0.75, 0.25, 0.2), Finish::Matte);
        let foot = p.mat((0.92, 0.88, 0.8), Finish::Matte);
        let cyl = p.lib.cyl6.clone();
        for _ in 0..(area / 100.0 * 0.6) as usize {
            let x = rng.range_f32(-half, half);
            let z = rng.range_f32(-half, half);
            let s = rng.range_f32(0.7, 1.3);
            cmd.spawn((Mesh3d(cyl.clone()), MeshMaterial3d(foot.clone()), Transform::from_xyz(x, 0.06 * s, z).with_scale(Vec3::new(0.05, 0.12, 0.05) * s), scope.clone()));
            cmd.spawn((Mesh3d(ico.clone()), MeshMaterial3d(cap.clone()), Transform::from_xyz(x, 0.13 * s, z).with_scale(Vec3::new(0.16, 0.08, 0.16) * s), scope.clone()));
        }
    }
}
