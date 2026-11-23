//! Récolte et artisanat.
//!
//! En expédition, on récolte (bois, minerai, peaux, fibres) en restant près
//! d'un nœud pendant que la jauge se remplit. En ville, l'atelier transforme
//! ces ressources en équipement : recette = base + ressources du tier,
//! qualité tirée avec un bonus selon la maîtrise de la Forge.
//!
//! Deux Voies de métier : `recolte` (vitesse, rendement, tiers récoltables)
//! et `forge` (tiers fabricables, qualité).

use crate::content::{Content, EnemyDef};
use crate::item::{EquipSlot, Item, ItemBase, Material, tier_mult};
use crate::look::Rgb;
use crate::loot::{item_value, roll_affixes, roll_quality};
use crate::mastery::{LevelUp, level_for_tier};
use crate::math::Vec2;
use crate::profile::Profile;
use crate::rng::Rng;
use serde::{Deserialize, Serialize};

pub const LINE_GATHER: &str = "recolte";
pub const LINE_FORGE: &str = "forge";

/// Distance à laquelle on peut récolter un nœud (centre à centre).
pub const GATHER_RANGE: f32 = 1.9;

/// Où pousse / se trouve une ressource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum NodeKind {
    /// Arbre marqué d'une entaille : bois.
    Tree,
    /// Filon dans la roche : minerai.
    Vein,
    /// Touffe de plantes : fibre.
    Herb,
    /// Bête abattue : peau.
    Carcass,
}

impl NodeKind {
    pub fn verb(self) -> &'static str {
        match self {
            NodeKind::Tree => "Couper",
            NodeKind::Vein => "Miner",
            NodeKind::Herb => "Cueillir",
            NodeKind::Carcass => "Dépecer",
        }
    }
}

/// Une ressource (contenu).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceDef {
    pub id: String,
    pub name: String,
    pub node: NodeKind,
    /// Famille d'objets fabriqués avec.
    pub material: Material,
    /// Ressource d'appoint de ces recettes.
    pub secondary: String,
    pub color: Rgb,
    #[serde(default)]
    pub description: String,
    /// Nom à chaque tier (8 noms).
    pub tiers: Vec<String>,
}

impl ResourceDef {
    pub fn tier_name(&self, tier: u8) -> &str {
        let t = (tier.clamp(1, 8) - 1) as usize;
        self.tiers.get(t).map(|s| s.as_str()).unwrap_or(&self.name)
    }
}

/// Une pile de ressources dans la réserve du joueur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceStack {
    pub id: String,
    pub tier: u8,
    pub count: u32,
}

/// Prix de vente d'une unité (au marchand de l'atelier).
pub fn resource_unit_value(tier: u8) -> u32 {
    let v = 4.0 * 3.2f32.powi(tier.clamp(1, 8) as i32 - 1) / 6.0;
    v.round().max(1.0) as u32
}

// ------------------------------------------------------------------
// Récolte
// ------------------------------------------------------------------

/// Un nœud de récolte posé dans une zone.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub pos: Vec2,
    pub kind: NodeKind,
    pub tier: u8,
    /// Unités encore à prendre.
    pub left: u32,
}

/// Niveau de Récolte requis pour un tier (même échelle que l'équipement).
pub fn gather_level_for(tier: u8) -> u32 {
    level_for_tier(tier)
}

/// Secondes pour arracher une unité.
pub fn gather_time(tier: u8, level: u32) -> f32 {
    1.3 * (1.0 + 0.2 * (tier.clamp(1, 8) - 1) as f32) / (1.0 + level.min(100) as f32 * 0.02)
}

/// Gloire de Récolte par unité.
pub fn gather_fame(tier: u8) -> f64 {
    5.0 * tier_mult(tier) as f64
}

/// Chance d'obtenir une unité de plus (rendement).
pub fn bonus_yield_chance(level: u32) -> f32 {
    level.min(100) as f32 * 0.01
}

/// Une bête vaincue laisse une carcasse à dépecer.
pub fn carcass_of(e: &EnemyDef, pos: Vec2) -> Option<Node> {
    if e.family != "bete" {
        return None;
    }
    let left = if e.boss { 6 } else if e.scale > 1.2 { 3 } else { 2 };
    Some(Node { pos, kind: NodeKind::Carcass, tier: e.tier.clamp(1, 8), left })
}

/// Tier d'un nœud tiré pour une zone : surtout le tier de la zone,
/// parfois un cran en dessous, rarement un au-dessus.
pub fn roll_node_tier(zone_tier: u8, rng: &mut Rng) -> u8 {
    let x = rng.f32();
    let t = if x < 0.25 {
        zone_tier.saturating_sub(1)
    } else if x > 0.93 {
        zone_tier + 1
    } else {
        zone_tier
    };
    t.clamp(1, 8)
}

/// Où en est la récolte en cours.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Gathering {
    pub node: Option<usize>,
    pub progress: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GatherTick {
    /// Aucun nœud à portée.
    Idle,
    /// Jauge en cours sur ce nœud (0..1).
    Working { node: usize, frac: f32 },
    /// Une ou plusieurs unités récoltées.
    Got { node: usize, kind: NodeKind, tier: u8, count: u32 },
    /// Nœud trop dur pour votre niveau.
    TooHard { node: usize, need: u32 },
}

/// Le nœud récoltable le plus proche, s'il est à portée.
pub fn nearest_node(nodes: &[Node], at: Vec2) -> Option<usize> {
    nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.left > 0 && n.pos.distance(at) <= GATHER_RANGE)
        .min_by(|a, b| a.1.pos.distance(at).total_cmp(&b.1.pos.distance(at)))
        .map(|(i, _)| i)
}

impl Gathering {
    /// Fait avancer la récolte. `level` = niveau de Récolte.
    pub fn tick(&mut self, nodes: &mut [Node], hero: Vec2, dt: f32, level: u32, rng: &mut Rng) -> GatherTick {
        let Some(i) = nearest_node(nodes, hero) else {
            *self = Gathering::default();
            return GatherTick::Idle;
        };
        if self.node != Some(i) {
            self.node = Some(i);
            self.progress = 0.0;
        }
        let n = &mut nodes[i];
        let need = gather_level_for(n.tier);
        if level < need {
            self.progress = 0.0;
            return GatherTick::TooHard { node: i, need };
        }
        self.progress += dt / gather_time(n.tier, level);
        if self.progress < 1.0 {
            return GatherTick::Working { node: i, frac: self.progress };
        }
        self.progress = 0.0;
        let mut count = 1;
        if rng.chance(bonus_yield_chance(level)) {
            count += 1;
        }
        n.left -= 1;
        GatherTick::Got { node: i, kind: n.kind, tier: n.tier, count }
    }

    /// Un coup reçu fait lâcher l'outil.
    pub fn interrupt(&mut self) {
        self.progress = 0.0;
    }
}

/// Ajoute des ressources à une liste de piles (empile).
pub fn stack_add(stacks: &mut Vec<ResourceStack>, id: &str, tier: u8, count: u32) {
    if count == 0 {
        return;
    }
    if let Some(s) = stacks.iter_mut().find(|s| s.id == id && s.tier == tier) {
        s.count += count;
    } else {
        stacks.push(ResourceStack { id: id.to_string(), tier, count });
        stacks.sort_by(|a, b| (a.id.as_str(), a.tier).cmp(&(b.id.as_str(), b.tier)));
    }
}

pub fn stack_count(stacks: &[ResourceStack], id: &str, tier: u8) -> u32 {
    stacks.iter().find(|s| s.id == id && s.tier == tier).map(|s| s.count).unwrap_or(0)
}

/// Retire des ressources. Faux (et rien retiré) s'il en manque.
pub fn stack_take(stacks: &mut Vec<ResourceStack>, id: &str, tier: u8, count: u32) -> bool {
    let Some(k) = stacks.iter().position(|s| s.id == id && s.tier == tier) else { return count == 0 };
    if stacks[k].count < count {
        return false;
    }
    stacks[k].count -= count;
    if stacks[k].count == 0 {
        stacks.remove(k);
    }
    true
}

// ------------------------------------------------------------------
// Atelier
// ------------------------------------------------------------------

/// Une recette : ce qu'il faut pour forger `base` au tier `tier`.
#[derive(Debug, Clone, PartialEq)]
pub struct Recipe {
    pub base: String,
    pub tier: u8,
    /// (ressource, quantité), toutes du même tier que l'objet.
    pub needs: Vec<(String, u32)>,
    pub gold: u32,
    /// Niveau de Forge requis.
    pub forge_level: u32,
}

/// La ressource principale d'un matériau.
pub fn resource_for(c: &Content, m: Material) -> Option<&crate::craft::ResourceDef> {
    c.resources.iter().find(|r| r.material == m)
}

/// Quantité de ressource principale selon l'emplacement.
fn main_count(b: &ItemBase) -> Option<u32> {
    Some(match b.slot {
        EquipSlot::MainHand if b.two_handed => 12,
        EquipSlot::MainHand => 8,
        EquipSlot::Chest => 8,
        EquipSlot::OffHand => 6,
        EquipSlot::Head | EquipSlot::Feet | EquipSlot::Cape => 4,
        _ => return None,
    })
}

/// Les bases qu'on peut forger à l'atelier.
pub fn craftable(b: &ItemBase) -> bool {
    main_count(b).is_some() && matches!(b.material, Material::Metal | Material::Wood | Material::Leather | Material::Cloth)
}

pub fn recipe(c: &Content, base: &str, tier: u8) -> Option<Recipe> {
    let b = c.base(base)?;
    if !craftable(b) || tier < b.min_tier || tier > b.max_tier {
        return None;
    }
    let main = resource_for(c, b.material)?;
    let n = main_count(b)?;
    let mut needs = vec![(main.id.clone(), n)];
    if !main.secondary.is_empty() {
        needs.push((main.secondary.clone(), n.div_ceil(4)));
    }
    let probe = Item::new(0, base, tier);
    let gold = (item_value(&probe) / 3).max(1);
    Some(Recipe { base: base.to_string(), tier, needs, gold, forge_level: level_for_tier(tier) })
}

/// Toutes les recettes d'un tier, dans l'ordre du contenu.
pub fn recipes_at(c: &Content, tier: u8) -> Vec<Recipe> {
    c.bases.iter().filter_map(|b| recipe(c, &b.id, tier)).collect()
}

/// Ce qui manque pour forger (vide = c'est bon).
pub fn missing(c: &Content, p: &Profile, r: &Recipe) -> Vec<String> {
    let mut out = vec![];
    let lvl = p.hero.masteries.level(LINE_FORGE);
    if lvl < r.forge_level {
        out.push(format!("Forge niveau {}", r.forge_level));
    }
    for (id, n) in &r.needs {
        let have = stack_count(&p.resources, id, r.tier);
        if have < *n {
            let name = c.resource(id).map(|d| d.tier_name(r.tier).to_string()).unwrap_or(id.clone());
            out.push(format!("{} {name}", n - have));
        }
    }
    if p.gold < r.gold {
        out.push(format!("{} po", r.gold - p.gold));
    }
    out
}

/// Chance de qualité : la maîtrise de la Forge fait pencher la balance.
pub fn forge_luck(forge_level: u32) -> f32 {
    forge_level.min(100) as f32 * 0.04
}

/// Gloire de Forge pour une recette.
pub fn forge_fame(r: &Recipe) -> f64 {
    let units: u32 = r.needs.iter().map(|(_, n)| n).sum();
    4.0 * units as f64 * tier_mult(r.tier) as f64
}

/// Forge l'objet : retire ressources et or, tire la qualité, met l'objet
/// au sac. Renvoie l'objet (copie) et une éventuelle montée de niveau.
pub fn craft(c: &Content, p: &mut Profile, base: &str, tier: u8) -> Result<(Item, Option<LevelUp>), String> {
    let r = recipe(c, base, tier).ok_or("Recette inconnue.")?;
    let miss = missing(c, p, &r);
    if !miss.is_empty() {
        return Err(format!("Il manque : {}.", miss.join(", ")));
    }
    if p.bag.len() >= crate::profile::BAG_SIZE {
        return Err("Sac plein.".into());
    }
    for (id, n) in &r.needs {
        stack_take(&mut p.resources, id, tier, *n);
    }
    p.gold -= r.gold;
    let lvl = p.hero.masteries.level(LINE_FORGE);
    let mut rng = p.rng.fork();
    let mut item = p.new_item(base, tier);
    item.quality = roll_quality(&mut rng, forge_luck(lvl));
    let slot = c.base(base).map(|b| b.slot).unwrap_or(EquipSlot::MainHand);
    roll_affixes(c, &mut rng, &mut item, slot);
    let up = p.hero.masteries.add_fame(LINE_FORGE, forge_fame(&r));
    p.counters.crafted += 1;
    let copy = item.clone();
    p.add_to_bag(c, item);
    Ok((copy, up))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::content;
    use crate::item::Quality;
    use crate::mastery::total_fame_for;

    #[test]
    fn piles_de_ressources() {
        let mut s = vec![];
        stack_add(&mut s, "bois", 1, 3);
        stack_add(&mut s, "bois", 1, 2);
        stack_add(&mut s, "bois", 2, 1);
        assert_eq!(s.len(), 2);
        assert_eq!(stack_count(&s, "bois", 1), 5);
        assert!(!stack_take(&mut s, "bois", 2, 2));
        assert_eq!(stack_count(&s, "bois", 2), 1);
        assert!(stack_take(&mut s, "bois", 2, 1));
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn recolter_un_arbre() {
        let mut nodes = vec![Node { pos: Vec2::new(0.0, 0.0), kind: NodeKind::Tree, tier: 1, left: 2 }];
        let mut g = Gathering::default();
        let mut rng = Rng::new(1);
        // Trop loin : rien.
        assert_eq!(g.tick(&mut nodes, Vec2::new(5.0, 0.0), 1.0, 0, &mut rng), GatherTick::Idle);
        // À côté : la jauge monte, puis une unité tombe.
        let mut got = 0;
        for _ in 0..100 {
            if let GatherTick::Got { count, .. } = g.tick(&mut nodes, Vec2::new(1.0, 0.0), 0.1, 0, &mut rng) {
                got += count;
            }
        }
        assert_eq!(got, 2, "niveau 0 : pas de rendement bonus");
        assert_eq!(nodes[0].left, 0);
        // Épuisé : plus rien.
        assert_eq!(g.tick(&mut nodes, Vec2::new(1.0, 0.0), 1.0, 0, &mut rng), GatherTick::Idle);
    }

    #[test]
    fn noeud_trop_dur() {
        let mut nodes = vec![Node { pos: Vec2::new(0.0, 0.0), kind: NodeKind::Vein, tier: 3, left: 4 }];
        let mut g = Gathering::default();
        let mut rng = Rng::new(1);
        let t = g.tick(&mut nodes, Vec2::new(1.0, 0.0), 5.0, 0, &mut rng);
        assert_eq!(t, GatherTick::TooHard { node: 0, need: level_for_tier(3) });
        assert_eq!(nodes[0].left, 4);
    }

    #[test]
    fn la_maitrise_accelere() {
        assert!(gather_time(1, 50) < gather_time(1, 0) * 0.6);
        assert!(gather_time(4, 0) > gather_time(1, 0));
    }

    #[test]
    fn carcasses_de_betes_seulement() {
        let c = content();
        let loup = c.enemy("loup_affame").unwrap();
        assert!(carcass_of(loup, Vec2::default()).is_some());
        let brigand = c.enemy("va_nu_pieds").unwrap();
        assert!(carcass_of(brigand, Vec2::default()).is_none());
    }

    #[test]
    fn recettes_coherentes() {
        let c = content();
        let epee = recipe(c, "epee_courte", 1).expect("épée forgeable");
        assert_eq!(epee.needs[0].0, "minerai");
        assert_eq!(epee.needs[1].0, "bois");
        assert!(recipe(c, "potion_soin", 1).is_none());
        assert!(recipe(c, "anneau", 1).is_none());
        // Chaque tier a de quoi faire.
        for t in 1..=8 {
            assert!(recipes_at(c, t).len() > 10, "tier {t}");
        }
        // Chaque ressource sert à quelque chose.
        for r in &c.resources {
            assert!(c.bases.iter().any(|b| craftable(b) && b.material == r.material), "{} inutile", r.id);
        }
    }

    #[test]
    fn forger_une_epee() {
        let c = content();
        let mut p = Profile::new_game(c, "Adeh", 5);
        assert!(craft(c, &mut p, "epee_courte", 1).is_err(), "sans rien, on ne forge rien");
        stack_add(&mut p.resources, "minerai", 1, 10);
        stack_add(&mut p.resources, "bois", 1, 2);
        p.gold = 50;
        let (it, _) = craft(c, &mut p, "epee_courte", 1).expect("forge");
        assert_eq!(it.base, "epee_courte");
        assert_eq!(p.bag.len(), 1);
        assert_eq!(stack_count(&p.resources, "minerai", 1), 2);
        assert_eq!(stack_count(&p.resources, "bois", 1), 0);
        assert!(p.gold < 50);
        assert!(p.hero.masteries.progress(LINE_FORGE).lifetime > 0.0);
        // Tier 3 : il faut la maîtrise.
        stack_add(&mut p.resources, "minerai", 3, 10);
        stack_add(&mut p.resources, "bois", 3, 2);
        p.gold = 5000;
        let e = craft(c, &mut p, "epee_courte", 3).unwrap_err();
        assert!(e.contains("Forge niveau"), "{e}");
    }

    #[test]
    fn maitre_forgeron_fait_mieux() {
        let c = content();
        let avg = |lvl: u32| {
            let mut p = Profile::new_game(c, "Adeh", 7);
            p.gold = 1_000_000;
            let mut sum = 0usize;
            for _ in 0..300 {
                // Niveau figé : on mesure la chance, pas la progression.
                p.hero.masteries = Default::default();
                p.hero.masteries.add_fame(LINE_FORGE, total_fame_for(lvl) + 1.0);
                stack_add(&mut p.resources, "minerai", 1, 8);
                stack_add(&mut p.resources, "bois", 1, 2);
                let (it, _) = craft(c, &mut p, "epee_courte", 1).unwrap();
                p.bag.clear();
                sum += Quality::ALL.iter().position(|q| *q == it.quality).unwrap();
            }
            sum as f32 / 300.0
        };
        let (hi, lo) = (avg(60), avg(0));
        assert!(hi > lo + 0.4, "{hi} vs {lo}");
    }
}
