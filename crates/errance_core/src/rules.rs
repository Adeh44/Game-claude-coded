//! Règles de comportement (« si … alors … »), pour les compagnons et les
//! monstres.
//!
//! Une liste ordonnée : à chaque décision, la première règle dont la
//! condition est vraie et dont l'action est possible gagne.

use crate::skill::{SkillSlot, StatusKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Condition {
    Always,
    /// Mes PV sous ce seuil (fraction).
    SelfHpBelow(f32),
    /// Un allié (moi compris) sous ce seuil → il devient la cible.
    AllyHpBelow(f32),
    /// Mon énergie au-dessus de ce seuil (fraction).
    SelfEnergyAbove(f32),
    /// Au moins N ennemis à moins de R mètres de moi.
    EnemiesNear { count: u32, radius: f32 },
    /// Un ennemi attaque un allié qui n'est pas moi → cet ennemi devient la cible.
    AllyAttacked,
    /// La cible est sous ce seuil de PV.
    TargetHpBelow(f32),
    /// La cible est en train d'incanter.
    TargetCasting,
    /// La cible n'a pas cet état.
    TargetMissing(StatusKind),
    /// Je n'ai pas cet état.
    SelfMissing(StatusKind),
    /// Un allié a un malus → il devient la cible.
    AllyDebuffed,
    /// Je suis la cible d'un ennemi.
    SelfTargeted,
}

impl Condition {
    pub fn label(&self) -> String {
        match self {
            Condition::Always => "Toujours".into(),
            Condition::SelfHpBelow(f) => format!("Mes PV < {} %", (f * 100.0) as i32),
            Condition::AllyHpBelow(f) => format!("Allié PV < {} %", (f * 100.0) as i32),
            Condition::SelfEnergyAbove(f) => format!("Mon énergie > {} %", (f * 100.0) as i32),
            Condition::EnemiesNear { count, radius } => format!("{count}+ ennemis à {radius} m"),
            Condition::AllyAttacked => "Un allié est attaqué".into(),
            Condition::TargetHpBelow(f) => format!("Cible PV < {} %", (f * 100.0) as i32),
            Condition::TargetCasting => "La cible incante".into(),
            Condition::TargetMissing(s) => format!("Cible sans {}", s.label()),
            Condition::SelfMissing(s) => format!("Moi sans {}", s.label()),
            Condition::AllyDebuffed => "Allié avec un malus".into(),
            Condition::SelfTargeted => "Je suis visé".into(),
        }
    }

    /// Conditions proposées dans l'éditeur.
    pub fn palette() -> Vec<Condition> {
        vec![
            Condition::Always,
            Condition::SelfHpBelow(0.3),
            Condition::SelfHpBelow(0.6),
            Condition::AllyHpBelow(0.4),
            Condition::AllyHpBelow(0.7),
            Condition::SelfEnergyAbove(0.5),
            Condition::EnemiesNear { count: 2, radius: 4.0 },
            Condition::EnemiesNear { count: 3, radius: 5.0 },
            Condition::AllyAttacked,
            Condition::TargetHpBelow(0.3),
            Condition::TargetCasting,
            Condition::TargetMissing(StatusKind::Burn),
            Condition::TargetMissing(StatusKind::Chill),
            Condition::TargetMissing(StatusKind::Bleed),
            Condition::TargetMissing(StatusKind::Poison),
            Condition::TargetMissing(StatusKind::Vulnerable),
            Condition::SelfMissing(StatusKind::Fortify),
            Condition::SelfMissing(StatusKind::Empower),
            Condition::AllyDebuffed,
            Condition::SelfTargeted,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    /// Utiliser le skill de cet emplacement (sur la cible désignée par la condition).
    Use(SkillSlot),
    /// Revenir près du chef.
    Regroup,
    /// Fuir le danger.
    Retreat,
}

impl Action {
    pub fn label(&self) -> String {
        match self {
            Action::Use(s) => format!("Utiliser {}", s.label()),
            Action::Regroup => "Se regrouper".into(),
            Action::Retreat => "Reculer".into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    pub when: Condition,
    pub then: Action,
    #[serde(default = "yes")]
    pub enabled: bool,
}

fn yes() -> bool {
    true
}

impl Rule {
    pub const fn new(when: Condition, then: Action) -> Self {
        Self { when, then, enabled: true }
    }
}

/// Ordres rapides du joueur à ses compagnons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Order {
    /// Attaquer la cible du chef (par défaut).
    #[default]
    Assist,
    /// Ne pas combattre, suivre le chef.
    Regroup,
    /// Défendre : ne viser que ce qui attaque le groupe, rester près.
    Defensive,
}

impl Order {
    pub fn label(self) -> &'static str {
        match self {
            Order::Assist => "Attaquez !",
            Order::Regroup => "Regroupez !",
            Order::Defensive => "Défendez !",
        }
    }
}

/// Règles par défaut raisonnables pour tout le monde.
pub fn default_rules() -> Vec<Rule> {
    vec![
        Rule::new(Condition::Always, Action::Use(SkillSlot::Weapon3)),
        Rule::new(Condition::Always, Action::Use(SkillSlot::Weapon2)),
        Rule::new(Condition::Always, Action::Use(SkillSlot::Weapon1)),
        Rule::new(Condition::Always, Action::Use(SkillSlot::Head)),
        Rule::new(Condition::Always, Action::Use(SkillSlot::Chest)),
    ]
}
