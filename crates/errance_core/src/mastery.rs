//! Voies et maîtrises : on progresse en pratiquant.
//!
//! Chaque arme, armure, main secondaire et métier a sa voie. La gloire
//! gagnée en l'utilisant fait monter son niveau (0 → 100). Pas de niveau
//! global : un épéiste vétéran peut être un archer débutant.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum LineCategory {
    Weapon,
    OffHand,
    Armor,
    Job,
}

impl LineCategory {
    pub fn label(self) -> &'static str {
        match self {
            LineCategory::Weapon => "Armes",
            LineCategory::OffHand => "Mains secondaires",
            LineCategory::Armor => "Armures",
            LineCategory::Job => "Métiers",
        }
    }
}

/// Description d'une voie (contenu).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineDef {
    pub id: String,
    pub name: String,
    pub category: LineCategory,
    pub description: String,
    /// Couleur d'interface (r, g, b) 0..1.
    #[serde(default = "default_color")]
    pub color: (f32, f32, f32),
}

fn default_color() -> (f32, f32, f32) {
    (0.7, 0.7, 0.7)
}

pub const MAX_LEVEL: u32 = 100;

/// Gloire nécessaire pour passer du niveau `level` au suivant.
/// Courbe longue : quelques minutes pour les premiers niveaux, des dizaines
/// d'heures pour les derniers.
pub fn fame_for_next(level: u32) -> f64 {
    if level >= MAX_LEVEL {
        return f64::INFINITY;
    }
    40.0 * 1.075f64.powi(level as i32)
}

/// Gloire totale pour atteindre `level` depuis 0.
pub fn total_fame_for(level: u32) -> f64 {
    (0..level.min(MAX_LEVEL)).map(fame_for_next).sum()
}

/// Niveau de maîtrise requis pour porter un tier d'équipement.
pub fn level_for_tier(tier: u8) -> u32 {
    match tier {
        0 | 1 => 0,
        2 => 4,
        3 => 12,
        4 => 22,
        5 => 35,
        6 => 50,
        7 => 68,
        _ => 85,
    }
}

/// Bonus de puissance d'objet apporté par la maîtrise (+1 % par niveau).
pub fn item_power_bonus(level: u32) -> f32 {
    level.min(MAX_LEVEL) as f32 * 0.01
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Progress {
    pub level: u32,
    /// Gloire accumulée vers le prochain niveau.
    pub fame: f64,
    /// Gloire totale jamais gagnée (statistiques).
    pub lifetime: f64,
}

/// Résultat d'un gain de gloire, pour célébrer à l'écran.
#[derive(Debug, Clone, PartialEq)]
pub struct LevelUp {
    pub line: String,
    pub from: u32,
    pub to: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Masteries {
    pub lines: BTreeMap<String, Progress>,
}

impl Masteries {
    pub fn level(&self, line: &str) -> u32 {
        self.lines.get(line).map(|p| p.level).unwrap_or(0)
    }

    pub fn progress(&self, line: &str) -> Progress {
        self.lines.get(line).cloned().unwrap_or_default()
    }

    /// Fraction (0..1) vers le prochain niveau.
    pub fn fraction(&self, line: &str) -> f32 {
        let p = self.progress(line);
        if p.level >= MAX_LEVEL {
            return 1.0;
        }
        (p.fame / fame_for_next(p.level)) as f32
    }

    /// Somme de tous les niveaux (débloque des emplacements de talents).
    pub fn total(&self) -> u32 {
        self.lines.values().map(|p| p.level).sum()
    }

    pub fn total_in(&self, lines: impl Iterator<Item = String>) -> u32 {
        lines.map(|l| self.level(&l)).sum()
    }

    /// Ajoute de la gloire à une voie. Renvoie un `LevelUp` si on a monté.
    pub fn add_fame(&mut self, line: &str, fame: f64) -> Option<LevelUp> {
        if fame <= 0.0 {
            return None;
        }
        let p = self.lines.entry(line.to_string()).or_default();
        let from = p.level;
        p.lifetime += fame;
        if p.level >= MAX_LEVEL {
            return None;
        }
        p.fame += fame;
        while p.level < MAX_LEVEL && p.fame >= fame_for_next(p.level) {
            p.fame -= fame_for_next(p.level);
            p.level += 1;
        }
        if p.level >= MAX_LEVEL {
            p.fame = 0.0;
        }
        (p.level > from).then(|| LevelUp { line: line.to_string(), from, to: p.level })
    }

    pub fn can_wear_tier(&self, line: &str, tier: u8) -> bool {
        self.level(line) >= level_for_tier(tier)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monter_de_niveau() {
        let mut m = Masteries::default();
        assert_eq!(m.level("epee"), 0);
        let up = m.add_fame("epee", fame_for_next(0) + 1.0).unwrap();
        assert_eq!((up.from, up.to), (0, 1));
        assert_eq!(m.level("epee"), 1);
    }

    #[test]
    fn plafond_100() {
        let mut m = Masteries::default();
        m.add_fame("epee", 1e12);
        assert_eq!(m.level("epee"), MAX_LEVEL);
        assert!(m.add_fame("epee", 1e6).is_none());
    }

    #[test]
    fn courbe_longue() {
        // Le dernier niveau coûte beaucoup plus que le premier.
        assert!(fame_for_next(99) > 1000.0 * fame_for_next(0));
        // Mais tout reste atteignable : total fini.
        assert!(total_fame_for(100).is_finite());
    }

    #[test]
    fn tiers_progressifs() {
        for t in 1..8u8 {
            assert!(level_for_tier(t + 1) > level_for_tier(t));
        }
    }
}
