//! Générateur aléatoire maison, à graine.
//!
//! Même graine = même suite de nombres : un combat ou un butin se rejoue à
//! l'identique, ce qui rend les tests et l'équilibrage reproductibles.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed ^ 0x9E37_79B9_7F4A_7C15 }
    }

    /// SplitMix64 : rapide, bonne qualité pour un jeu.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Nombre dans [0, 1).
    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Nombre dans [lo, hi).
    pub fn range_f32(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.f32()
    }

    /// Entier dans [lo, hi] (bornes incluses).
    pub fn range_u32(&mut self, lo: u32, hi: u32) -> u32 {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u64() % (hi - lo + 1) as u64) as u32
    }

    /// Indice dans [0, n).
    pub fn index(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        (self.next_u64() % n as u64) as usize
    }

    /// Vrai avec la probabilité `p` (0..1).
    pub fn chance(&mut self, p: f32) -> bool {
        self.f32() < p
    }

    /// Choisit un élément au hasard.
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        if items.is_empty() {
            None
        } else {
            Some(&items[self.index(items.len())])
        }
    }

    /// Choisit un indice selon des poids.
    pub fn weighted(&mut self, weights: &[f32]) -> Option<usize> {
        let total: f32 = weights.iter().filter(|w| **w > 0.0).sum();
        if total <= 0.0 {
            return None;
        }
        let mut roll = self.f32() * total;
        for (i, w) in weights.iter().enumerate() {
            if *w <= 0.0 {
                continue;
            }
            if roll < *w {
                return Some(i);
            }
            roll -= w;
        }
        weights.iter().rposition(|w| *w > 0.0)
    }

    /// Dérive un nouveau générateur indépendant (pour un sous-système).
    pub fn fork(&mut self) -> Rng {
        Rng::new(self.next_u64())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meme_graine_meme_suite() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn bornes_respectees() {
        let mut r = Rng::new(7);
        for _ in 0..10_000 {
            let x = r.f32();
            assert!((0.0..1.0).contains(&x));
            let n = r.range_u32(3, 5);
            assert!((3..=5).contains(&n));
        }
    }

    #[test]
    fn poids_zero_jamais_choisi() {
        let mut r = Rng::new(1);
        for _ in 0..1000 {
            assert_ne!(r.weighted(&[1.0, 0.0, 2.0]), Some(1));
        }
    }
}
