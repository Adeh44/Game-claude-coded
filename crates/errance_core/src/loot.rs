//! Génération de butin : base, tier, qualité, affixes, parfois un unique.

use crate::content::{Content, EnemyDef};
use crate::item::{AffixRoll, EquipSlot, Item, Quality};
use crate::rng::Rng;

/// Tire une qualité. `luck` > 0 améliore les chances.
pub fn roll_quality(rng: &mut Rng, luck: f32) -> Quality {
    let w = [
        60.0 / (1.0 + luck),
        25.0,
        10.0 * (1.0 + luck * 0.5),
        4.0 * (1.0 + luck),
        1.0 * (1.0 + luck * 2.0),
    ];
    Quality::ALL[rng.weighted(&w).unwrap_or(0)]
}

/// Fabrique un objet aléatoire de ce tier (et de cet emplacement si précisé).
pub fn random_item(c: &Content, rng: &mut Rng, uid: u64, tier: u8, slot: Option<EquipSlot>, luck: f32) -> Option<Item> {
    let tier = tier.clamp(1, 8);
    let candidates: Vec<_> = c
        .bases
        .iter()
        .filter(|b| tier >= b.min_tier && tier <= b.max_tier)
        .filter(|b| slot.is_none_or(|s| b.slot == s))
        .collect();
    let weights: Vec<f32> = candidates.iter().map(|b| b.weight).collect();
    let base = candidates[rng.weighted(&weights)?];
    let mut item = Item::new(uid, &base.id, tier);
    if matches!(base.slot, EquipSlot::Potion | EquipSlot::Food) {
        item.count = rng.range_u32(1, 3);
        return Some(item);
    }
    item.quality = roll_quality(rng, luck);
    let n = item.quality.affix_count();
    let mut pool: Vec<_> = c.affixes.iter().filter(|a| a.allows(base.slot)).collect();
    for _ in 0..n {
        let w: Vec<f32> = pool.iter().map(|a| a.weight).collect();
        let Some(i) = rng.weighted(&w) else { break };
        let a = pool.remove(i);
        let v = rng.range_f32(a.min, a.max);
        item.affixes.push(AffixRoll { affix: a.id.clone(), value: v });
    }
    Some(item)
}

/// Butin d'un ennemi vaincu : pièces + éventuellement un objet.
pub fn enemy_drop(c: &Content, rng: &mut Rng, e: &EnemyDef, next_uid: &mut u64) -> (u32, Vec<Item>) {
    let gold = rng.range_u32(e.gold.0, e.gold.1);
    let mut items = vec![];
    if rng.chance(e.loot_chance) {
        // Parfois un tier au-dessus, parfois en dessous.
        let tier = match rng.f32() {
            x if x < 0.15 => e.tier.saturating_sub(1).max(1),
            x if x > 0.92 => (e.tier + 1).min(8),
            _ => e.tier,
        };
        let luck = if e.boss { 2.0 } else { 0.0 };
        *next_uid += 1;
        if let Some(it) = random_item(c, rng, *next_uid, tier, None, luck) {
            items.push(it);
        }
    }
    // Les boss ont une chance de lâcher un unique de leur tier.
    if e.boss && rng.chance(0.35) {
        let uniques: Vec<_> = c.uniques.iter().filter(|u| u.tier <= e.tier).collect();
        if let Some(u) = rng.pick(&uniques) {
            *next_uid += 1;
            let mut it = Item::new(*next_uid, &u.base, u.tier);
            it.quality = Quality::Excellent;
            it.unique = Some(u.id.clone());
            items.push(it);
        }
    }
    (gold, items)
}

/// Prix de vente / d'achat d'un objet.
pub fn item_value(item: &Item) -> u32 {
    let base = match item.tier {
        1 => 4.0,
        t => 4.0 * 3.2f32.powi(t as i32 - 1),
    };
    let q = match item.quality {
        Quality::Ordinary => 1.0,
        Quality::Good => 1.4,
        Quality::Remarkable => 2.0,
        Quality::Excellent => 3.2,
        Quality::Masterpiece => 6.0,
    };
    let u = if item.unique.is_some() { 5.0 } else { 1.0 };
    (base * q * u * item.count.max(1) as f32).round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::content;

    #[test]
    fn butin_coherent() {
        let c = content();
        let mut rng = Rng::new(9);
        for t in 1..=8u8 {
            for k in 0..50 {
                let it = random_item(c, &mut rng, k, t, None, 0.0).unwrap();
                let b = c.base(&it.base).unwrap();
                assert!(t >= b.min_tier && t <= b.max_tier);
                assert_eq!(it.affixes.len(), it.quality.affix_count().min(it.affixes.len()));
            }
        }
    }

    #[test]
    fn valeur_croissante() {
        let a = Item::new(1, "epee_courte", 1);
        let b = Item::new(2, "epee_courte", 4);
        assert!(item_value(&b) > item_value(&a) * 10);
    }
}
