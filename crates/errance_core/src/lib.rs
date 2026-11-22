//! # Errance — le cœur des règles
//!
//! Ce crate ne connaît ni l'écran ni le tactile : il décrit le monde, les
//! objets, les skills et simule les combats. Le jeu (Bevy) ne fait que
//! l'afficher et lui transmettre les gestes du joueur.

pub mod item;
pub mod mastery;
pub mod math;
pub mod passive;
pub mod rng;
pub mod rules;
pub mod skill;
pub mod stats;
