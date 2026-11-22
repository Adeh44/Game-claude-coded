//! Apparence : ce que le jeu doit dessiner. Le cœur ne dessine rien, il
//! décrit (« un humain, casque à cornes d'acier, épée de cobalt »).

use serde::{Deserialize, Serialize};

pub type Rgb = (f32, f32, f32);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Body {
    /// Humain assemblé de pièces.
    Humanoid { skin: Rgb, hair: Rgb, hair_style: u8 },
    /// Créature dessinée d'un bloc (rat, loup, ours...).
    Creature { model: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LookPart {
    /// Où : "head", "chest", "feet", "main", "off", "cape".
    pub at: String,
    /// Forme (le jeu sait dessiner chaque modèle, sinon forme par défaut).
    pub model: String,
    pub color: Rgb,
    pub tier: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Look {
    pub body: Body,
    pub scale: f32,
    /// Couleur principale (créatures) ou teinte des vêtements de base.
    pub color: Rgb,
    pub parts: Vec<LookPart>,
}

impl Look {
    pub fn creature(model: &str, color: Rgb, scale: f32) -> Self {
        Self { body: Body::Creature { model: model.to_string() }, scale, color, parts: vec![] }
    }

    pub fn humanoid(skin: Rgb, hair: Rgb, hair_style: u8) -> Self {
        Self {
            body: Body::Humanoid { skin, hair, hair_style },
            scale: 1.0,
            color: (0.55, 0.5, 0.42),
            parts: vec![],
        }
    }

    pub fn part(&self, at: &str) -> Option<&LookPart> {
        self.parts.iter().find(|p| p.at == at)
    }
}

impl Default for Look {
    fn default() -> Self {
        Look::humanoid((0.85, 0.66, 0.5), (0.3, 0.2, 0.12), 0)
    }
}
