//! Préparation d'une expédition : plan de la zone (arbres, rochers,
//! groupes de monstres) et mise en place du combat.

use crate::character::enemy_spec;
use crate::combat::{Arena, Combat, Controller, FighterId, FighterSpec, Team};
use crate::content::{Content, ZoneDef};
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
}

fn far_enough(p: Vec2, others: &[Vec2], d: f32) -> bool {
    others.iter().all(|o| o.distance(p) >= d)
}

/// Dessine le plan d'une zone. Le départ est au sud (+y, vers la caméra).
pub fn layout_zone(z: &ZoneDef, rng: &mut Rng) -> Layout {
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
    while packs.len() < z.pack_count as usize && tries < 500 {
        tries += 1;
        let p = Vec2::new(rng.range_f32(-half + 4.0, half - 4.0), rng.range_f32(-half + 4.0, half - 8.0));
        if p.distance(start) < 12.0 || !far_enough(p, &centers, 8.0) {
            continue;
        }
        let Some(k) = rng.weighted(&weights) else { break };
        let mut names = vec![];
        for (id, n) in &z.packs[k].enemies {
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
    Layout { half, decor, packs, boss, start }
}

/// Crée le combat de l'expédition. Renvoie le combat et l'id du héros.
pub fn setup_combat(c: &Content, layout: &Layout, hero: FighterSpec, party: Vec<FighterSpec>, seed: u64) -> (Combat, FighterId, Vec<FighterId>) {
    let obstacles = layout.decor.iter().filter(|d| d.1 > 0.0).map(|d| (d.0, d.1)).collect();
    let mut combat = Combat::new(seed, Arena { half_extent: Vec2::new(layout.half, layout.half), obstacles });
    let hero_controlled = hero.controller == Controller::Player;
    let hid = combat.spawn(hero, layout.start);
    let mut ids = vec![];
    for (k, spec) in party.into_iter().enumerate() {
        let off = Vec2::new((k as f32 - 1.0) * 1.3, 1.4);
        ids.push(combat.spawn(spec, layout.start + off));
    }
    if !hero_controlled {
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
        }
    }
}
