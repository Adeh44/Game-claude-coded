//! La partie du joueur : héros, bourse, sac, compagnons, réputation.
//! C'est ce qui est sauvegardé sur le téléphone.

use crate::character::{Character, item_name};
use crate::content::Content;
use crate::item::{EquipSlot, Item};
use crate::loot::item_value;
use crate::rng::Rng;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SAVE_VERSION: u32 = 1;
pub const BAG_SIZE: usize = 40;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Counters {
    pub kills: u32,
    pub deaths: u32,
    pub chores: u32,
    pub expeditions: u32,
    pub gold_earned: u64,
    #[serde(default)]
    pub gathered: u32,
    #[serde(default)]
    pub crafted: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub version: u32,
    pub hero: Character,
    pub gold: u32,
    pub bag: Vec<Item>,
    pub next_uid: u64,
    pub day: u32,
    pub rng: Rng,
    #[serde(default)]
    pub counters: Counters,
    /// Étapes du tutoriel / de l'histoire déjà vues.
    #[serde(default)]
    pub flags: Vec<String>,
    /// Réputation par faction / compagnie.
    #[serde(default)]
    pub reputation: BTreeMap<String, i32>,
    #[serde(default)]
    pub world: crate::world::World,
    /// Votre propre compagnie, une fois fondée.
    #[serde(default)]
    pub own_company: Option<String>,
    /// Ressources récoltées (réserve de l'atelier, ne prend pas de place au sac).
    #[serde(default)]
    pub resources: Vec<crate::craft::ResourceStack>,
}

impl Profile {
    /// Nouvelle partie : en haillons, sans un sou.
    pub fn new_game(c: &Content, name: &str, seed: u64) -> Self {
        let mut hero = Character::new(name);
        let mut p = Profile {
            version: SAVE_VERSION,
            hero: hero.clone(),
            gold: 0,
            bag: vec![],
            next_uid: 1,
            day: 1,
            rng: Rng::new(seed),
            counters: Counters::default(),
            flags: vec![],
            reputation: BTreeMap::new(),
            world: crate::world::World::default(),
            own_company: None,
            resources: vec![],
        };
        let rags = p.new_item("haillons", 1);
        let _ = hero.equip(c, rags);
        hero.fix_loadout(c);
        p.hero = hero;
        let mut wrng = p.rng.fork();
        p.world = crate::world::World::generate(c, &mut wrng);
        p
    }

    pub fn new_uid(&mut self) -> u64 {
        self.next_uid += 1;
        self.next_uid
    }

    pub fn new_item(&mut self, base: &str, tier: u8) -> Item {
        let uid = self.new_uid();
        Item::new(uid, base, tier)
    }

    pub fn has_flag(&self, f: &str) -> bool {
        self.flags.iter().any(|x| x == f)
    }

    pub fn set_flag(&mut self, f: &str) {
        if !self.has_flag(f) {
            self.flags.push(f.to_string());
        }
    }

    /// Ajoute au sac (empile les consommables identiques). Faux si plein.
    pub fn add_to_bag(&mut self, c: &Content, item: Item) -> bool {
        let stackable = c
            .base(&item.base)
            .map(|b| matches!(b.slot, EquipSlot::Potion | EquipSlot::Food))
            .unwrap_or(false);
        if stackable {
            if let Some(it) = self.bag.iter_mut().find(|i| i.base == item.base && i.tier == item.tier) {
                it.count += item.count;
                return true;
            }
        }
        if self.bag.len() >= BAG_SIZE {
            return false;
        }
        self.bag.push(item);
        true
    }

    /// Équipe l'objet n° `idx` du sac. Renvoie un message d'erreur lisible sinon.
    pub fn equip_from_bag(&mut self, c: &Content, idx: usize) -> Result<(), String> {
        if idx >= self.bag.len() {
            return Err("Objet introuvable".into());
        }
        let item = self.bag.remove(idx);
        match self.hero.equip(c, item) {
            Ok(removed) => {
                for r in removed {
                    self.bag.push(r);
                }
                Ok(())
            }
            Err((e, item)) => {
                let msg = match &e {
                    crate::character::EquipError::NeedsMastery { line, level } => {
                        let name = c.line(line).map(|l| l.name.as_str()).unwrap_or(line);
                        format!("Il faut {name} niveau {level} pour porter {}", item_name(c, &item))
                    }
                    _ => "Impossible d'équiper cet objet".into(),
                };
                self.bag.insert(idx.min(self.bag.len()), item);
                Err(msg)
            }
        }
    }

    pub fn unequip_to_bag(&mut self, c: &Content, slot: EquipSlot) -> bool {
        if self.bag.len() >= BAG_SIZE {
            return false;
        }
        if let Some(it) = self.hero.unequip(c, slot) {
            self.bag.push(it);
            true
        } else {
            false
        }
    }

    /// Vend l'objet n° `idx` du sac (la moitié de sa valeur).
    pub fn sell(&mut self, idx: usize) -> Option<u32> {
        if idx >= self.bag.len() {
            return None;
        }
        let it = self.bag.remove(idx);
        let v = (item_value(&it) / 2).max(1);
        self.gold += v;
        self.counters.gold_earned += v as u64;
        Some(v)
    }

    pub fn earn(&mut self, gold: u32) {
        self.gold += gold;
        self.counters.gold_earned += gold as u64;
    }

    pub fn spend(&mut self, gold: u32) -> bool {
        if self.gold >= gold {
            self.gold -= gold;
            true
        } else {
            false
        }
    }

    /// Consomme une potion équipée (après usage en combat).
    pub fn consume_potion(&mut self) {
        if let Some(p) = self.hero.equipment.get_mut(&EquipSlot::Potion) {
            if p.count > 1 {
                p.count -= 1;
            } else {
                self.hero.equipment.remove(&EquipSlot::Potion);
            }
        }
    }

    /// Peut-on fonder sa compagnie ? Il faut avoir fait ses preuves.
    pub fn can_found_company(&self) -> Result<(), String> {
        if self.own_company.is_some() {
            return Err("Vous avez déjà votre compagnie.".into());
        }
        let best_rank = self
            .reputation
            .iter()
            .filter(|(k, _)| k.starts_with("co:"))
            .map(|(_, v)| crate::world::Rank::from_rep(*v))
            .max()
            .unwrap_or_default();
        let mastery = self.hero.masteries.total();
        if best_rank < crate::world::Rank::Veteran && mastery < 250 {
            return Err(format!(
                "Personne ne suivrait un inconnu. Il faut être Vétéran d'une compagnie, ou 250 de maîtrise totale (vous : {mastery})."
            ));
        }
        if self.gold < 200 {
            return Err("Fonder une compagnie coûte 200 po (charte, bannière, tonneau de bière).".into());
        }
        Ok(())
    }

    /// Vend toute une pile de ressources. Renvoie le gain.
    pub fn sell_resource(&mut self, idx: usize) -> Option<u32> {
        if idx >= self.resources.len() {
            return None;
        }
        let s = self.resources.remove(idx);
        let v = crate::craft::resource_unit_value(s.tier) * s.count;
        self.earn(v);
        Some(v)
    }

    pub fn to_ron(&self) -> String {
        ron::ser::to_string(self).unwrap_or_default()
    }

    pub fn from_ron(s: &str) -> Option<Profile> {
        ron::from_str(s).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::content;

    #[test]
    fn nouvelle_partie_pauvre() {
        let c = content();
        let p = Profile::new_game(c, "Adeh", 1);
        assert_eq!(p.gold, 0);
        assert!(p.bag.is_empty());
        assert!(p.hero.item(EquipSlot::Chest).is_some());
        assert!(p.hero.item(EquipSlot::MainHand).is_none());
    }

    #[test]
    fn sauvegarde_aller_retour() {
        let c = content();
        let mut p = Profile::new_game(c, "Adeh", 2);
        let it = p.new_item("gourdin_rouille", 1);
        p.add_to_bag(c, it);
        p.equip_from_bag(c, 0).unwrap();
        let s = p.to_ron();
        let q = Profile::from_ron(&s).expect("relecture");
        assert_eq!(p, q);
    }

    #[test]
    fn potions_empilees() {
        let c = content();
        let mut p = Profile::new_game(c, "Adeh", 3);
        let a = p.new_item("potion_soin", 1);
        let b = p.new_item("potion_soin", 1);
        p.add_to_bag(c, a);
        p.add_to_bag(c, b);
        assert_eq!(p.bag.len(), 1);
        assert_eq!(p.bag[0].count, 2);
    }
}
