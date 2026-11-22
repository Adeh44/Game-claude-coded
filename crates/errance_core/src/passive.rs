//! Talents passifs : ce qu'on choisit en plus de l'équipement.
//!
//! Ils se débloquent dans l'Atlas des Voies (par la maîtrise) puis
//! s'équipent dans un nombre limité d'emplacements de talents.

use crate::skill::{Effect, StatusKind, Tag};
use crate::stats::{Modifier, Stat};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PassiveCond {
    Always,
    /// PV sous ce seuil (fraction).
    HpBelow(f32),
    HpAbove(f32),
    /// Tant qu'on a un bouclier actif.
    Shielded,
    /// Tant qu'on est sous cet état.
    While(StatusKind),
    /// Tant qu'au moins N ennemis sont à 4 m.
    Surrounded(u32),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PassiveEffect {
    /// Bonus de stat permanent.
    Stat(Modifier),
    /// Bonus de stat sous condition.
    StatWhile { cond: PassiveCond, modifier: Modifier },
    /// +% de puissance (dégâts, soins, boucliers) des skills portant ce mot-clé.
    TagPower { tag: Tag, pct: f32 },
    /// -% de recharge des skills portant ce mot-clé.
    TagCooldown { tag: Tag, pct: f32 },
    /// Quand un de vos coups (avec ce mot-clé, ou n'importe lequel) touche :
    /// applique `effect` à la cible.
    OnHit {
        #[serde(default)]
        tag: Option<Tag>,
        chance: f32,
        effect: Effect,
    },
    /// Quand un de vos coups touche : applique `effect` à vous-même.
    OnHitSelf {
        #[serde(default)]
        tag: Option<Tag>,
        chance: f32,
        effect: Effect,
        /// Délai minimal entre deux déclenchements.
        #[serde(default)]
        internal_cd: f32,
    },
    /// Quand vous soignez un allié : applique aussi `effect` à cet allié.
    OnHeal { effect: Effect },
    /// Quand vous êtes touché : applique `effect` (à l'attaquant ou à vous).
    OnDamaged {
        chance: f32,
        effect: Effect,
        to_attacker: bool,
        #[serde(default)]
        internal_cd: f32,
    },
    /// Quand vous achevez un ennemi : applique `effect` à vous-même.
    OnKill { effect: Effect },
    /// Convertit une stat en une autre : to += from × ratio.
    Convert { from: Stat, to: Stat, ratio: f32 },
    /// Allonge (+) ou raccourcit (-) les états que vous infligez de ce type.
    StatusDuration { status: StatusKind, pct: f32 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PassiveDef {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Voie qui le débloque.
    pub line: String,
    /// Niveau de maîtrise requis dans cette voie.
    #[serde(default)]
    pub mastery: u32,
    /// Si vrai : ne marche que si un objet de cette voie est équipé.
    #[serde(default)]
    pub needs_equipped: bool,
    pub effects: Vec<PassiveEffect>,
}

/// Nombre d'emplacements de talents selon la maîtrise totale cumulée.
/// 1 au départ, jusqu'à 6 quand on a beaucoup pratiqué.
pub fn talent_slots(total_mastery: u32) -> usize {
    match total_mastery {
        0..=9 => 1,
        10..=39 => 2,
        40..=119 => 3,
        120..=299 => 4,
        300..=599 => 5,
        _ => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn emplacements_croissants() {
        let mut last = 0;
        for m in (0..1000).step_by(5) {
            let s = talent_slots(m);
            assert!(s >= last);
            last = s;
        }
        assert_eq!(talent_slots(0), 1);
        assert_eq!(talent_slots(5000), 6);
    }
}
