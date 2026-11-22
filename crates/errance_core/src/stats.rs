//! Les caractéristiques d'un combattant.
//!
//! Pas de « Force » à répartir : tout vient de l'équipement, des maîtrises et
//! des talents. Une stat finale = (base + bonus fixes) × (1 + bonus en %).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Stat {
    MaxHp,
    MaxEnergy,
    HpRegen,
    EnergyRegen,
    Armor,
    MagicResist,
    PhysicalPower,
    MagicPower,
    HealPower,
    /// Vitesse d'attaque de base, en %.
    AttackSpeed,
    /// Réduit les temps de recharge et d'incantation, en %.
    Haste,
    MoveSpeed,
    CritChance,
    CritDamage,
    /// Multiplie la menace générée (tanks > 1).
    ThreatMult,
    BlockChance,
    Lifesteal,
    ShieldPower,
    /// Allonge la durée des états infligés, en %.
    ControlPower,
    /// Raccourcit les états de contrôle subis, en %.
    Tenacity,
    DodgeChance,
    /// Bonus de puissance d'objet : multiplie les stats de l'équipement.
    ItemPower,
}

pub const STAT_COUNT: usize = 22;

impl Stat {
    pub const ALL: [Stat; STAT_COUNT] = [
        Stat::MaxHp,
        Stat::MaxEnergy,
        Stat::HpRegen,
        Stat::EnergyRegen,
        Stat::Armor,
        Stat::MagicResist,
        Stat::PhysicalPower,
        Stat::MagicPower,
        Stat::HealPower,
        Stat::AttackSpeed,
        Stat::Haste,
        Stat::MoveSpeed,
        Stat::CritChance,
        Stat::CritDamage,
        Stat::ThreatMult,
        Stat::BlockChance,
        Stat::Lifesteal,
        Stat::ShieldPower,
        Stat::ControlPower,
        Stat::Tenacity,
        Stat::DodgeChance,
        Stat::ItemPower,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    /// Nom affiché au joueur.
    pub fn label(self) -> &'static str {
        match self {
            Stat::MaxHp => "Points de vie",
            Stat::MaxEnergy => "Énergie",
            Stat::HpRegen => "Régén. de vie",
            Stat::EnergyRegen => "Régén. d'énergie",
            Stat::Armor => "Armure",
            Stat::MagicResist => "Résistance magique",
            Stat::PhysicalPower => "Puissance physique",
            Stat::MagicPower => "Puissance magique",
            Stat::HealPower => "Puissance de soin",
            Stat::AttackSpeed => "Vitesse d'attaque",
            Stat::Haste => "Célérité",
            Stat::MoveSpeed => "Vitesse",
            Stat::CritChance => "Chance de critique",
            Stat::CritDamage => "Dégâts critiques",
            Stat::ThreatMult => "Menace",
            Stat::BlockChance => "Blocage",
            Stat::Lifesteal => "Vol de vie",
            Stat::ShieldPower => "Puissance de bouclier",
            Stat::ControlPower => "Puissance de contrôle",
            Stat::Tenacity => "Ténacité",
            Stat::DodgeChance => "Esquive",
            Stat::ItemPower => "Puissance d'objet",
        }
    }

    /// Les stats exprimées en fraction (0.1 = 10 %) s'affichent en %.
    pub fn is_ratio(self) -> bool {
        matches!(
            self,
            Stat::AttackSpeed
                | Stat::Haste
                | Stat::CritChance
                | Stat::CritDamage
                | Stat::BlockChance
                | Stat::Lifesteal
                | Stat::ControlPower
                | Stat::Tenacity
                | Stat::DodgeChance
                | Stat::ThreatMult
        )
    }

    /// Plafond pour éviter les builds cassés (100 % esquive...).
    pub fn cap(self) -> Option<f32> {
        match self {
            Stat::CritChance => Some(0.75),
            Stat::BlockChance => Some(0.6),
            Stat::DodgeChance => Some(0.5),
            Stat::Haste => Some(0.6),
            Stat::Lifesteal => Some(0.4),
            Stat::Tenacity => Some(0.7),
            Stat::MoveSpeed => Some(9.0),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModKind {
    /// S'ajoute à la base.
    Flat,
    /// Pourcentage additif (0.1 = +10 %).
    Percent,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Modifier {
    pub stat: Stat,
    pub kind: ModKind,
    pub value: f32,
}

impl Modifier {
    pub const fn flat(stat: Stat, value: f32) -> Self {
        Self { stat, kind: ModKind::Flat, value }
    }
    pub const fn pct(stat: Stat, value: f32) -> Self {
        Self { stat, kind: ModKind::Percent, value }
    }

    pub fn describe(&self) -> String {
        let sign = if self.value >= 0.0 { "+" } else { "" };
        match self.kind {
            ModKind::Flat if self.stat.is_ratio() => {
                format!("{sign}{:.0} % {}", self.value * 100.0, self.stat.label())
            }
            ModKind::Flat => format!("{sign}{} {}", fmt_num(self.value), self.stat.label()),
            ModKind::Percent => {
                format!("{sign}{:.0} % {}", self.value * 100.0, self.stat.label())
            }
        }
    }
}

pub fn fmt_num(v: f32) -> String {
    if (v - v.round()).abs() < 0.05 {
        format!("{:.0}", v)
    } else {
        format!("{:.1}", v)
    }
}

/// Accumulateur de modificateurs, puis lecture des valeurs finales.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatBlock {
    flat: [f32; STAT_COUNT],
    pct: [f32; STAT_COUNT],
}

impl Default for StatBlock {
    fn default() -> Self {
        Self { flat: [0.0; STAT_COUNT], pct: [0.0; STAT_COUNT] }
    }
}

impl StatBlock {
    pub fn new() -> Self {
        Self::default()
    }

    /// Stats d'un humain sans rien : fragile, lent, faible.
    pub fn human_base() -> Self {
        let mut s = Self::new();
        s.add_flat(Stat::MaxHp, 60.0);
        s.add_flat(Stat::MaxEnergy, 40.0);
        s.add_flat(Stat::HpRegen, 0.6);
        s.add_flat(Stat::EnergyRegen, 3.0);
        s.add_flat(Stat::MoveSpeed, 4.2);
        s.add_flat(Stat::CritDamage, 0.5);
        s.add_flat(Stat::ThreatMult, 1.0);
        s.add_flat(Stat::PhysicalPower, 4.0);
        s.add_flat(Stat::MagicPower, 4.0);
        s.add_flat(Stat::HealPower, 4.0);
        s
    }

    pub fn add(&mut self, m: Modifier) {
        match m.kind {
            ModKind::Flat => self.flat[m.stat.index()] += m.value,
            ModKind::Percent => self.pct[m.stat.index()] += m.value,
        }
    }

    pub fn add_all<'a>(&mut self, mods: impl IntoIterator<Item = &'a Modifier>) {
        for m in mods {
            self.add(*m);
        }
    }

    pub fn add_flat(&mut self, stat: Stat, v: f32) {
        self.flat[stat.index()] += v;
    }

    pub fn add_pct(&mut self, stat: Stat, v: f32) {
        self.pct[stat.index()] += v;
    }

    pub fn merge(&mut self, other: &StatBlock) {
        for i in 0..STAT_COUNT {
            self.flat[i] += other.flat[i];
            self.pct[i] += other.pct[i];
        }
    }

    /// Valeur finale, plafonnée si besoin, jamais négative.
    pub fn get(&self, stat: Stat) -> f32 {
        let i = stat.index();
        let v = (self.flat[i] * (1.0 + self.pct[i])).max(0.0);
        match stat.cap() {
            Some(c) => v.min(c),
            None => v,
        }
    }

    pub fn flat(&self, stat: Stat) -> f32 {
        self.flat[stat.index()]
    }
}

/// Réduction de dégâts d'une armure / résistance : r / (r + 100).
/// 100 d'armure = -50 %, 300 = -75 %. Ne peut jamais atteindre 100 %.
pub fn mitigation(resist: f32) -> f32 {
    let r = resist.max(0.0);
    r / (r + 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calcul_final() {
        let mut s = StatBlock::new();
        s.add(Modifier::flat(Stat::MaxHp, 100.0));
        s.add(Modifier::pct(Stat::MaxHp, 0.5));
        assert!((s.get(Stat::MaxHp) - 150.0).abs() < 1e-4);
    }

    #[test]
    fn plafonds() {
        let mut s = StatBlock::new();
        s.add_flat(Stat::DodgeChance, 3.0);
        assert_eq!(s.get(Stat::DodgeChance), 0.5);
    }

    #[test]
    fn mitigation_bornee() {
        assert_eq!(mitigation(0.0), 0.0);
        assert!((mitigation(100.0) - 0.5).abs() < 1e-6);
        assert!(mitigation(1_000_000.0) < 1.0);
    }

    #[test]
    fn toutes_les_stats_indexees() {
        for (i, s) in Stat::ALL.iter().enumerate() {
            assert_eq!(s.index(), i);
        }
    }
}
