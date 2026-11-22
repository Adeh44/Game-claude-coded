//! Skills actifs, effets, états et mots-clés.
//!
//! Tout est décrit en données (fichiers `.ron`) : un skill = un ciblage,
//! une façon d'arriver (instantané, projectile, zone retardée) et une liste
//! d'effets appliqués à chaque cible touchée.

use crate::stats::Stat;
use serde::{Deserialize, Serialize};

/// Mots-clés : servent aux synergies des talents et au badge de rôle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Tag {
    Physical,
    Fire,
    Frost,
    Arcane,
    Holy,
    Nature,
    Shadow,
    Melee,
    Ranged,
    Spell,
    Heal,
    Shield,
    Control,
    Movement,
    Aoe,
    Bleed,
    Poison,
    Burn,
    Taunt,
    Defense,
    Buff,
    Debuff,
    Execute,
    Basic,
}

impl Tag {
    pub fn label(self) -> &'static str {
        match self {
            Tag::Physical => "Physique",
            Tag::Fire => "Feu",
            Tag::Frost => "Givre",
            Tag::Arcane => "Arcane",
            Tag::Holy => "Sacré",
            Tag::Nature => "Nature",
            Tag::Shadow => "Ombre",
            Tag::Melee => "Mêlée",
            Tag::Ranged => "Distance",
            Tag::Spell => "Sort",
            Tag::Heal => "Soin",
            Tag::Shield => "Bouclier",
            Tag::Control => "Contrôle",
            Tag::Movement => "Mouvement",
            Tag::Aoe => "Zone",
            Tag::Bleed => "Saignement",
            Tag::Poison => "Poison",
            Tag::Burn => "Brûlure",
            Tag::Taunt => "Provocation",
            Tag::Defense => "Défense",
            Tag::Buff => "Renfort",
            Tag::Debuff => "Affaiblissement",
            Tag::Execute => "Exécution",
            Tag::Basic => "Attaque de base",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DamageKind {
    Physical,
    Fire,
    Frost,
    Arcane,
    Holy,
    Nature,
    Shadow,
    /// Ignore armure et résistance.
    True,
}

impl DamageKind {
    pub fn is_magic(self) -> bool {
        !matches!(self, DamageKind::Physical | DamageKind::True)
    }

    /// Stat de puissance qui renforce ce type de dégâts.
    pub fn power_stat(self) -> Stat {
        if self == DamageKind::Physical { Stat::PhysicalPower } else { Stat::MagicPower }
    }

    pub fn tag(self) -> Option<Tag> {
        Some(match self {
            DamageKind::Physical => Tag::Physical,
            DamageKind::Fire => Tag::Fire,
            DamageKind::Frost => Tag::Frost,
            DamageKind::Arcane => Tag::Arcane,
            DamageKind::Holy => Tag::Holy,
            DamageKind::Nature => Tag::Nature,
            DamageKind::Shadow => Tag::Shadow,
            DamageKind::True => return None,
        })
    }

    pub fn label(self) -> &'static str {
        match self {
            DamageKind::Physical => "physiques",
            DamageKind::Fire => "de feu",
            DamageKind::Frost => "de givre",
            DamageKind::Arcane => "arcaniques",
            DamageKind::Holy => "sacrés",
            DamageKind::Nature => "de nature",
            DamageKind::Shadow => "d'ombre",
            DamageKind::True => "purs",
        }
    }
}

/// États (bonus et malus) temporaires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum StatusKind {
    /// Dégâts de feu par seconde (magnitude).
    Burn,
    /// Dégâts de nature par seconde, cumulable.
    Poison,
    /// Dégâts physiques par seconde, cumulable.
    Bleed,
    /// Ralenti (magnitude = fraction), à 5 cumuls → Gelé.
    Chill,
    /// Gelé : ne peut ni bouger ni agir.
    Frozen,
    /// Étourdi : ne peut ni bouger ni agir.
    Stun,
    /// Enraciné : ne peut pas bouger.
    Root,
    /// Ralenti (magnitude = fraction).
    Slow,
    /// Réduit au silence : pas de skills (attaque de base possible).
    Silence,
    /// Effrayé : fuit la source.
    Fear,
    /// Provoqué : doit attaquer la source.
    Taunted,
    /// Inflige moins de dégâts (magnitude = fraction).
    Weaken,
    /// Subit plus de dégâts (magnitude = fraction).
    Vulnerable,
    /// Subit moins de dégâts (magnitude = fraction).
    Fortify,
    /// Inflige plus de dégâts (magnitude = fraction).
    Empower,
    /// Plus rapide (déplacement et attaque, magnitude = fraction).
    Haste,
    /// Soin par seconde (magnitude).
    Regen,
    /// Renvoie une part des dégâts de mêlée subis (magnitude = fraction).
    Thorns,
    /// Insaisissable : esquive accrue (magnitude = fraction).
    Evasion,
}

impl StatusKind {
    pub fn label(self) -> &'static str {
        match self {
            StatusKind::Burn => "Brûlure",
            StatusKind::Poison => "Poison",
            StatusKind::Bleed => "Saignement",
            StatusKind::Chill => "Transi",
            StatusKind::Frozen => "Gelé",
            StatusKind::Stun => "Étourdi",
            StatusKind::Root => "Enraciné",
            StatusKind::Slow => "Ralenti",
            StatusKind::Silence => "Silence",
            StatusKind::Fear => "Effrayé",
            StatusKind::Taunted => "Provoqué",
            StatusKind::Weaken => "Affaibli",
            StatusKind::Vulnerable => "Vulnérable",
            StatusKind::Fortify => "Fortifié",
            StatusKind::Empower => "Galvanisé",
            StatusKind::Haste => "Hâte",
            StatusKind::Regen => "Régénération",
            StatusKind::Thorns => "Épines",
            StatusKind::Evasion => "Évasion",
        }
    }

    /// Malus (ce que la Ténacité raccourcit et ce que Purifier retire).
    pub fn is_debuff(self) -> bool {
        matches!(
            self,
            StatusKind::Burn
                | StatusKind::Poison
                | StatusKind::Bleed
                | StatusKind::Chill
                | StatusKind::Frozen
                | StatusKind::Stun
                | StatusKind::Root
                | StatusKind::Slow
                | StatusKind::Silence
                | StatusKind::Fear
                | StatusKind::Taunted
                | StatusKind::Weaken
                | StatusKind::Vulnerable
        )
    }

    /// Contrôle dur : empêche d'agir.
    pub fn is_hard_cc(self) -> bool {
        matches!(self, StatusKind::Frozen | StatusKind::Stun | StatusKind::Fear)
    }

    pub fn stacks(self) -> bool {
        matches!(self, StatusKind::Poison | StatusKind::Bleed | StatusKind::Chill)
    }

    pub fn max_stacks(self) -> u32 {
        match self {
            StatusKind::Chill => 5,
            StatusKind::Poison | StatusKind::Bleed => 10,
            _ => 1,
        }
    }

    pub fn tag(self) -> Tag {
        match self {
            StatusKind::Burn => Tag::Burn,
            StatusKind::Poison => Tag::Poison,
            StatusKind::Bleed => Tag::Bleed,
            StatusKind::Taunted => Tag::Taunt,
            StatusKind::Fortify | StatusKind::Thorns | StatusKind::Evasion => Tag::Defense,
            StatusKind::Empower | StatusKind::Haste | StatusKind::Regen => Tag::Buff,
            StatusKind::Weaken | StatusKind::Vulnerable => Tag::Debuff,
            _ => Tag::Control,
        }
    }
}

/// Qui un skill vise.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Targeting {
    /// Soi-même.
    SelfOnly,
    /// Un ennemi à portée.
    Enemy { range: f32 },
    /// Un allié (ou soi) à portée.
    Ally { range: f32 },
    /// Zone centrée sur la cible ennemie (ou devant soi).
    AreaAtTarget { range: f32, radius: f32 },
    /// Zone centrée sur un allié blessé (ou soi).
    AreaAtAlly { range: f32, radius: f32 },
    /// Zone autour de soi (touche les ennemis).
    AreaAroundSelf { radius: f32 },
    /// Zone autour de soi (touche les alliés, soi compris).
    AlliesAroundSelf { radius: f32 },
    /// Cône devant soi.
    Cone { range: f32, angle_deg: f32 },
    /// Ligne devant soi.
    Line { range: f32, width: f32 },
    /// Bond vers la cible / devant soi, puis effets autour du point d'arrivée.
    Dash { distance: f32, radius: f32 },
    /// Bond en arrière (fuite), effets autour du point de départ.
    Leap { distance: f32, radius: f32 },
}

impl Targeting {
    /// Portée utile pour l'IA (à quelle distance de la cible se placer).
    pub fn reach(&self) -> f32 {
        match *self {
            Targeting::SelfOnly => 0.0,
            Targeting::Enemy { range } | Targeting::Ally { range } => range,
            Targeting::AreaAtTarget { range, .. } | Targeting::AreaAtAlly { range, .. } => range,
            Targeting::AreaAroundSelf { radius } | Targeting::AlliesAroundSelf { radius } => radius * 0.8,
            Targeting::Cone { range, .. } | Targeting::Line { range, .. } => range * 0.9,
            Targeting::Dash { distance, .. } => distance,
            Targeting::Leap { radius, .. } => radius,
        }
    }

    pub fn is_friendly(&self) -> bool {
        matches!(
            self,
            Targeting::SelfOnly
                | Targeting::Ally { .. }
                | Targeting::AreaAtAlly { .. }
                | Targeting::AlliesAroundSelf { .. }
        )
    }

    pub fn needs_enemy(&self) -> bool {
        matches!(self, Targeting::Enemy { .. } | Targeting::AreaAtTarget { .. })
    }
}

/// Comment le skill arrive sur sa cible.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum Delivery {
    #[default]
    Instant,
    /// Projectile à tête chercheuse.
    Projectile { speed: f32 },
    /// Zone dessinée au sol qui se déclenche après `delay` secondes :
    /// on peut l'esquiver en sortant.
    Delayed { delay: f32 },
}

/// Ce qu'un skill fait à chaque cible touchée.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Effect {
    /// base + scaling × puissance (physique ou magique selon le type).
    Damage {
        kind: DamageKind,
        base: f32,
        scaling: f32,
        #[serde(default)]
        bonus_vs: Option<(StatusKind, f32)>,
        /// Bonus si la cible est sous `seuil` PV (exécution).
        #[serde(default)]
        execute: Option<(f32, f32)>,
    },
    Heal { base: f32, scaling: f32 },
    Shield { base: f32, scaling: f32, duration: f32 },
    Status {
        kind: StatusKind,
        duration: f32,
        #[serde(default)]
        magnitude: f32,
        #[serde(default = "one")]
        chance: f32,
        #[serde(default = "one_u32")]
        stacks: u32,
    },
    /// Applique un état au lanceur (pas à la cible).
    SelfStatus {
        kind: StatusKind,
        duration: f32,
        #[serde(default)]
        magnitude: f32,
    },
    /// Bouclier sur le lanceur.
    SelfShield { base: f32, scaling: f32, duration: f32 },
    /// Soin sur le lanceur.
    SelfHeal { base: f32, scaling: f32 },
    /// Menace pure.
    Threat { amount: f32 },
    /// Force la cible à attaquer le lanceur.
    Taunt { duration: f32 },
    Knockback { distance: f32 },
    Pull { distance: f32 },
    RestoreEnergy { amount: f32 },
    /// Retire les malus de la cible.
    Cleanse,
    /// Fait exploser les cumuls d'un état : dégâts × cumuls, puis les retire.
    Detonate { status: StatusKind, per_stack: f32, kind: DamageKind },
    /// Réduit la recharge des autres skills du lanceur.
    ReduceCooldowns { seconds: f32 },
}

fn one() -> f32 {
    1.0
}
fn one_u32() -> u32 {
    1
}

/// Emplacement de skill sur la barre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum SkillSlot {
    /// Attaque de base de l'arme (automatique).
    Basic,
    /// Trois emplacements d'arme (Q, W, E sur Albion).
    Weapon1,
    Weapon2,
    Weapon3,
    Head,
    Chest,
    Feet,
    /// Consommable (potion).
    Potion,
}

impl SkillSlot {
    pub const ACTIVE: [SkillSlot; 6] = [
        SkillSlot::Weapon1,
        SkillSlot::Weapon2,
        SkillSlot::Weapon3,
        SkillSlot::Head,
        SkillSlot::Chest,
        SkillSlot::Feet,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SkillSlot::Basic => "Attaque",
            SkillSlot::Weapon1 => "Arme I",
            SkillSlot::Weapon2 => "Arme II",
            SkillSlot::Weapon3 => "Arme III",
            SkillSlot::Head => "Tête",
            SkillSlot::Chest => "Torse",
            SkillSlot::Feet => "Pieds",
            SkillSlot::Potion => "Potion",
        }
    }

    pub fn bar_index(self) -> usize {
        match self {
            SkillSlot::Basic => 0,
            SkillSlot::Weapon1 => 1,
            SkillSlot::Weapon2 => 2,
            SkillSlot::Weapon3 => 3,
            SkillSlot::Head => 4,
            SkillSlot::Chest => 5,
            SkillSlot::Feet => 6,
            SkillSlot::Potion => 7,
        }
    }
}

pub const BAR_SIZE: usize = 8;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillDef {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Voie (ligne de maîtrise) qui fournit ce skill.
    pub line: String,
    pub slot: SkillSlot,
    /// Niveau de maîtrise de la voie requis.
    #[serde(default)]
    pub mastery: u32,
    #[serde(default)]
    pub cost: f32,
    #[serde(default)]
    pub cooldown: f32,
    #[serde(default)]
    pub cast_time: f32,
    pub targeting: Targeting,
    #[serde(default)]
    pub delivery: Delivery,
    pub effects: Vec<Effect>,
    #[serde(default)]
    pub tags: Vec<Tag>,
    /// Symbole court affiché sur le bouton (1-2 caractères ou mot-clé d'icône).
    #[serde(default)]
    pub icon: String,
}

impl SkillDef {
    pub fn has_tag(&self, t: Tag) -> bool {
        self.tags.contains(&t)
    }

    /// Tous les mots-clés, y compris ceux déduits des effets.
    pub fn all_tags(&self) -> Vec<Tag> {
        let mut out = self.tags.clone();
        for e in &self.effects {
            let t = match e {
                Effect::Damage { kind, .. } => kind.tag(),
                Effect::Heal { .. } | Effect::SelfHeal { .. } => Some(Tag::Heal),
                Effect::Shield { .. } | Effect::SelfShield { .. } => Some(Tag::Shield),
                Effect::Status { kind, .. } | Effect::SelfStatus { kind, .. } => Some(kind.tag()),
                Effect::Taunt { .. } => Some(Tag::Taunt),
                Effect::Knockback { .. } | Effect::Pull { .. } => Some(Tag::Control),
                Effect::Cleanse => Some(Tag::Heal),
                _ => None,
            };
            if let Some(t) = t
                && !out.contains(&t) {
                    out.push(t);
                }
        }
        match self.targeting {
            Targeting::AreaAtTarget { .. }
            | Targeting::AreaAroundSelf { .. }
            | Targeting::Cone { .. }
            | Targeting::Line { .. }
            | Targeting::AlliesAroundSelf { .. }
            | Targeting::AreaAtAlly { .. } => {
                if !out.contains(&Tag::Aoe) {
                    out.push(Tag::Aoe)
                }
            }
            Targeting::Dash { .. } | Targeting::Leap { .. }
                if !out.contains(&Tag::Movement) => {
                    out.push(Tag::Movement)
                }
            _ => {}
        }
        out
    }

    /// Texte des effets pour l'interface (valeurs de base).
    pub fn effect_lines(&self) -> Vec<String> {
        self.effects.iter().map(describe_effect).collect()
    }
}

pub fn describe_effect(e: &Effect) -> String {
    use crate::stats::fmt_num;
    match e {
        Effect::Damage { kind, base, scaling, bonus_vs, execute } => {
            let mut s = format!("{} dégâts {} (+{} % puiss.)", fmt_num(*base), kind.label(), (scaling * 100.0) as i32);
            if let Some((st, m)) = bonus_vs {
                s += &format!(", ×{} si {}", fmt_num(*m), st.label());
            }
            if let Some((th, m)) = execute {
                s += &format!(", ×{} sous {} % PV", fmt_num(*m), (th * 100.0) as i32);
            }
            s
        }
        Effect::Heal { base, scaling } => format!("Soigne {} (+{} % soin)", fmt_num(*base), (scaling * 100.0) as i32),
        Effect::Shield { base, scaling, duration } => {
            format!("Bouclier {} (+{} %) pendant {} s", fmt_num(*base), (scaling * 100.0) as i32, fmt_num(*duration))
        }
        Effect::Status { kind, duration, magnitude, chance, stacks } => {
            let mut s = format!("{} {} s", kind.label(), fmt_num(*duration));
            if *magnitude > 0.0 {
                if *magnitude < 1.0 {
                    s += &format!(" ({} %)", (magnitude * 100.0) as i32);
                } else {
                    s += &format!(" ({}/s)", fmt_num(*magnitude));
                }
            }
            if *stacks > 1 {
                s += &format!(" ×{}", stacks);
            }
            if *chance < 1.0 {
                s += &format!(" ({} % de chance)", (chance * 100.0) as i32);
            }
            s
        }
        Effect::SelfStatus { kind, duration, magnitude } => {
            if *magnitude > 0.0 && *magnitude < 1.0 {
                format!("Vous : {} {} s ({} %)", kind.label(), fmt_num(*duration), (magnitude * 100.0) as i32)
            } else {
                format!("Vous : {} {} s", kind.label(), fmt_num(*duration))
            }
        }
        Effect::SelfShield { base, duration, .. } => format!("Vous : bouclier {} pendant {} s", fmt_num(*base), fmt_num(*duration)),
        Effect::SelfHeal { base, .. } => format!("Vous soigne de {}", fmt_num(*base)),
        Effect::Threat { amount } => format!("Génère {} de menace", fmt_num(*amount)),
        Effect::Taunt { duration } => format!("Provoque pendant {} s", fmt_num(*duration)),
        Effect::Knockback { distance } => format!("Repousse de {} m", fmt_num(*distance)),
        Effect::Pull { distance } => format!("Attire de {} m", fmt_num(*distance)),
        Effect::RestoreEnergy { amount } => format!("Rend {} d'énergie", fmt_num(*amount)),
        Effect::Cleanse => "Retire les malus".to_string(),
        Effect::Detonate { status, per_stack, kind } => {
            format!("Fait éclater {} : {} dégâts {} par cumul", status.label(), fmt_num(*per_stack), kind.label())
        }
        Effect::ReduceCooldowns { seconds } => format!("Réduit vos recharges de {} s", fmt_num(*seconds)),
    }
}
