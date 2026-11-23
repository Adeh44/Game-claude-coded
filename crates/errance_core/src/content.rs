//! Le contenu du jeu, lu depuis les fichiers `.ron` de `content/`.
//!
//! Les fichiers sont embarqués dans l'exécutable à la compilation : rien à
//! charger sur le téléphone. Pour ajouter une arme, un skill, un monstre :
//! on édite le `.ron`, on lance `cargo test` (le test `contenu_valide`
//! vérifie toutes les références) et c'est tout.

use crate::craft::{NodeKind, ResourceDef};
use crate::item::{AffixDef, EquipSlot, ItemBase, UniqueDef};
use crate::look::Rgb;
use crate::mastery::{LineCategory, LineDef};
use crate::passive::PassiveDef;
use crate::rules::Rule;
use crate::skill::{SkillDef, SkillSlot};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

/// Un monstre (ou un brigand).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnemyDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub tier: u8,
    /// Famille : "bete", "brigand", "mort", "esprit"...
    pub family: String,
    pub model: String,
    pub color: Rgb,
    #[serde(default = "one")]
    pub scale: f32,
    #[serde(default = "radius")]
    pub radius: f32,
    pub hp: f32,
    #[serde(default)]
    pub armor: f32,
    #[serde(default)]
    pub resist: f32,
    #[serde(default)]
    pub power: f32,
    #[serde(default = "speed")]
    pub speed: f32,
    #[serde(default)]
    pub crit: f32,
    #[serde(default)]
    pub dodge: f32,
    /// Skills par emplacement ; `Basic` obligatoire.
    pub skills: Vec<(SkillSlot, String)>,
    /// Règles d'IA ; vide = règles par défaut.
    #[serde(default)]
    pub rules: Vec<Rule>,
    pub fame: f32,
    #[serde(default)]
    pub gold: (u32, u32),
    /// Chance de lâcher un objet.
    #[serde(default)]
    pub loot_chance: f32,
    #[serde(default = "seven")]
    pub aggro: f32,
    #[serde(default = "leash")]
    pub leash: f32,
    #[serde(default)]
    pub boss: bool,
    /// Pièces d'apparence pour les humanoïdes : (où, modèle, couleur).
    #[serde(default)]
    pub parts: Vec<(String, String, Rgb)>,
}

fn one() -> f32 {
    1.0
}
fn radius() -> f32 {
    0.45
}
fn speed() -> f32 {
    3.6
}
fn seven() -> f32 {
    7.0
}
fn leash() -> f32 {
    22.0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MiniGame {
    /// Glisser des caisses jusqu'à la charrette.
    Carry,
    /// Taper au bon moment (couper du bois, forger).
    Rhythm,
    /// Frotter l'écran (nettoyer l'écurie, laver).
    Scrub,
    /// Livrer : toucher les bonnes portes dans l'ordre.
    Deliver,
}

/// Petit boulot de la ville.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChoreDef {
    pub id: String,
    pub name: String,
    pub description: String,
    pub giver: String,
    pub game: MiniGame,
    /// Paie en pièces.
    pub pay: u32,
    /// Gloire de Labeur.
    pub fame: f32,
    /// Niveau de Labeur requis.
    #[serde(default)]
    pub labor: u32,
    /// Difficulté du mini-jeu (1 = facile).
    #[serde(default = "one")]
    pub difficulty: f32,
}

/// Groupe de monstres qu'on peut croiser dans une zone.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pack {
    pub enemies: Vec<(String, u32)>,
    #[serde(default = "one")]
    pub weight: f32,
}

/// Une zone d'expédition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZoneDef {
    pub id: String,
    pub name: String,
    pub tier: u8,
    pub description: String,
    /// Couleur du sol, des arbres, des rochers.
    pub ground: Rgb,
    pub foliage: Rgb,
    pub rock: Rgb,
    /// Densité de décor (arbres pour 100 m²).
    #[serde(default = "one")]
    pub trees: f32,
    /// Type d'arbre : "feuillu", "sapin", "mort", "saule".
    #[serde(default)]
    pub tree_kind: String,
    pub packs: Vec<Pack>,
    /// Nombre de groupes de monstres.
    pub pack_count: u32,
    #[serde(default)]
    pub boss: Option<String>,
    /// Demi-largeur de la zone (m).
    #[serde(default = "zone_size")]
    pub size: f32,
    /// Couleur du ciel / brouillard.
    #[serde(default = "sky")]
    pub sky: Rgb,
    /// Nœuds de récolte : (type, nombre). Vide = un mélange par défaut.
    #[serde(default)]
    pub nodes: Vec<(NodeKind, u32)>,
    /// Passages d'escalade : une barre rocheuse coupe la zone, ces voies la franchissent.
    #[serde(default)]
    pub climbs: Vec<ClimbDef>,
}

/// Une voie d'escalade d'une zone (le tracé exact est tiré à chaque expédition).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClimbDef {
    pub name: String,
    pub tier: u8,
    /// Hauteur (m).
    pub height: f32,
    /// Part de prises verglacées.
    pub ice: f32,
}

impl ZoneDef {
    pub fn node_counts(&self) -> Vec<(NodeKind, u32)> {
        if self.nodes.is_empty() {
            vec![(NodeKind::Tree, 5), (NodeKind::Vein, 4), (NodeKind::Herb, 4)]
        } else {
            self.nodes.clone()
        }
    }
}

fn zone_size() -> f32 {
    34.0
}
fn sky() -> Rgb {
    (0.62, 0.72, 0.8)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NameLists {
    pub male: Vec<String>,
    pub female: Vec<String>,
    pub surnames: Vec<String>,
    pub nicknames: Vec<String>,
    pub companies: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Content {
    pub lines: Vec<LineDef>,
    pub skills: Vec<SkillDef>,
    pub passives: Vec<PassiveDef>,
    pub bases: Vec<ItemBase>,
    pub affixes: Vec<AffixDef>,
    pub uniques: Vec<UniqueDef>,
    pub enemies: Vec<EnemyDef>,
    pub chores: Vec<ChoreDef>,
    pub zones: Vec<ZoneDef>,
    pub names: NameLists,
    pub resources: Vec<ResourceDef>,
    pub quests: Vec<crate::quest::QuestDef>,
    line_idx: HashMap<String, usize>,
    skill_idx: HashMap<String, usize>,
    passive_idx: HashMap<String, usize>,
    base_idx: HashMap<String, usize>,
    affix_idx: HashMap<String, usize>,
    unique_idx: HashMap<String, usize>,
    enemy_idx: HashMap<String, usize>,
}

macro_rules! files {
    ($($f:literal),* $(,)?) => {
        &[$(($f, include_str!(concat!("../content/", $f)))),*]
    };
}

const LINE_FILES: &[(&str, &str)] = files!["voies.ron"];
const SKILL_FILES: &[(&str, &str)] = files![
    "skills_debut.ron",
    "skills_melee.ron",
    "skills_distance.ron",
    "skills_magie.ron",
    "skills_armures.ron",
    "skills_monstres.ron",
];
const PASSIVE_FILES: &[(&str, &str)] = files!["talents.ron"];
const BASE_FILES: &[(&str, &str)] = files!["objets_armes.ron", "objets_armures.ron", "objets_divers.ron"];
const AFFIX_FILES: &[(&str, &str)] = files!["affixes.ron"];
const UNIQUE_FILES: &[(&str, &str)] = files!["uniques.ron"];
const ENEMY_FILES: &[(&str, &str)] = files!["ennemis.ron"];
const CHORE_FILES: &[(&str, &str)] = files!["corvees.ron"];
const ZONE_FILES: &[(&str, &str)] = files!["zones.ron"];
const RESOURCE_FILES: &[(&str, &str)] = files!["ressources.ron"];
const QUEST_FILES: &[(&str, &str)] = files!["quetes.ron"];
const NAMES: &str = include_str!("../content/noms.ron");

fn parse_all<T: for<'de> Deserialize<'de>>(files: &[(&str, &str)], errors: &mut Vec<String>) -> Vec<T> {
    let mut out = vec![];
    for (name, text) in files {
        match ron::from_str::<Vec<T>>(text) {
            Ok(mut v) => out.append(&mut v),
            Err(e) => errors.push(format!("{name}: {e}")),
        }
    }
    out
}

fn index<T>(items: &[T], id: impl Fn(&T) -> &str, what: &str, errors: &mut Vec<String>) -> HashMap<String, usize> {
    let mut m = HashMap::new();
    for (i, it) in items.iter().enumerate() {
        if m.insert(id(it).to_string(), i).is_some() {
            errors.push(format!("{what} en double : {}", id(it)));
        }
    }
    m
}

impl Content {
    /// Lit tout le contenu. Renvoie aussi la liste des problèmes trouvés.
    pub fn load() -> (Content, Vec<String>) {
        let mut errors = vec![];
        let lines: Vec<LineDef> = parse_all(LINE_FILES, &mut errors);
        let skills: Vec<SkillDef> = parse_all(SKILL_FILES, &mut errors);
        let passives: Vec<PassiveDef> = parse_all(PASSIVE_FILES, &mut errors);
        let bases: Vec<ItemBase> = parse_all(BASE_FILES, &mut errors);
        let affixes: Vec<AffixDef> = parse_all(AFFIX_FILES, &mut errors);
        let uniques: Vec<UniqueDef> = parse_all(UNIQUE_FILES, &mut errors);
        let enemies: Vec<EnemyDef> = parse_all(ENEMY_FILES, &mut errors);
        let chores: Vec<ChoreDef> = parse_all(CHORE_FILES, &mut errors);
        let zones: Vec<ZoneDef> = parse_all(ZONE_FILES, &mut errors);
        let resources: Vec<ResourceDef> = parse_all(RESOURCE_FILES, &mut errors);
        let quests: Vec<crate::quest::QuestDef> = parse_all(QUEST_FILES, &mut errors);
        let names = match ron::from_str::<NameLists>(NAMES) {
            Ok(n) => n,
            Err(e) => {
                errors.push(format!("noms.ron: {e}"));
                NameLists::default()
            }
        };
        let line_idx = index(&lines, |l| &l.id, "voie", &mut errors);
        let skill_idx = index(&skills, |s| &s.id, "skill", &mut errors);
        let passive_idx = index(&passives, |s| &s.id, "talent", &mut errors);
        let base_idx = index(&bases, |s| &s.id, "objet", &mut errors);
        let affix_idx = index(&affixes, |s| &s.id, "affixe", &mut errors);
        let unique_idx = index(&uniques, |s| &s.id, "unique", &mut errors);
        let enemy_idx = index(&enemies, |s| &s.id, "ennemi", &mut errors);
        let c = Content {
            lines,
            skills,
            passives,
            bases,
            affixes,
            uniques,
            enemies,
            chores,
            zones,
            names,
            resources,
            quests,
            line_idx,
            skill_idx,
            passive_idx,
            base_idx,
            affix_idx,
            unique_idx,
            enemy_idx,
        };
        errors.extend(c.validate());
        (c, errors)
    }

    /// Vérifie que toutes les références pointent vers quelque chose.
    pub fn validate(&self) -> Vec<String> {
        let mut e = vec![];
        for s in &self.skills {
            if !s.line.is_empty() && self.line(&s.line).is_none() && !s.line.starts_with("monstre") {
                e.push(format!("skill {} : voie inconnue {}", s.id, s.line));
            }
            if s.effects.is_empty() {
                e.push(format!("skill {} : aucun effet", s.id));
            }
        }
        for p in &self.passives {
            if self.line(&p.line).is_none() {
                e.push(format!("talent {} : voie inconnue {}", p.id, p.line));
            }
        }
        for b in &self.bases {
            if !b.line.is_empty() && self.line(&b.line).is_none() {
                e.push(format!("objet {} : voie inconnue {}", b.id, b.line));
            }
            if b.slot == EquipSlot::MainHand {
                match self.skill(&b.basic) {
                    Some(s) if s.slot == SkillSlot::Basic => {}
                    Some(_) => e.push(format!("objet {} : {} n'est pas une attaque de base", b.id, b.basic)),
                    None => e.push(format!("objet {} : attaque de base inconnue {}", b.id, b.basic)),
                }
                if !self.skills.iter().any(|s| s.line == b.line && s.slot == SkillSlot::Weapon1) {
                    e.push(format!("objet {} : sa voie {} n'a aucun skill Arme I", b.id, b.line));
                }
            }
            if !b.use_skill.is_empty() && self.skill(&b.use_skill).is_none() {
                e.push(format!("objet {} : skill d'usage inconnu {}", b.id, b.use_skill));
            }
            if b.min_tier < 1 || b.max_tier > 8 || b.min_tier > b.max_tier {
                e.push(format!("objet {} : tiers invalides", b.id));
            }
        }
        for q in &self.quests {
            e.extend(crate::quest::validate(self, q));
        }
        for u in &self.uniques {
            if self.base(&u.base).is_none() {
                e.push(format!("unique {} : base inconnue {}", u.id, u.base));
            }
        }
        for en in &self.enemies {
            if !en.skills.iter().any(|(s, _)| *s == SkillSlot::Basic) {
                e.push(format!("ennemi {} : pas d'attaque de base", en.id));
            }
            for (_, sk) in &en.skills {
                if self.skill(sk).is_none() {
                    e.push(format!("ennemi {} : skill inconnu {}", en.id, sk));
                }
            }
        }
        for z in &self.zones {
            for p in &z.packs {
                for (id, _) in &p.enemies {
                    if self.enemy(id).is_none() {
                        e.push(format!("zone {} : ennemi inconnu {}", z.id, id));
                    }
                }
            }
            if let Some(b) = &z.boss {
                if self.enemy(b).is_none() {
                    e.push(format!("zone {} : boss inconnu {}", z.id, b));
                }
            }
        }
        for r in &self.resources {
            if r.tiers.len() != 8 {
                e.push(format!("ressource {} : il faut 8 noms de tiers", r.id));
            }
            if !r.secondary.is_empty() && self.resource(&r.secondary).is_none() {
                e.push(format!("ressource {} : ressource d'appoint inconnue {}", r.id, r.secondary));
            }
        }
        for kind in [NodeKind::Tree, NodeKind::Vein, NodeKind::Herb, NodeKind::Carcass] {
            if self.resource_of(kind).is_none() {
                e.push(format!("aucune ressource pour les nœuds {kind:?}"));
            }
        }
        for l in [crate::craft::LINE_GATHER, crate::craft::LINE_FORGE] {
            if self.line(l).is_none() {
                e.push(format!("voie de métier manquante : {l}"));
            }
        }
        // Chaque voie d'arme a au moins un skill par emplacement d'arme.
        for l in self.lines.iter().filter(|l| l.category == LineCategory::Weapon) {
            for slot in [SkillSlot::Basic, SkillSlot::Weapon1, SkillSlot::Weapon2, SkillSlot::Weapon3] {
                if !self.skills.iter().any(|s| s.line == l.id && s.slot == slot) {
                    e.push(format!("voie {} : aucun skill {:?}", l.id, slot));
                }
            }
            if !self.skills.iter().any(|s| s.line == l.id && s.slot != SkillSlot::Basic && s.mastery == 0) {
                e.push(format!("voie {} : aucun skill dispo au niveau 0", l.id));
            }
        }
        for l in self.lines.iter().filter(|l| l.category == LineCategory::Armor) {
            for slot in [SkillSlot::Head, SkillSlot::Chest, SkillSlot::Feet] {
                if !self.skills.iter().any(|s| s.line == l.id && s.slot == slot && s.mastery == 0) {
                    e.push(format!("armure {} : aucun skill {:?} au niveau 0", l.id, slot));
                }
            }
        }
        e
    }

    pub fn line(&self, id: &str) -> Option<&LineDef> {
        self.line_idx.get(id).map(|&i| &self.lines[i])
    }
    pub fn skill(&self, id: &str) -> Option<&SkillDef> {
        self.skill_idx.get(id).map(|&i| &self.skills[i])
    }
    pub fn passive(&self, id: &str) -> Option<&PassiveDef> {
        self.passive_idx.get(id).map(|&i| &self.passives[i])
    }
    pub fn base(&self, id: &str) -> Option<&ItemBase> {
        self.base_idx.get(id).map(|&i| &self.bases[i])
    }
    pub fn affix(&self, id: &str) -> Option<&AffixDef> {
        self.affix_idx.get(id).map(|&i| &self.affixes[i])
    }
    pub fn unique(&self, id: &str) -> Option<&UniqueDef> {
        self.unique_idx.get(id).map(|&i| &self.uniques[i])
    }
    pub fn enemy(&self, id: &str) -> Option<&EnemyDef> {
        self.enemy_idx.get(id).map(|&i| &self.enemies[i])
    }
    pub fn zone(&self, id: &str) -> Option<&ZoneDef> {
        self.zones.iter().find(|z| z.id == id)
    }

    pub fn resource(&self, id: &str) -> Option<&ResourceDef> {
        self.resources.iter().find(|r| r.id == id)
    }

    /// La ressource qu'on tire d'un type de nœud.
    pub fn resource_of(&self, kind: NodeKind) -> Option<&ResourceDef> {
        self.resources.iter().find(|r| r.node == kind)
    }

    pub fn quest(&self, id: &str) -> Option<&crate::quest::QuestDef> {
        self.quests.iter().find(|q| q.id == id)
    }

    pub fn chore(&self, id: &str) -> Option<&ChoreDef> {
        self.chores.iter().find(|c| c.id == id)
    }

    /// Skills d'une voie pour un emplacement, triés par niveau requis.
    pub fn skills_for(&self, line: &str, slot: SkillSlot) -> Vec<&SkillDef> {
        let mut v: Vec<&SkillDef> = self.skills.iter().filter(|s| s.line == line && s.slot == slot).collect();
        v.sort_by_key(|s| s.mastery);
        v
    }

    pub fn lines_of(&self, cat: LineCategory) -> Vec<&LineDef> {
        self.lines.iter().filter(|l| l.category == cat).collect()
    }
}

static CONTENT: OnceLock<Content> = OnceLock::new();

/// Le contenu global (lu une seule fois).
pub fn content() -> &'static Content {
    CONTENT.get_or_init(|| {
        let (c, errors) = Content::load();
        for e in &errors {
            eprintln!("[contenu] {e}");
        }
        c
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contenu_valide() {
        let (c, errors) = Content::load();
        assert!(errors.is_empty(), "Problèmes dans le contenu :\n{}", errors.join("\n"));
        println!(
            "voies {} · skills {} · talents {} · objets {} · affixes {} · uniques {} · ennemis {} · corvées {} · zones {} · ressources {} · quêtes {}",
            c.lines.len(), c.skills.len(), c.passives.len(), c.bases.len(), c.affixes.len(),
            c.uniques.len(), c.enemies.len(), c.chores.len(), c.zones.len(), c.resources.len(), c.quests.len()
        );
        assert!(c.skills.len() > 100);
        assert!(c.bases.len() > 40);
        assert!(c.passives.len() > 50);
    }
}
