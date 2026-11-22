//! Fiche de personnage : équipement, skills choisis, talents, maîtrises.
//!
//! Sert au héros comme aux compagnons de longue date. C'est ici que le
//! « tu es ce que tu portes » devient des chiffres.

use crate::combat::{Controller, FighterSpec, Team};
use crate::content::{Content, EnemyDef};
use crate::item::{EquipSlot, Item, scale_mod, affix_value_at_tier};
use crate::look::{Look, LookPart, Rgb};
use crate::mastery::{LineCategory, Masteries, item_power_bonus, level_for_tier};
use crate::passive::{PassiveDef, PassiveEffect, talent_slots};
use crate::rules::{Rule, default_rules};
use crate::skill::{BAR_SIZE, SkillDef, SkillSlot, Tag};
use crate::stats::{Modifier, Stat, StatBlock};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Character {
    pub name: String,
    pub skin: Rgb,
    pub hair: Rgb,
    pub hair_style: u8,
    pub equipment: BTreeMap<EquipSlot, Item>,
    /// Skill choisi pour chaque emplacement actif.
    pub skills: BTreeMap<SkillSlot, String>,
    /// Talents équipés (ids).
    pub talents: Vec<String>,
    pub masteries: Masteries,
    pub rules: Vec<Rule>,
}

/// Pourquoi on ne peut pas équiper un objet.
#[derive(Debug, Clone, PartialEq)]
pub enum EquipError {
    UnknownBase,
    WrongSlot,
    NeedsMastery { line: String, level: u32 },
}

impl Character {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            skin: (0.85, 0.66, 0.5),
            hair: (0.3, 0.2, 0.12),
            hair_style: 0,
            equipment: BTreeMap::new(),
            skills: BTreeMap::new(),
            talents: vec![],
            masteries: Masteries::default(),
            rules: default_rules(),
        }
    }

    pub fn item(&self, slot: EquipSlot) -> Option<&Item> {
        self.equipment.get(&slot)
    }

    /// Voie de l'objet porté dans cet emplacement.
    pub fn line_in(&self, c: &Content, slot: EquipSlot) -> Option<String> {
        let it = self.item(slot)?;
        let b = c.base(&it.base)?;
        (!b.line.is_empty()).then(|| b.line.clone())
    }

    /// Voie de l'arme portée (ou mains nues).
    pub fn weapon_line(&self, c: &Content) -> String {
        self.line_in(c, EquipSlot::MainHand).unwrap_or_else(|| "poings".to_string())
    }

    /// Toutes les voies portées (pour les talents « si équipé » et la gloire).
    pub fn equipped_lines(&self, c: &Content) -> Vec<String> {
        let mut v = vec![self.weapon_line(c)];
        for s in [EquipSlot::OffHand, EquipSlot::Head, EquipSlot::Chest, EquipSlot::Feet] {
            if let Some(l) = self.line_in(c, s)
                && !v.contains(&l) {
                    v.push(l);
                }
        }
        v
    }

    pub fn can_equip(&self, c: &Content, item: &Item) -> Result<EquipSlot, EquipError> {
        let b = c.base(&item.base).ok_or(EquipError::UnknownBase)?;
        if !b.line.is_empty() {
            let need = level_for_tier(item.tier);
            if self.masteries.level(&b.line) < need {
                return Err(EquipError::NeedsMastery { line: b.line.clone(), level: need });
            }
        }
        Ok(b.slot)
    }

    /// Équipe un objet ; renvoie ce qui a été retiré (pour l'inventaire).
    pub fn equip(&mut self, c: &Content, item: Item) -> Result<Vec<Item>, (EquipError, Item)> {
        let slot = match self.can_equip(c, &item) {
            Ok(s) => s,
            Err(e) => return Err((e, item)),
        };
        let mut removed = vec![];
        let two_handed = c.base(&item.base).map(|b| b.two_handed).unwrap_or(false);
        if slot == EquipSlot::MainHand && two_handed
            && let Some(off) = self.equipment.remove(&EquipSlot::OffHand) {
                removed.push(off);
            }
        if slot == EquipSlot::OffHand {
            let main_2h = self
                .item(EquipSlot::MainHand)
                .and_then(|m| c.base(&m.base))
                .map(|b| b.two_handed)
                .unwrap_or(false);
            if main_2h
                && let Some(m) = self.equipment.remove(&EquipSlot::MainHand) {
                    removed.push(m);
                }
        }
        if let Some(old) = self.equipment.insert(slot, item) {
            removed.push(old);
        }
        self.fix_loadout(c);
        Ok(removed)
    }

    pub fn unequip(&mut self, c: &Content, slot: EquipSlot) -> Option<Item> {
        let it = self.equipment.remove(&slot);
        self.fix_loadout(c);
        it
    }

    /// Skills disponibles pour un emplacement (selon l'équipement et la maîtrise).
    pub fn available_skills<'a>(&self, c: &'a Content, slot: SkillSlot) -> Vec<&'a SkillDef> {
        let line = match slot {
            SkillSlot::Basic | SkillSlot::Weapon1 | SkillSlot::Weapon2 | SkillSlot::Weapon3 => Some(self.weapon_line(c)),
            SkillSlot::Head => self.line_in(c, EquipSlot::Head),
            SkillSlot::Chest => self.line_in(c, EquipSlot::Chest),
            SkillSlot::Feet => self.line_in(c, EquipSlot::Feet),
            SkillSlot::Potion => None,
        };
        let Some(line) = line else { return vec![] };
        let lvl = self.masteries.level(&line);
        c.skills_for(&line, slot).into_iter().filter(|s| s.mastery <= lvl).collect()
    }

    /// Tous les skills d'un emplacement, y compris ceux encore verrouillés.
    pub fn all_skills_for<'a>(&self, c: &'a Content, slot: SkillSlot) -> Vec<(&'a SkillDef, bool)> {
        let line = match slot {
            SkillSlot::Basic | SkillSlot::Weapon1 | SkillSlot::Weapon2 | SkillSlot::Weapon3 => Some(self.weapon_line(c)),
            SkillSlot::Head => self.line_in(c, EquipSlot::Head),
            SkillSlot::Chest => self.line_in(c, EquipSlot::Chest),
            SkillSlot::Feet => self.line_in(c, EquipSlot::Feet),
            SkillSlot::Potion => None,
        };
        let Some(line) = line else { return vec![] };
        let lvl = self.masteries.level(&line);
        c.skills_for(&line, slot).into_iter().map(|s| (s, s.mastery <= lvl)).collect()
    }

    /// Après un changement d'équipement : garde les choix valides, complète le reste.
    pub fn fix_loadout(&mut self, c: &Content) {
        for slot in SkillSlot::ACTIVE {
            let avail = self.available_skills(c, slot);
            let keep = self.skills.get(&slot).map(|id| avail.iter().any(|s| &s.id == id)).unwrap_or(false);
            if !keep {
                match avail.first() {
                    Some(s) => {
                        self.skills.insert(slot, s.id.clone());
                    }
                    None => {
                        self.skills.remove(&slot);
                    }
                }
            }
        }
        // Talents : retire ceux qui ne sont plus débloqués, respecte le nombre d'emplacements.
        let unlocked: Vec<String> = self.unlocked_talents(c).iter().map(|p| p.id.clone()).collect();
        self.talents.retain(|t| unlocked.contains(t));
        let max = self.talent_slots();
        self.talents.truncate(max);
    }

    pub fn choose_skill(&mut self, c: &Content, slot: SkillSlot, id: &str) -> bool {
        if self.available_skills(c, slot).iter().any(|s| s.id == id) {
            self.skills.insert(slot, id.to_string());
            true
        } else {
            false
        }
    }

    pub fn talent_slots(&self) -> usize {
        talent_slots(self.masteries.total())
    }

    pub fn unlocked_talents<'a>(&self, c: &'a Content) -> Vec<&'a PassiveDef> {
        c.passives.iter().filter(|p| self.masteries.level(&p.line) >= p.mastery).collect()
    }

    /// Active ou retire un talent. Renvoie faux si impossible (plein / verrouillé).
    pub fn toggle_talent(&mut self, c: &Content, id: &str) -> bool {
        if let Some(i) = self.talents.iter().position(|t| t == id) {
            self.talents.remove(i);
            return true;
        }
        if self.talents.len() >= self.talent_slots() {
            return false;
        }
        if !self.unlocked_talents(c).iter().any(|p| p.id == id) {
            return false;
        }
        self.talents.push(id.to_string());
        true
    }

    /// Stats d'un objet porté, avec tier, qualité, affixes et maîtrise.
    pub fn item_mods(&self, c: &Content, item: &Item) -> Vec<Modifier> {
        let Some(b) = c.base(&item.base) else { return vec![] };
        let bonus = if b.line.is_empty() { 0.0 } else { item_power_bonus(self.masteries.level(&b.line)) };
        item_mods_with_bonus(c, item, bonus)
    }

    /// Effets passifs actifs (talents équipés + uniques portés).
    pub fn active_passives(&self, c: &Content) -> Vec<PassiveEffect> {
        let lines = self.equipped_lines(c);
        let mut out = vec![];
        for id in &self.talents {
            if let Some(p) = c.passive(id) {
                if p.needs_equipped && !lines.contains(&p.line) {
                    continue;
                }
                out.extend(p.effects.iter().cloned());
            }
        }
        for it in self.equipment.values() {
            if let Some(u) = it.unique.as_ref().and_then(|u| c.unique(u)) {
                out.extend(u.effects.iter().cloned());
            }
        }
        out
    }

    /// Stats finales hors combat (affichées dans la fiche).
    pub fn stats(&self, c: &Content) -> StatBlock {
        let mut s = StatBlock::human_base();
        for it in self.equipment.values() {
            s.add_all(&self.item_mods(c, it));
        }
        for p in self.active_passives(c) {
            if let PassiveEffect::Stat(m) = p {
                s.add(m);
            }
        }
        s
    }

    /// Barre de skills pour le combat.
    pub fn skill_bar(&self, c: &Content) -> [Option<SkillDef>; BAR_SIZE] {
        let mut bar: [Option<SkillDef>; BAR_SIZE] = Default::default();
        let basic_id = self
            .item(EquipSlot::MainHand)
            .and_then(|i| c.base(&i.base))
            .map(|b| b.basic.clone())
            .unwrap_or_else(|| "poing".to_string());
        bar[SkillSlot::Basic.bar_index()] = c.skill(&basic_id).cloned();
        for (slot, id) in &self.skills {
            if let Some(s) = c.skill(id) {
                bar[slot.bar_index()] = Some(s.clone());
            }
        }
        if let Some(p) = self.item(EquipSlot::Potion)
            && let Some(b) = c.base(&p.base)
                && let Some(s) = c.skill(&b.use_skill) {
                    let mut s = s.clone();
                    // Les potions de tier élevé soignent plus.
                    for e in s.effects.iter_mut() {
                        if let crate::skill::Effect::SelfHeal { base, .. } = e {
                            *base *= crate::item::tier_mult(p.tier);
                        }
                        if let crate::skill::Effect::RestoreEnergy { amount } = e {
                            *amount *= 1.0 + (crate::item::tier_mult(p.tier) - 1.0) * 0.3;
                        }
                    }
                    bar[SkillSlot::Potion.bar_index()] = Some(s);
                }
        bar
    }

    pub fn look(&self, c: &Content) -> Look {
        let mut look = Look::humanoid(self.skin, self.hair, self.hair_style);
        let places = [
            (EquipSlot::MainHand, "main"),
            (EquipSlot::OffHand, "off"),
            (EquipSlot::Head, "head"),
            (EquipSlot::Chest, "chest"),
            (EquipSlot::Feet, "feet"),
            (EquipSlot::Cape, "cape"),
        ];
        for (slot, at) in places {
            if let Some(it) = self.item(slot)
                && let Some(b) = c.base(&it.base) {
                    look.parts.push(LookPart {
                        at: at.to_string(),
                        model: b.model.clone(),
                        color: b.material.tier_color(it.tier),
                        tier: it.tier,
                    });
                }
        }
        look
    }

    pub fn fighter_spec(&self, c: &Content, team: Team, controller: Controller) -> FighterSpec {
        FighterSpec {
            name: self.name.clone(),
            team,
            stats: self.stats(c),
            skills: self.skill_bar(c),
            passives: self.active_passives(c),
            controller,
            look: self.look(c),
            radius: 0.4,
            fame: 20.0,
            key: self.name.clone(),
            tier: self.power_tier(),
        }
    }

    /// Tier « moyen » de l'équipement (pour l'affichage et la difficulté).
    pub fn power_tier(&self) -> u8 {
        let tiers: Vec<u8> = [EquipSlot::MainHand, EquipSlot::Head, EquipSlot::Chest, EquipSlot::Feet]
            .iter()
            .map(|s| self.item(*s).map(|i| i.tier).unwrap_or(0))
            .collect();
        let sum: u32 = tiers.iter().map(|t| *t as u32).sum();
        ((sum as f32 / 4.0).round() as u8).max(1)
    }

    /// Répartit la gloire d'un combat entre les voies utilisées.
    /// Arme 50 %, armures et main secondaire se partagent le reste.
    pub fn award_fame(&mut self, c: &Content, fame: f32) -> Vec<crate::mastery::LevelUp> {
        let mut ups = vec![];
        if fame <= 0.0 {
            return ups;
        }
        let weapon = self.weapon_line(c);
        let others: Vec<String> = [EquipSlot::OffHand, EquipSlot::Head, EquipSlot::Chest, EquipSlot::Feet]
            .iter()
            .filter_map(|s| self.line_in(c, *s))
            .collect();
        let weapon_share = if others.is_empty() { 1.0 } else { 0.5 };
        if let Some(u) = self.masteries.add_fame(&weapon, (fame * weapon_share) as f64) {
            ups.push(u);
        }
        if !others.is_empty() {
            let each = fame * 0.5 / others.len() as f32;
            for l in others {
                if let Some(u) = self.masteries.add_fame(&l, each as f64) {
                    ups.push(u);
                }
            }
        }
        self.fix_loadout(c);
        ups
    }

    /// Le badge « rôle détecté », calculé d'après le build.
    pub fn role(&self, c: &Content) -> RoleBadge {
        detect_role(c, &self.stats(c), &self.skill_bar(c), &self.weapon_line(c))
    }
}

/// Stats d'un objet avec un bonus de maîtrise donné.
pub fn item_mods_with_bonus(c: &Content, item: &Item, bonus: f32) -> Vec<Modifier> {
    let Some(b) = c.base(&item.base) else { return vec![] };
    let mut out: Vec<Modifier> = b.mods.iter().map(|m| scale_mod(m, item.tier, item.quality, bonus)).collect();
    for a in &item.affixes {
        if let Some(def) = c.affix(&a.affix) {
            let v = affix_value_at_tier(def, a.value, item.tier) * (1.0 + bonus * 0.5);
            out.push(Modifier { stat: def.stat, kind: def.kind, value: v });
        }
    }
    if let Some(u) = item.unique.as_ref().and_then(|u| c.unique(u)) {
        out.extend(u.mods.iter().copied());
    }
    out
}

/// Nom complet d'un objet : « Épée longue d'acier du Faucon ».
pub fn item_name(c: &Content, item: &Item) -> String {
    if let Some(u) = item.unique.as_ref().and_then(|u| c.unique(u)) {
        return u.name.clone();
    }
    let Some(b) = c.base(&item.base) else { return "???".into() };
    let mut n = format!("{} {}", b.name, b.material.tier_name(item.tier));
    if let Some(a) = item.affixes.first().and_then(|a| c.affix(&a.affix)) {
        n.push(' ');
        n.push_str(&a.name);
    }
    n
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoleBadge {
    pub title: String,
    pub tank: f32,
    pub healer: f32,
    pub damage: f32,
    pub control: f32,
    pub support: f32,
}

/// Lit les skills et les stats pour deviner le rôle. Fun, pas contraignant.
pub fn detect_role(_c: &Content, stats: &StatBlock, bar: &[Option<SkillDef>; BAR_SIZE], weapon_line: &str) -> RoleBadge {
    let mut tank = 0.0f32;
    let mut healer = 0.0f32;
    let mut damage = 0.0f32;
    let mut control = 0.0f32;
    let mut support = 0.0f32;
    let mut magic = 0.0f32;
    for s in bar.iter().flatten() {
        if s.slot == SkillSlot::Basic || s.slot == SkillSlot::Potion {
            continue;
        }
        let tags = s.all_tags();
        if tags.contains(&Tag::Taunt) {
            tank += 2.0;
        }
        if tags.contains(&Tag::Defense) || tags.contains(&Tag::Shield) {
            tank += 0.8;
            support += 0.3;
        }
        // Ne compte comme soigneur que ce qui soigne les autres.
        if tags.contains(&Tag::Heal) && s.targeting.is_friendly() && !matches!(s.targeting, crate::skill::Targeting::SelfOnly) {
            healer += 1.6;
        }
        if tags.contains(&Tag::Control) {
            control += 1.2;
        }
        if tags.contains(&Tag::Buff) || tags.contains(&Tag::Debuff) {
            support += 0.7;
        }
        if s.effects.iter().any(|e| matches!(e, crate::skill::Effect::Damage { .. } | crate::skill::Effect::Detonate { .. })) {
            damage += 1.0;
        }
        if tags.contains(&Tag::Spell) {
            magic += 1.0;
        }
    }
    let armor = stats.get(Stat::Armor) + stats.get(Stat::MagicResist);
    tank += (armor / 40.0).min(3.0) + (stats.get(Stat::ThreatMult) - 1.0).max(0.0) * 3.0;
    healer += (stats.get(Stat::HealPower) / 15.0).min(2.0);
    damage += ((stats.get(Stat::PhysicalPower) + stats.get(Stat::MagicPower)) / 20.0).min(2.5);
    damage += stats.get(Stat::CritChance) * 6.0;
    let scores = [("tank", tank), ("soin", healer), ("degats", damage), ("controle", control), ("soutien", support)];
    let mut sorted = scores.to_vec();
    sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    let (first, f) = sorted[0];
    let (second, s2) = sorted[1];
    let magic_weapon = matches!(weapon_line, "feu" | "givre" | "arcane" | "sacre" | "nature" | "maudit");
    let mage = magic_weapon || magic >= 2.0;
    let close = s2 > f * 0.75;
    let title = match (first, mage, close.then_some(second)) {
        (_, _, _) if f < 1.5 || (weapon_line == "poings" && f < 4.0) => "Vagabond sans le sou".to_string(),
        ("tank", true, Some("soin")) => "Paladin des arcanes".into(),
        ("tank", true, _) => "Mage-Rempart".into(),
        ("tank", false, Some("degats")) => "Briseur de lignes".into(),
        ("tank", false, Some("soin")) => "Templier".into(),
        ("tank", false, _) => "Rempart".into(),
        ("soin", _, Some("tank")) => "Gardien sacré".into(),
        ("soin", _, Some("degats")) => "Guérisseur de bataille".into(),
        ("soin", _, _) => "Soigneur".into(),
        ("degats", true, Some("controle")) => "Sorcier tempétueux".into(),
        ("degats", true, Some("tank")) => "Mage de guerre".into(),
        ("degats", true, _) => "Arcaniste dévastateur".into(),
        ("degats", false, Some("controle")) => "Duelliste tactique".into(),
        ("degats", false, Some("tank")) => "Bretteur cuirassé".into(),
        ("degats", false, _) => "Lame".into(),
        ("controle", true, _) => "Entraveur des arcanes".into(),
        ("controle", false, _) => "Geôlier".into(),
        ("soutien", _, Some("soin")) => "Barde de guerre".into(),
        _ => "Touche-à-tout".into(),
    };
    RoleBadge { title, tank, healer, damage, control, support }
}

/// Fabrique un combattant ennemi depuis sa fiche.
pub fn enemy_spec(c: &Content, e: &EnemyDef) -> FighterSpec {
    let mut s = StatBlock::new();
    s.add_flat(Stat::MaxHp, e.hp);
    s.add_flat(Stat::Armor, e.armor);
    s.add_flat(Stat::MagicResist, e.resist);
    s.add_flat(Stat::PhysicalPower, e.power);
    s.add_flat(Stat::MagicPower, e.power);
    s.add_flat(Stat::HealPower, e.power);
    s.add_flat(Stat::MoveSpeed, e.speed);
    s.add_flat(Stat::MaxEnergy, 100.0);
    s.add_flat(Stat::EnergyRegen, 10.0);
    s.add_flat(Stat::CritChance, e.crit);
    s.add_flat(Stat::CritDamage, 0.5);
    s.add_flat(Stat::DodgeChance, e.dodge);
    s.add_flat(Stat::ThreatMult, 1.0);
    let mut bar: [Option<SkillDef>; BAR_SIZE] = Default::default();
    for (slot, id) in &e.skills {
        bar[slot.bar_index()] = c.skill(id).cloned();
    }
    let look = if e.model == "humain" {
        let mut l = Look::humanoid((0.8, 0.62, 0.48), (0.2, 0.15, 0.1), 1);
        l.color = e.color;
        l.scale = e.scale;
        for (at, model, color) in &e.parts {
            l.parts.push(LookPart { at: at.clone(), model: model.clone(), color: *color, tier: e.tier });
        }
        l
    } else {
        Look::creature(&e.model, e.color, e.scale)
    };
    let rules = if e.rules.is_empty() { default_rules() } else { e.rules.clone() };
    FighterSpec {
        name: e.name.clone(),
        team: Team::Enemy,
        stats: s,
        skills: bar,
        passives: vec![],
        controller: Controller::Monster { rules, aggro: e.aggro, leash: e.leash },
        look,
        radius: e.radius,
        fame: e.fame,
        key: e.id.clone(),
        tier: e.tier,
    }
}

/// Résumé lisible des voies pour l'Atlas.
pub fn atlas_lines(c: &Content) -> Vec<(LineCategory, Vec<&crate::mastery::LineDef>)> {
    [LineCategory::Weapon, LineCategory::OffHand, LineCategory::Armor, LineCategory::Job]
        .into_iter()
        .map(|cat| (cat, c.lines_of(cat)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::content;

    fn hero() -> Character {
        let c = content();
        let mut h = Character::new("Test");
        h.equip(c, Item::new(1, "haillons", 1)).unwrap();
        h
    }

    #[test]
    fn mains_nues_au_depart() {
        let c = content();
        let h = hero();
        assert_eq!(h.weapon_line(c), "poings");
        let bar = h.skill_bar(c);
        assert_eq!(bar[0].as_ref().unwrap().id, "poing");
        assert!(h.stats(c).get(Stat::MaxHp) > 60.0);
    }

    #[test]
    fn l_arme_donne_les_skills() {
        let c = content();
        let mut h = hero();
        h.equip(c, Item::new(2, "baton_givre", 1)).unwrap();
        assert_eq!(h.skills.get(&SkillSlot::Weapon1).unwrap(), "trait_de_glace");
        // Ultime pas encore débloqué au niveau 0.
        assert!(!h.skills.contains_key(&SkillSlot::Weapon3));
        h.masteries.add_fame("givre", 1e5);
        h.fix_loadout(c);
        assert!(h.skills.contains_key(&SkillSlot::Weapon3));
    }

    #[test]
    fn tier_verrouille_par_maitrise() {
        let c = content();
        let mut h = hero();
        let r = h.equip(c, Item::new(3, "epee_longue", 5));
        assert!(matches!(r, Err((EquipError::NeedsMastery { .. }, _))));
        h.masteries.add_fame("epee", 1e9);
        assert!(h.equip(c, Item::new(3, "epee_longue", 5)).is_ok());
    }

    #[test]
    fn deux_mains_retire_le_bouclier() {
        let c = content();
        let mut h = hero();
        h.equip(c, Item::new(4, "epee_courte", 1)).unwrap();
        h.equip(c, Item::new(5, "bouclier_rond", 1)).unwrap();
        let removed = h.equip(c, Item::new(6, "baton_feu", 1)).unwrap();
        assert_eq!(removed.len(), 2);
        assert!(h.item(EquipSlot::OffHand).is_none());
    }

    #[test]
    fn mage_tank_possible() {
        let c = content();
        let mut h = hero();
        for l in ["givre", "plaque"] {
            h.masteries.add_fame(l, 1e5);
        }
        h.masteries.add_fame("labeur", 1e5);
        for (uid, b) in [(10, "baton_givre"), (11, "casque"), (12, "cotte"), (13, "solerets")] {
            h.equip(c, Item::new(uid, b, 4)).unwrap();
        }
        h.choose_skill(c, SkillSlot::Head, "provocation");
        assert!(h.toggle_talent(c, "bastion_arcanique"));
        let badge = h.role(c);
        assert!(badge.tank > 2.0, "{badge:?}");
        assert!(badge.title.contains("Mage") || badge.title.contains("Paladin"), "{}", badge.title);
    }

    #[test]
    fn noms_d_objets() {
        let c = content();
        let mut it = Item::new(1, "epee_courte", 3);
        assert_eq!(item_name(c, &it), "Épée courte d'acier");
        it.affixes.push(crate::item::AffixRoll { affix: "faucon".into(), value: 0.02 });
        assert_eq!(item_name(c, &it), "Épée courte d'acier du Faucon");
    }
}
