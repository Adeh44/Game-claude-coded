//! Le monde qui vit : aventuriers PNJ, compagnies, zones.
//!
//! Les aventuriers ont une vraie fiche (équipement, maîtrises, règles) :
//! ce sont eux qu'on recrute comme mercenaires ou compagnons, et eux qui
//! vous recrutent quand ils mènent une compagnie.

use crate::character::Character;
use crate::content::Content;
use crate::item::{EquipSlot, Item, Quality};
use crate::look::Rgb;
use crate::rng::Rng;
use crate::rules::{Action, Condition, Rule};
use crate::skill::SkillSlot;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Trait {
    /// Recule vite quand ça tourne mal.
    Coward,
    /// Ne recule jamais, même quand il faudrait.
    Proud,
    /// Veut plus d'or, se plaint du partage.
    Greedy,
    /// Fidèle : l'affinité baisse moins.
    Loyal,
    /// Bavard : commente tout (bonne humeur du groupe).
    Chatty,
    /// Prudent : garde ses potions, se soigne tôt.
    Careful,
    /// Téméraire : fonce sur les plus gros.
    Reckless,
    /// Pieux : n'aime pas les bâtons maudits.
    Pious,
}

impl Trait {
    pub const ALL: [Trait; 8] = [
        Trait::Coward,
        Trait::Proud,
        Trait::Greedy,
        Trait::Loyal,
        Trait::Chatty,
        Trait::Careful,
        Trait::Reckless,
        Trait::Pious,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Trait::Coward => "Couard",
            Trait::Proud => "Fier",
            Trait::Greedy => "Avide",
            Trait::Loyal => "Loyal",
            Trait::Chatty => "Bavard",
            Trait::Careful => "Prudent",
            Trait::Reckless => "Téméraire",
            Trait::Pious => "Pieux",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Trait::Coward => "Recule quand ses PV baissent.",
            Trait::Proud => "N'accepte pas d'être payé moins que vous.",
            Trait::Greedy => "Coûte plus cher, se plaint du partage.",
            Trait::Loyal => "Son affinité baisse moins vite.",
            Trait::Chatty => "Commente tout ; le groupe garde le moral.",
            Trait::Careful => "Boit sa potion plus tôt.",
            Trait::Reckless => "Fonce, même sur ce qui le dépasse.",
            Trait::Pious => "Déteste la magie maudite.",
        }
    }
}

/// Un aventurier du monde.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Npc {
    pub id: u32,
    pub sheet: Character,
    pub traits: Vec<Trait>,
    /// -100 .. 100 : ce qu'il pense de vous.
    pub affinity: i32,
    /// Compagnie dont il est membre.
    pub company: Option<u32>,
    pub alive: bool,
    /// Phrase d'accroche à la taverne.
    pub line: String,
    /// Est-il votre compagnon de longue date ?
    pub companion: bool,
    /// Mercenaire sous contrat : expéditions restantes.
    pub hired_for: u32,
    pub gender_female: bool,
}

impl Npc {
    pub fn name(&self) -> &str {
        &self.sheet.name
    }

    /// Prix d'un contrat de mercenaire pour une expédition.
    pub fn hire_price(&self) -> u32 {
        let tier = self.sheet.power_tier() as u32;
        let lvl = self.sheet.masteries.total();
        let mut p = 6 + tier * tier * 8 + lvl / 3;
        if self.traits.contains(&Trait::Greedy) {
            p = p * 3 / 2;
        }
        p
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Company {
    pub id: u32,
    pub name: String,
    pub leader: u32,
    pub members: Vec<u32>,
    pub renown: i32,
    pub color: Rgb,
}

/// Votre rang dans une compagnie : de Porteur à Officier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub enum Rank {
    #[default]
    Porter,
    Recruit,
    Member,
    Veteran,
    Officer,
}

impl Rank {
    pub fn label(self) -> &'static str {
        match self {
            Rank::Porter => "Porteur",
            Rank::Recruit => "Recrue",
            Rank::Member => "Membre",
            Rank::Veteran => "Vétéran",
            Rank::Officer => "Officier",
        }
    }

    /// Part du butin selon le rang.
    pub fn share(self) -> f32 {
        match self {
            Rank::Porter => 0.03,
            Rank::Recruit => 0.08,
            Rank::Member => 0.15,
            Rank::Veteran => 0.22,
            Rank::Officer => 0.3,
        }
    }

    /// Réputation nécessaire pour ce rang.
    pub fn threshold(self) -> i32 {
        match self {
            Rank::Porter => 0,
            Rank::Recruit => 30,
            Rank::Member => 100,
            Rank::Veteran => 250,
            Rank::Officer => 500,
        }
    }

    pub fn from_rep(rep: i32) -> Rank {
        [Rank::Officer, Rank::Veteran, Rank::Member, Rank::Recruit, Rank::Porter]
            .into_iter()
            .find(|r| rep >= r.threshold())
            .unwrap_or(Rank::Porter)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct World {
    pub npcs: Vec<Npc>,
    pub companies: Vec<Company>,
    pub next_id: u32,
}

/// Archétypes de PNJ : arme + armure + règles.
struct Archetype {
    weapon: &'static str,
    off: Option<&'static str>,
    head: &'static str,
    chest: &'static str,
    feet: &'static str,
    rules: fn() -> Vec<Rule>,
}

fn rules_dps() -> Vec<Rule> {
    vec![
        Rule::new(Condition::SelfHpBelow(0.3), Action::Use(SkillSlot::Potion)),
        Rule::new(Condition::Always, Action::Use(SkillSlot::Weapon3)),
        Rule::new(Condition::Always, Action::Use(SkillSlot::Weapon1)),
        Rule::new(Condition::Always, Action::Use(SkillSlot::Weapon2)),
        Rule::new(Condition::Always, Action::Use(SkillSlot::Head)),
    ]
}

fn rules_tank() -> Vec<Rule> {
    vec![
        Rule::new(Condition::SelfHpBelow(0.3), Action::Use(SkillSlot::Potion)),
        Rule::new(Condition::AllyAttacked, Action::Use(SkillSlot::Head)),
        Rule::new(Condition::AllyAttacked, Action::Use(SkillSlot::Weapon2)),
        Rule::new(Condition::SelfHpBelow(0.6), Action::Use(SkillSlot::Chest)),
        Rule::new(Condition::Always, Action::Use(SkillSlot::Weapon1)),
        Rule::new(Condition::Always, Action::Use(SkillSlot::Weapon3)),
    ]
}

fn rules_healer() -> Vec<Rule> {
    vec![
        Rule::new(Condition::AllyHpBelow(0.45), Action::Use(SkillSlot::Weapon3)),
        Rule::new(Condition::AllyHpBelow(0.75), Action::Use(SkillSlot::Weapon1)),
        Rule::new(Condition::AllyHpBelow(0.85), Action::Use(SkillSlot::Weapon2)),
        Rule::new(Condition::AllyDebuffed, Action::Use(SkillSlot::Weapon2)),
        Rule::new(Condition::SelfHpBelow(0.35), Action::Use(SkillSlot::Potion)),
    ]
}

const ARCHETYPES: &[Archetype] = &[
    Archetype { weapon: "epee_courte", off: Some("bouclier_rond"), head: "casque", chest: "cotte", feet: "solerets", rules: rules_tank },
    Archetype { weapon: "masse", off: Some("bouclier_rond"), head: "casque", chest: "cotte", feet: "bottes", rules: rules_tank },
    Archetype { weapon: "marteau_de_guerre", off: None, head: "casque", chest: "cotte", feet: "solerets", rules: rules_tank },
    Archetype { weapon: "hachette", off: None, head: "capuche", chest: "veste_cuir", feet: "bottes", rules: rules_dps },
    Archetype { weapon: "dague", off: None, head: "capuche", chest: "veste_cuir", feet: "bottes", rules: rules_dps },
    Archetype { weapon: "lance", off: None, head: "casque", chest: "veste_cuir", feet: "bottes", rules: rules_dps },
    Archetype { weapon: "arc_court", off: None, head: "capuche", chest: "veste_cuir", feet: "bottes", rules: rules_dps },
    Archetype { weapon: "arbalete", off: None, head: "casque", chest: "veste_cuir", feet: "bottes", rules: rules_dps },
    Archetype { weapon: "baton_feu", off: None, head: "chapeau_pointu", chest: "robe", feet: "sandales", rules: rules_dps },
    Archetype { weapon: "baton_givre", off: None, head: "chapeau_pointu", chest: "robe", feet: "sandales", rules: rules_dps },
    Archetype { weapon: "baton_sacre", off: None, head: "chapeau_pointu", chest: "robe", feet: "sandales", rules: rules_healer },
    Archetype { weapon: "baton_nature", off: None, head: "capuche", chest: "robe", feet: "bottes", rules: rules_healer },
    Archetype { weapon: "baton_maudit", off: None, head: "chapeau_pointu", chest: "robe", feet: "sandales", rules: rules_dps },
    Archetype { weapon: "baguette_givre", off: Some("bouclier_rond"), head: "casque", chest: "cotte", feet: "solerets", rules: rules_tank },
];

const TAVERN_LINES: &[&str] = &[
    "Encore un qui cherche la gloire ? Paie d'abord, on verra après.",
    "J'ai vu des loups gros comme des chevaux dans la forêt de Vergues.",
    "Ma lame est à vendre. Mon dos aussi, si le prix est bon.",
    "Si tu cherches quelqu'un pour tenir la ligne, c'est moi.",
    "On dit que Gérald le Borgne prépare un coup sur la route du sud.",
    "Je soigne, je ne me bats pas. Enfin... pas beaucoup.",
    "T'as une tête de porteur, toi. Pas de honte, on a tous commencé comme ça.",
    "La dernière compagnie avec qui je suis partie n'est pas revenue. Moi si.",
    "Un verre et je te raconte comment j'ai perdu ce doigt.",
    "Les marais ? Jamais sans quelqu'un qui maîtrise le feu.",
];

fn rand_color(rng: &mut Rng, palette: &[Rgb]) -> Rgb {
    *rng.pick(palette).unwrap_or(&(0.3, 0.2, 0.1))
}

impl World {
    /// Crée le monde de départ : ~30 aventuriers, quelques compagnies.
    pub fn generate(c: &Content, rng: &mut Rng) -> World {
        let mut w = World { next_id: 1, ..Default::default() };
        for _ in 0..32 {
            let tier = match rng.f32() {
                x if x < 0.45 => 1,
                x if x < 0.75 => 2,
                x if x < 0.92 => 3,
                _ => 4,
            };
            w.spawn_npc(c, rng, tier);
        }
        // Compagnies : un chef de tier élevé + des membres.
        let n_companies = 4.min(c.names.companies.len());
        let mut names = c.names.companies.clone();
        for _ in 0..n_companies {
            let idx = rng.index(names.len());
            let name = names.remove(idx);
            let leader_tier = 3 + (rng.f32() > 0.6) as u8;
            let leader_id = w.spawn_npc(c, rng, leader_tier);
            let mut members = vec![leader_id];
            let free: Vec<u32> = w.npcs.iter().filter(|n| n.company.is_none() && n.id != leader_id).map(|n| n.id).collect();
            for _ in 0..rng.range_u32(2, 4) {
                if let Some(&m) = rng.pick(&free) {
                    if !members.contains(&m) {
                        members.push(m);
                    }
                }
            }
            let id = w.next_id;
            w.next_id += 1;
            for m in &members {
                if let Some(n) = w.npcs.iter_mut().find(|n| n.id == *m) {
                    n.company = Some(id);
                }
            }
            let color = rand_color(rng, &[(0.6, 0.15, 0.12), (0.15, 0.3, 0.6), (0.2, 0.45, 0.2), (0.5, 0.4, 0.1), (0.35, 0.2, 0.45)]);
            w.companies.push(Company { id, name, leader: leader_id, members, renown: rng.range_u32(10, 80) as i32, color });
        }
        w
    }

    /// Crée un aventurier d'un tier donné, avec équipement et maîtrises cohérents.
    pub fn spawn_npc(&mut self, c: &Content, rng: &mut Rng, tier: u8) -> u32 {
        let female = rng.chance(0.5);
        let first = if female { rng.pick(&c.names.female) } else { rng.pick(&c.names.male) }
            .cloned()
            .unwrap_or_else(|| "Anonyme".into());
        let name = if rng.chance(0.4) {
            format!("{first} {}", rng.pick(&c.names.nicknames).cloned().unwrap_or_default())
        } else {
            format!("{first} {}", rng.pick(&c.names.surnames).cloned().unwrap_or_default())
        };
        let mut sheet = Character::new(name.trim());
        sheet.skin = rand_color(rng, &[(0.95, 0.8, 0.68), (0.85, 0.66, 0.5), (0.7, 0.5, 0.35), (0.5, 0.35, 0.25), (0.35, 0.25, 0.18)]);
        sheet.hair = rand_color(rng, &[(0.1, 0.08, 0.06), (0.3, 0.2, 0.12), (0.55, 0.35, 0.15), (0.75, 0.6, 0.35), (0.6, 0.25, 0.1), (0.8, 0.8, 0.8)]);
        sheet.hair_style = rng.range_u32(0, 3) as u8;
        let arch = &ARCHETYPES[rng.index(ARCHETYPES.len())];
        // Maîtrise de quoi porter ce tier, plus un peu.
        let lvl = crate::mastery::level_for_tier(tier) + rng.range_u32(0, 8);
        let mut uid = (self.next_id as u64) * 100;
        let mut equip = |sheet: &mut Character, base: &str, slot_tier: u8| {
            uid += 1;
            let b = c.base(base);
            let t = match b {
                Some(b) => slot_tier.clamp(b.min_tier, b.max_tier),
                None => return,
            };
            let mut it = Item::new(uid, base, t);
            it.quality = crate::loot::roll_quality(rng, 0.0);
            if let Some(line) = b.map(|b| b.line.clone()).filter(|l| !l.is_empty()) {
                let need = lvl.max(crate::mastery::level_for_tier(t));
                let cur = sheet.masteries.level(&line);
                if cur < need {
                    let missing = crate::mastery::total_fame_for(need) - crate::mastery::total_fame_for(cur) + 1.0;
                    sheet.masteries.add_fame(&line, missing.max(1.0));
                }
            }
            let _ = sheet.equip(c, it);
        };
        equip(&mut sheet, arch.weapon, tier);
        if let Some(off) = arch.off {
            equip(&mut sheet, off, tier);
        }
        equip(&mut sheet, arch.head, tier);
        equip(&mut sheet, arch.chest, tier);
        equip(&mut sheet, arch.feet, tier);
        if tier >= 2 {
            equip(&mut sheet, "potion_soin", tier.min(3));
        }
        // Un peu de labeur, comme tout le monde.
        sheet.masteries.add_fame("labeur", crate::mastery::total_fame_for(rng.range_u32(0, 8)) + 1.0);
        // Talents : les premiers débloqués, au hasard.
        let mut unlocked: Vec<String> = sheet.unlocked_talents(c).iter().map(|p| p.id.clone()).collect();
        while sheet.talents.len() < sheet.talent_slots() && !unlocked.is_empty() {
            let i = rng.index(unlocked.len());
            let t = unlocked.remove(i);
            sheet.toggle_talent(c, &t);
        }
        sheet.rules = (arch.rules)();
        sheet.fix_loadout(c);
        let mut traits = vec![];
        while traits.len() < 2 {
            let t = Trait::ALL[rng.index(Trait::ALL.len())];
            if !traits.contains(&t) {
                traits.push(t);
            }
        }
        let id = self.next_id;
        self.next_id += 1;
        self.npcs.push(Npc {
            id,
            sheet,
            traits,
            affinity: rng.range_u32(0, 20) as i32 - 5,
            company: None,
            alive: true,
            line: rng.pick(TAVERN_LINES).map(|s| s.to_string()).unwrap_or_default(),
            companion: false,
            hired_for: 0,
            gender_female: female,
        });
        id
    }

    pub fn npc(&self, id: u32) -> Option<&Npc> {
        self.npcs.iter().find(|n| n.id == id)
    }

    pub fn npc_mut(&mut self, id: u32) -> Option<&mut Npc> {
        self.npcs.iter_mut().find(|n| n.id == id)
    }

    /// Aventuriers libres présents à la taverne.
    pub fn tavern(&self) -> Vec<&Npc> {
        self.npcs.iter().filter(|n| n.alive && n.company.is_none() && !n.companion && n.hired_for == 0).collect()
    }

    /// Votre groupe actuel (compagnons + mercenaires sous contrat).
    pub fn party(&self) -> Vec<&Npc> {
        self.npcs.iter().filter(|n| n.alive && (n.companion || n.hired_for > 0)).collect()
    }

    pub fn company(&self, id: u32) -> Option<&Company> {
        self.companies.iter().find(|c| c.id == id)
    }

    /// Le temps passe : les PNJ progressent un peu hors écran.
    pub fn advance_day(&mut self, c: &Content, rng: &mut Rng) {
        for n in self.npcs.iter_mut().filter(|n| n.alive && !n.companion && n.hired_for == 0) {
            if rng.chance(0.3) {
                let line = n.sheet.weapon_line(c);
                n.sheet.masteries.add_fame(&line, rng.range_f32(20.0, 120.0) as f64);
                n.sheet.fix_loadout(c);
            }
        }
        // De temps en temps, un nouvel aventurier arrive en ville.
        if rng.chance(0.35) {
            let tier = if rng.chance(0.7) { 1 } else { 2 };
            self.spawn_npc(c, rng, tier);
        }
    }
}

/// Équipement de départ d'un PNJ pour l'affichage d'un slot.
pub fn slot_summary(c: &Content, n: &Npc, slot: EquipSlot) -> String {
    n.sheet
        .item(slot)
        .map(|it| crate::character::item_name(c, it))
        .unwrap_or_else(|| "—".into())
}

#[allow(dead_code)]
fn _quality_unused(_: Quality) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::content;

    #[test]
    fn monde_genere() {
        let c = content();
        let mut rng = Rng::new(5);
        let w = World::generate(c, &mut rng);
        assert!(w.npcs.len() >= 32);
        assert!(!w.companies.is_empty());
        for n in &w.npcs {
            assert!(n.sheet.item(EquipSlot::MainHand).is_some(), "{} sans arme", n.name());
            assert!(n.hire_price() > 0);
        }
        assert!(!w.tavern().is_empty());
    }

    #[test]
    fn rangs() {
        assert_eq!(Rank::from_rep(0), Rank::Porter);
        assert_eq!(Rank::from_rep(120), Rank::Member);
        assert_eq!(Rank::from_rep(9999), Rank::Officer);
    }
}
