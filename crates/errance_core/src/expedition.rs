//! Préparation d'une expédition : plan de la zone (arbres, rochers,
//! groupes de monstres) et mise en place du combat.

use crate::character::enemy_spec;
use crate::combat::{Arena, Combat, Controller, FighterId, FighterSpec, Team};
use crate::content::{Content, ZoneDef};
use crate::craft::{Node, NodeKind, roll_node_tier};
use crate::math::Vec2;
use crate::rng::Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Decor {
    Tree,
    Rock,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Layout {
    pub half: f32,
    /// (position, rayon de collision, type, échelle visuelle)
    pub decor: Vec<(Vec2, f32, Decor, f32)>,
    pub packs: Vec<(Vec2, Vec<String>)>,
    pub boss: Option<(Vec2, String)>,
    pub start: Vec2,
    /// Nœuds de récolte (arbres marqués, filons, plantes).
    #[serde(default)]
    pub nodes: Vec<Node>,
}

fn far_enough(p: Vec2, others: &[Vec2], d: f32) -> bool {
    others.iter().all(|o| o.distance(p) >= d)
}

/// Dessine le plan d'une zone. Le départ est au sud (+y, vers la caméra).
pub fn layout_zone(z: &ZoneDef, rng: &mut Rng) -> Layout {
    layout_inner(z, None, rng)
}

/// Plan d'une mission de quête : les groupes sont écrits, posés dans l'ordre.
pub fn layout_mission(z: &ZoneDef, packs: &[Vec<(String, u32)>], rng: &mut Rng) -> Layout {
    layout_inner(z, Some(packs), rng)
}

fn layout_inner(z: &ZoneDef, fixed: Option<&[Vec<(String, u32)>]>, rng: &mut Rng) -> Layout {
    let half = z.size;
    let start = Vec2::new(0.0, half - 4.0);
    let mut packs = vec![];
    let mut centers = vec![start];
    let boss = z.boss.as_ref().map(|b| (Vec2::new(0.0, -half + 6.0), b.clone()));
    if let Some((p, _)) = &boss {
        centers.push(*p);
    }
    let weights: Vec<f32> = z.packs.iter().map(|p| p.weight).collect();
    let mut tries = 0;
    let wanted = fixed.map(|f| f.len()).unwrap_or(z.pack_count as usize);
    while packs.len() < wanted && tries < 2000 {
        tries += 1;
        let p = Vec2::new(rng.range_f32(-half + 4.0, half - 4.0), rng.range_f32(-half + 4.0, half - 8.0));
        // Une mission doit tout placer : on desserre petit à petit.
        let spacing = if tries > 500 { 5.0 } else { 8.0 };
        if p.distance(start) < 12.0 || !far_enough(p, &centers, spacing) {
            continue;
        }
        let group: &[(String, u32)] = match fixed {
            Some(f) => &f[packs.len()],
            None => {
                let Some(k) = rng.weighted(&weights) else { break };
                &z.packs[k].enemies
            }
        };
        let mut names = vec![];
        for (id, n) in group {
            for _ in 0..*n {
                names.push(id.clone());
            }
        }
        centers.push(p);
        packs.push((p, names));
    }
    // Décor : arbres et rochers, loin des chemins importants.
    let area = (2.0 * half) * (2.0 * half);
    let n_trees = (z.trees * area / 100.0) as usize;
    let n_rocks = (area / 260.0) as usize;
    let mut decor = vec![];
    let mut placed: Vec<Vec2> = vec![];
    // Nœuds de récolte d'abord : ils doivent trouver leur place.
    let mut nodes = vec![];
    for (kind, n) in z.node_counts() {
        let mut k = 0;
        let mut tries = 0;
        while k < n && tries < n * 40 {
            tries += 1;
            let p = Vec2::new(rng.range_f32(-half + 2.0, half - 2.0), rng.range_f32(-half + 2.0, half - 2.0));
            if !far_enough(p, &centers, 5.0) || !far_enough(p, &placed, 3.0) {
                continue;
            }
            placed.push(p);
            let tier = roll_node_tier(z.tier, rng);
            nodes.push(Node { pos: p, kind, tier, left: rng.range_u32(3, 5) });
            k += 1;
        }
    }
    // Mission avec boss : les murs effondrés d'un fortin autour de lui, ouverts au sud.
    if let (Some(_), Some((bp, _))) = (fixed, &boss) {
        let n = 14;
        for k in 0..n {
            let a = k as f32 / n as f32 * std::f32::consts::TAU;
            let dir = Vec2::from_angle(a);
            // Brèche face au chemin d'arrivée (+y).
            if dir.y > 0.75 {
                continue;
            }
            let p = *bp + dir * 5.5;
            let scale = rng.range_f32(0.9, 1.3);
            placed.push(p);
            decor.push((p, 0.55 * scale, Decor::Rock, scale));
        }
    }
    let mut put = |kind: Decor, n: usize, rng: &mut Rng, decor: &mut Vec<(Vec2, f32, Decor, f32)>| {
        let mut k = 0;
        let mut tries = 0;
        while k < n && tries < n * 30 {
            tries += 1;
            let p = Vec2::new(rng.range_f32(-half, half), rng.range_f32(-half, half));
            if !far_enough(p, &centers, 4.5) || !far_enough(p, &placed, 1.6) {
                continue;
            }
            let scale = rng.range_f32(0.8, 1.35);
            let r = match kind {
                Decor::Tree => 0.45 * scale,
                Decor::Rock => 0.55 * scale,
            };
            placed.push(p);
            decor.push((p, r, kind, scale));
            k += 1;
        }
    };
    put(Decor::Tree, n_trees, rng, &mut decor);
    put(Decor::Rock, n_rocks, rng, &mut decor);
    // Lisière : une rangée d'arbres hors des limites pour cadrer.
    let mut x = -half - 2.0;
    while x <= half + 2.0 {
        for y in [-half - 2.5, half + 2.5] {
            decor.push((Vec2::new(x + rng.range_f32(-0.8, 0.8), y + rng.range_f32(-1.0, 1.0)), 0.0, Decor::Tree, rng.range_f32(1.0, 1.5)));
        }
        for xx in [-half - 2.5, half + 2.5] {
            decor.push((Vec2::new(xx + rng.range_f32(-1.0, 1.0), x + rng.range_f32(-0.8, 0.8)), 0.0, Decor::Tree, rng.range_f32(1.0, 1.5)));
        }
        x += 3.2;
    }
    Layout { half, decor, packs, boss, start, nodes }
}

/// Crée le combat de l'expédition. Renvoie le combat et l'id du héros.
/// Si `party_leads` : le premier du groupe est le chef (mode porteur).
pub fn setup_combat(c: &Content, layout: &Layout, hero: FighterSpec, party: Vec<FighterSpec>, seed: u64, party_leads: bool) -> (Combat, FighterId, Vec<FighterId>) {
    let mut obstacles: Vec<(Vec2, f32)> = layout.decor.iter().filter(|d| d.1 > 0.0).map(|d| (d.0, d.1)).collect();
    // Arbres et filons bloquent le passage ; les plantes non.
    for n in &layout.nodes {
        if matches!(n.kind, NodeKind::Tree | NodeKind::Vein) {
            obstacles.push((n.pos, 0.5));
        }
    }
    let mut combat = Combat::new(seed, Arena { half_extent: Vec2::new(layout.half, layout.half), obstacles });
    let hero_controlled = hero.controller == Controller::Player;
    let hid = combat.spawn(hero, layout.start);
    let mut ids = vec![];
    for (k, spec) in party.into_iter().enumerate() {
        let off = Vec2::new((k as f32 - 1.0) * 1.3, 1.4);
        ids.push(combat.spawn(spec, layout.start + off));
    }
    if !hero_controlled || party_leads {
        combat.leader = ids.first().copied().or(Some(hid));
    }
    let mut rng = Rng::new(seed ^ 0xABCD);
    for (center, names) in &layout.packs {
        for (k, name) in names.iter().enumerate() {
            if let Some(e) = c.enemy(name) {
                let a = k as f32 * 2.1 + rng.f32();
                let p = *center + Vec2::from_angle(a) * (if names.len() > 1 { 1.3 } else { 0.0 });
                combat.spawn(enemy_spec(c, e), p);
            }
        }
    }
    if let Some((p, b)) = &layout.boss {
        if let Some(e) = c.enemy(b) {
            combat.spawn(enemy_spec(c, e), *p);
        }
    }
    let _ = Team::Ally;
    (combat, hid, ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::content;

    #[test]
    fn plan_de_zone() {
        let c = content();
        for z in &c.zones {
            let mut rng = Rng::new(3);
            let l = layout_zone(z, &mut rng);
            assert!(!l.packs.is_empty(), "{} sans monstres", z.id);
            assert!(l.decor.len() > 10);
            for (p, _) in &l.packs {
                assert!(p.distance(l.start) >= 12.0);
            }
            // Des ressources partout, au tier de la zone (à un cran près).
            assert!(l.nodes.len() >= 8, "{} : {} nœuds", z.id, l.nodes.len());
            for n in &l.nodes {
                assert!(n.tier.abs_diff(z.tier) <= 1);
                assert!(n.pos.x.abs() <= z.size && n.pos.y.abs() <= z.size);
            }
        }
    }
}
