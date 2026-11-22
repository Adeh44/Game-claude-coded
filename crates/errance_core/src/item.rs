//! Équipement : bases, tiers, qualités, affixes, objets uniques.
//!
//! Un objet = une base (« Épée longue ») × un tier (T1 → T8, le matériau)
//! × une qualité × des affixes tirés au hasard. Les uniques ajoutent un
//! effet spécial.

use crate::passive::PassiveEffect;
use crate::stats::{ModKind, Modifier, Stat};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum EquipSlot {
    MainHand,
    OffHand,
    Head,
    Chest,
    Feet,
    Cape,
    Amulet,
    Ring,
    Potion,
    Food,
}

impl EquipSlot {
    pub const ALL: [EquipSlot; 10] = [
        EquipSlot::MainHand,
        EquipSlot::OffHand,
        EquipSlot::Head,
        EquipSlot::Chest,
        EquipSlot::Feet,
        EquipSlot::Cape,
        EquipSlot::Amulet,
        EquipSlot::Ring,
        EquipSlot::Potion,
        EquipSlot::Food,
    ];

    pub fn label(self) -> &'static str {
        match self {
            EquipSlot::MainHand => "Arme",
            EquipSlot::OffHand => "Main secondaire",
            EquipSlot::Head => "Tête",
            EquipSlot::Chest => "Torse",
            EquipSlot::Feet => "Pieds",
            EquipSlot::Cape => "Cape",
            EquipSlot::Amulet => "Amulette",
            EquipSlot::Ring => "Anneau",
            EquipSlot::Potion => "Potion",
            EquipSlot::Food => "Repas",
        }
    }

    pub fn index(self) -> usize {
        self as usize
    }
}

/// Famille de matériau : donne les noms et couleurs des tiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Material {
    Metal,
    Wood,
    Leather,
    Cloth,
    Jewel,
    Alchemy,
}

impl Material {
    /// Complément de nom selon le tier : « Épée » + « d'acier ».
    pub fn tier_name(self, tier: u8) -> &'static str {
        let t = tier.clamp(1, 8) as usize - 1;
        let names: [&str; 8] = match self {
            Material::Metal => [
                "de fer rouillé",
                "de fer",
                "d'acier",
                "d'acier trempé",
                "de cobalt",
                "de mithril",
                "d'adamantite",
                "d'astralite",
            ],
            Material::Wood => [
                "de bois mort",
                "de frêne",
                "de chêne",
                "d'if",
                "de bois de fer",
                "de bois-de-lune",
                "de bois spectral",
                "de bois-monde",
            ],
            Material::Leather => [
                "de peau crue",
                "de cuir",
                "de cuir tanné",
                "de cuir clouté",
                "de cuir de wyverne",
                "de cuir runique",
                "de cuir d'ombre",
                "de cuir draconique",
            ],
            Material::Cloth => [
                "de toile de sac",
                "de lin",
                "de laine",
                "de soie",
                "de soie d'araignée",
                "d'étoffe runique",
                "d'étoffe astrale",
                "d'étoffe céleste",
            ],
            Material::Jewel => [
                "de cuivre",
                "d'argent",
                "d'or",
                "de platine",
                "d'opale",
                "de saphir",
                "de rubis",
                "d'étoile",
            ],
            Material::Alchemy => [
                "diluée",
                "mineure",
                "simple",
                "forte",
                "supérieure",
                "majeure",
                "suprême",
                "légendaire",
            ],
        };
        names[t]
    }

    /// Couleur du matériau à ce tier (r, g, b).
    pub fn tier_color(self, tier: u8) -> (f32, f32, f32) {
        let t = tier.clamp(1, 8) as usize - 1;
        let c: [(f32, f32, f32); 8] = match self {
            Material::Metal => [
                (0.45, 0.33, 0.25),
                (0.52, 0.53, 0.55),
                (0.72, 0.74, 0.78),
                (0.6, 0.66, 0.75),
                (0.25, 0.4, 0.75),
                (0.75, 0.9, 0.95),
                (0.35, 0.75, 0.45),
                (0.95, 0.85, 0.45),
            ],
            Material::Wood => [
                (0.4, 0.32, 0.22),
                (0.62, 0.5, 0.35),
                (0.5, 0.36, 0.2),
                (0.42, 0.26, 0.16),
                (0.3, 0.27, 0.25),
                (0.7, 0.75, 0.85),
                (0.55, 0.75, 0.7),
                (0.45, 0.7, 0.35),
            ],
            Material::Leather => [
                (0.5, 0.4, 0.3),
                (0.55, 0.35, 0.2),
                (0.45, 0.28, 0.15),
                (0.35, 0.25, 0.18),
                (0.3, 0.45, 0.3),
                (0.35, 0.3, 0.5),
                (0.18, 0.16, 0.22),
                (0.6, 0.15, 0.12),
            ],
            Material::Cloth => [
                (0.6, 0.55, 0.42),
                (0.8, 0.78, 0.7),
                (0.55, 0.45, 0.6),
                (0.7, 0.25, 0.3),
                (0.85, 0.85, 0.9),
                (0.25, 0.35, 0.7),
                (0.3, 0.15, 0.45),
                (0.95, 0.9, 0.6),
            ],
            Material::Jewel => [
                (0.75, 0.45, 0.25),
                (0.8, 0.8, 0.85),
                (0.95, 0.8, 0.3),
                (0.85, 0.88, 0.9),
                (0.7, 0.9, 0.9),
                (0.2, 0.35, 0.9),
                (0.85, 0.1, 0.2),
                (1.0, 1.0, 0.8),
            ],
            Material::Alchemy => [
                (0.6, 0.3, 0.3),
                (0.7, 0.2, 0.2),
                (0.8, 0.15, 0.15),
                (0.85, 0.1, 0.25),
                (0.75, 0.1, 0.45),
                (0.6, 0.1, 0.7),
                (0.4, 0.2, 0.9),
                (0.9, 0.85, 0.3),
            ],
        };
        c[t]
    }
}

/// Multiplicateur de puissance selon le tier. T8 ≈ 7 × T1.
pub fn tier_mult(tier: u8) -> f32 {
    let t = tier.clamp(1, 8) as f32 - 1.0;
    1.0 + 0.5 * t + 0.05 * t * t
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord, Default)]
pub enum Quality {
    #[default]
    Ordinary,
    Good,
    Remarkable,
    Excellent,
    Masterpiece,
}

impl Quality {
    pub const ALL: [Quality; 5] =
        [Quality::Ordinary, Quality::Good, Quality::Remarkable, Quality::Excellent, Quality::Masterpiece];

    pub fn label(self) -> &'static str {
        match self {
            Quality::Ordinary => "Ordinaire",
            Quality::Good => "Bonne facture",
            Quality::Remarkable => "Remarquable",
            Quality::Excellent => "Excellente",
            Quality::Masterpiece => "Chef-d'œuvre",
        }
    }

    pub fn mult(self) -> f32 {
        match self {
            Quality::Ordinary => 1.0,
            Quality::Good => 1.08,
            Quality::Remarkable => 1.17,
            Quality::Excellent => 1.28,
            Quality::Masterpiece => 1.42,
        }
    }

    pub fn affix_count(self) -> usize {
        match self {
            Quality::Ordinary => 0,
            Quality::Good | Quality::Remarkable => 1,
            Quality::Excellent => 2,
            Quality::Masterpiece => 3,
        }
    }

    /// Couleur d'interface.
    pub fn color(self) -> (f32, f32, f32) {
        match self {
            Quality::Ordinary => (0.85, 0.85, 0.85),
            Quality::Good => (0.45, 0.85, 0.4),
            Quality::Remarkable => (0.35, 0.6, 1.0),
            Quality::Excellent => (0.75, 0.4, 0.95),
            Quality::Masterpiece => (1.0, 0.65, 0.2),
        }
    }
}

/// Base d'objet (contenu).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemBase {
    pub id: String,
    pub name: String,
    pub slot: EquipSlot,
    /// Voie de maîtrise de l'objet (vide pour bijoux/consommables).
    #[serde(default)]
    pub line: String,
    pub material: Material,
    #[serde(default)]
    pub two_handed: bool,
    /// Attaque de base fournie (armes).
    #[serde(default)]
    pub basic: String,
    /// Skill utilisable (potions).
    #[serde(default)]
    pub use_skill: String,
    /// Stats au tier 1, qualité ordinaire.
    pub mods: Vec<Modifier>,
    /// Forme 3D (le jeu sait la dessiner).
    pub model: String,
    #[serde(default)]
    pub description: String,
    /// Tier minimal / maximal où cette base existe.
    #[serde(default = "one_u8")]
    pub min_tier: u8,
    #[serde(default = "eight_u8")]
    pub max_tier: u8,
    /// Poids de tirage dans le butin.
    #[serde(default = "one_f32")]
    pub weight: f32,
}

fn one_u8() -> u8 {
    1
}
fn eight_u8() -> u8 {
    8
}
fn one_f32() -> f32 {
    1.0
}

/// Affixe aléatoire (contenu).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AffixDef {
    pub id: String,
    /// Complément de nom : « de l'Ours ».
    pub name: String,
    /// Emplacements autorisés (vide = tous sauf consommables).
    #[serde(default)]
    pub slots: Vec<EquipSlot>,
    pub stat: Stat,
    pub kind: ModKind,
    /// Valeur au tier 1 (min, max).
    pub min: f32,
    pub max: f32,
    /// Si vrai, la valeur grandit avec le tier.
    #[serde(default = "yes")]
    pub scales: bool,
    #[serde(default = "one_f32")]
    pub weight: f32,
}

fn yes() -> bool {
    true
}

impl AffixDef {
    pub fn allows(&self, slot: EquipSlot) -> bool {
        if matches!(slot, EquipSlot::Potion | EquipSlot::Food) {
            return false;
        }
        self.slots.is_empty() || self.slots.contains(&slot)
    }
}

/// Objet unique (contenu).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniqueDef {
    pub id: String,
    pub name: String,
    pub base: String,
    pub tier: u8,
    /// Stats en plus de la base.
    #[serde(default)]
    pub mods: Vec<Modifier>,
    /// Effets spéciaux, actifs quand l'objet est porté.
    pub effects: Vec<PassiveEffect>,
    pub lore: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AffixRoll {
    pub affix: String,
    pub value: f32,
}

/// Un objet réel, dans un inventaire.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub uid: u64,
    pub base: String,
    pub tier: u8,
    #[serde(default)]
    pub quality: Quality,
    #[serde(default)]
    pub affixes: Vec<AffixRoll>,
    #[serde(default)]
    pub unique: Option<String>,
    /// Nombre d'utilisations (consommables).
    #[serde(default = "one_u32")]
    pub count: u32,
}

fn one_u32() -> u32 {
    1
}

impl Item {
    pub fn new(uid: u64, base: &str, tier: u8) -> Self {
        Self {
            uid,
            base: base.to_string(),
            tier: tier.clamp(1, 8),
            quality: Quality::Ordinary,
            affixes: vec![],
            unique: None,
            count: 1,
        }
    }

    /// Puissance d'objet (affichée, sert au tri) : tier et qualité.
    pub fn power(&self) -> u32 {
        (100.0 * tier_mult(self.tier) * self.quality.mult()) as u32
    }
}

/// Valeur d'un affixe au tier donné.
pub fn affix_value_at_tier(def: &AffixDef, base_value: f32, tier: u8) -> f32 {
    if def.scales && def.kind == ModKind::Flat {
        base_value * tier_mult(tier)
    } else if def.scales {
        // Les % grandissent plus doucement.
        base_value * (1.0 + (tier_mult(tier) - 1.0) * 0.35)
    } else {
        base_value
    }
}

/// Applique tier × qualité × bonus de maîtrise à un modificateur de base.
pub fn scale_mod(m: &Modifier, tier: u8, quality: Quality, power_bonus: f32) -> Modifier {
    let scale_flat = tier_mult(tier) * quality.mult() * (1.0 + power_bonus);
    let v = match (m.kind, m.stat.is_ratio()) {
        // Les ratios (crit, esquive...) ne s'envolent pas avec le tier.
        (ModKind::Flat, true) | (ModKind::Percent, _) => {
            m.value * (1.0 + (scale_flat - 1.0) * 0.25)
        }
        (ModKind::Flat, false) => {
            // La vitesse ne doit pas exploser non plus.
            if m.stat == Stat::MoveSpeed { m.value } else { m.value * scale_flat }
        }
    };
    Modifier { stat: m.stat, kind: m.kind, value: v }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiers_croissants() {
        for t in 1..8u8 {
            assert!(tier_mult(t + 1) > tier_mult(t));
        }
        assert!((tier_mult(1) - 1.0).abs() < 1e-6);
        assert!(tier_mult(8) > 6.0);
    }

    #[test]
    fn noms_de_tiers() {
        assert_eq!(Material::Metal.tier_name(3), "d'acier");
        assert_eq!(Material::Metal.tier_name(0), "de fer rouillé");
        assert_eq!(Material::Metal.tier_name(99), "d'astralite");
    }

    #[test]
    fn mise_a_echelle() {
        let m = Modifier::flat(Stat::MaxHp, 10.0);
        let s = scale_mod(&m, 8, Quality::Masterpiece, 1.0);
        assert!(s.value > 150.0);
        let speed = Modifier::flat(Stat::MoveSpeed, 0.3);
        assert_eq!(scale_mod(&speed, 8, Quality::Masterpiece, 1.0).value, 0.3);
    }
}
