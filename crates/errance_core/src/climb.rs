//! Escalade : les passages de paroi de Givrecol.
//!
//! Une voie = des prises sur un mur, du pied (prise 0) au sommet (dernière).
//! Le grimpeur va de prise en prise (à portée de bras) pendant que deux
//! jauges baissent :
//! - les Bras (endurance) : chaque prise use plus ou moins, un bac permet
//!   de « secouer » les bras, un relais permet de récupérer ;
//! - la Chaleur : le vent glace les doigts ; mains gourdes = bras qui
//!   s'usent plus vite. Le relais réchauffe.
//!
//! Bras à zéro, ou glissade sur une prise verglacée : c'est la chute.
//! Un piton planté (ou un relais) arrête la chute ; sans protection, on
//! tombe jusqu'au pied de la voie et ça fait très mal.
//!
//! L'équipement compte (tu es ce que tu portes) : la plaque fatigue les bras,
//! le tissu laisse passer le froid, le cuir est l'habit du grimpeur.
//! La Voie Escalade (métier) rend tout plus facile et donne plus de pitons.

use crate::character::Character;
use crate::content::Content;
use crate::item::{EquipSlot, tier_mult};
use crate::rng::Rng;
use serde::{Deserialize, Serialize};

pub const LINE_CLIMB: &str = "escalade";

/// Portée d'un mouvement normal (m, centre à centre).
pub const REACH: f32 = 1.7;
/// Au-delà, c'est un « jeté » : plus de bras, mais ça passe.
pub const REACH_EASY: f32 = 1.25;
/// Durée d'un mouvement (s).
pub const MOVE_TIME: f32 = 0.45;
/// Durée pour planter un piton (s).
pub const PITON_TIME: f32 = 1.4;
/// Largeur de la paroi (m).
pub const WALL_WIDTH: f32 = 6.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HoldKind {
    /// Grosse prise franche : on peut y secouer les bras.
    Bac,
    /// Petite arête pour le bout des doigts.
    Reglette,
    /// Prise ronde et fuyante.
    Plat,
    /// Prise couverte de glace : peut lâcher.
    Verglas,
    /// Vire où l'on tient debout : repos, chaleur, protection.
    Relais,
}

impl HoldKind {
    pub fn name(self) -> &'static str {
        match self {
            HoldKind::Bac => "Bac",
            HoldKind::Reglette => "Réglette",
            HoldKind::Plat => "Plat",
            HoldKind::Verglas => "Verglas",
            HoldKind::Relais => "Relais",
        }
    }

    /// Bras perdus par seconde accroché ici (négatif = on récupère).
    pub fn drain(self) -> f32 {
        match self {
            HoldKind::Bac => -3.0,
            HoldKind::Reglette => 7.0,
            HoldKind::Plat => 10.0,
            HoldKind::Verglas => 12.0,
            HoldKind::Relais => -22.0,
        }
    }

    /// On y est à l'abri : le froid y recule.
    pub fn sheltered(self) -> bool {
        self == HoldKind::Relais
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Hold {
    /// 0 = bord gauche de la paroi.
    pub x: f32,
    /// Hauteur (m) depuis le pied.
    pub y: f32,
    pub kind: HoldKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Route {
    pub name: String,
    pub tier: u8,
    pub height: f32,
    pub holds: Vec<Hold>,
}

impl Route {
    pub fn top(&self) -> usize {
        self.holds.len() - 1
    }

    /// Les prises atteignables depuis `from` (sans elle-même).
    pub fn reachable(&self, from: usize) -> Vec<usize> {
        let a = self.holds[from];
        (0..self.holds.len()).filter(|&k| k != from && dist(a, self.holds[k]) <= REACH).collect()
    }

    /// Nombre de mouvements minimum jusqu'au sommet (u32::MAX = coupé).
    pub fn hops_to_top(&self) -> Vec<u32> {
        let mut d = vec![u32::MAX; self.holds.len()];
        let top = self.top();
        d[top] = 0;
        let mut q = std::collections::VecDeque::from([top]);
        while let Some(i) = q.pop_front() {
            for k in self.reachable(i) {
                if d[k] == u32::MAX {
                    d[k] = d[i] + 1;
                    q.push_back(k);
                }
            }
        }
        d
    }

    /// Il existe un chemin du pied au sommet (vérifié par les tests).
    pub fn passable(&self) -> bool {
        let mut seen = vec![false; self.holds.len()];
        let mut stack = vec![0];
        seen[0] = true;
        while let Some(i) = stack.pop() {
            if i == self.top() {
                return true;
            }
            for k in self.reachable(i) {
                if !seen[k] {
                    seen[k] = true;
                    stack.push(k);
                }
            }
        }
        false
    }
}

pub fn dist(a: Hold, b: Hold) -> f32 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

/// Dessine une voie. `ice` = part de prises verglacées (0..1).
pub fn generate_route(name: &str, tier: u8, height: f32, ice: f32, rng: &mut Rng) -> Route {
    let tier = tier.clamp(1, 8);
    let mid = WALL_WIDTH / 2.0;
    let mut holds = vec![Hold { x: mid, y: 0.0, kind: HoldKind::Relais }];
    // Plus le tier est haut, moins il y a de bacs.
    let bac = (0.34 - 0.03 * tier as f32).max(0.1);
    let plat = 0.12 + 0.02 * tier as f32;
    let mut y = 0.0;
    // Rangée précédente : seulement les prises reliées au pied.
    let mut prev_row: Vec<usize> = vec![0];
    let mut relais_done = height < 9.0;
    loop {
        y += rng.range_f32(0.75, 1.15);
        if y >= height - 0.6 {
            break;
        }
        let row_start = holds.len();
        // Un relais à mi-hauteur sur les grandes voies.
        if !relais_done && y >= height * 0.5 {
            relais_done = true;
            let anchor = holds[prev_row[rng.index(prev_row.len())]];
            let x = (anchor.x + rng.range_f32(-0.6, 0.6)).clamp(0.5, WALL_WIDTH - 0.5);
            holds.push(Hold { x, y, kind: HoldKind::Relais });
        } else {
            let n = 2 + rng.index(2);
            for k in 0..n {
                // La première prise de la rangée est toujours à portée d'une prise d'en dessous.
                let x = if k == 0 {
                    let anchor = holds[prev_row[rng.index(prev_row.len())]];
                    anchor.x + rng.range_f32(-0.7, 0.7)
                } else {
                    rng.range_f32(0.4, WALL_WIDTH - 0.4)
                }
                .clamp(0.4, WALL_WIDTH - 0.4);
                // Pas deux prises l'une sur l'autre.
                if holds[row_start..].iter().any(|h| (h.x - x).abs() < 0.8) {
                    continue;
                }
                let r = rng.f32();
                let kind = if r < ice {
                    HoldKind::Verglas
                } else if r < ice + bac {
                    HoldKind::Bac
                } else if r < ice + bac + plat {
                    HoldKind::Plat
                } else {
                    HoldKind::Reglette
                };
                holds.push(Hold { x, y, kind });
            }
        }
        let linked: Vec<usize> = (row_start..holds.len()).filter(|&k| prev_row.iter().any(|&j| dist(holds[j], holds[k]) <= REACH)).collect();
        prev_row = linked;
    }
    // Sommet : une vire au-dessus de la dernière rangée.
    let anchor = holds[prev_row[0]];
    let top_y = (anchor.y + 1.0).max(height).min(anchor.y + REACH - 0.1);
    holds.push(Hold { x: anchor.x, y: top_y, kind: HoldKind::Relais });
    Route { name: name.to_string(), tier, height: top_y, holds }
}

/// Ce que l'équipement et la maîtrise changent sur la paroi.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClimbGear {
    /// Multiplicateur d'usure des bras.
    pub drain: f32,
    /// Multiplicateur de froid.
    pub cold: f32,
    /// Multiplicateur de risque de glissade.
    pub slip: f32,
    pub pitons: u32,
    /// Pour l'affichage : (tissu, cuir, plaque).
    pub armor: (u32, u32, u32),
}

impl ClimbGear {
    pub fn new(level: u32, cloth: u32, leather: u32, plate: u32) -> Self {
        let l = level.min(100) as f32;
        let drain = (1.0 + 0.12 * plate as f32 - 0.05 * leather as f32) / (1.0 + 0.015 * l);
        let cold = (1.0 + 0.15 * cloth as f32 - 0.12 * leather as f32 - 0.04 * plate as f32).max(0.4) / (1.0 + 0.01 * l);
        let slip = (1.0 + 0.1 * plate as f32) / (1.0 + 0.03 * l);
        Self { drain, cold, slip, pitons: 2 + level.min(100) / 20, armor: (cloth, leather, plate) }
    }

    /// Lit l'armure portée (tête, torse, pieds) et la Voie Escalade.
    pub fn of(c: &Content, ch: &Character) -> Self {
        let (mut t, mut l, mut p) = (0, 0, 0);
        for slot in [EquipSlot::Head, EquipSlot::Chest, EquipSlot::Feet] {
            if let Some(b) = ch.equipment.get(&slot).and_then(|it| c.base(&it.base)) {
                match b.line.as_str() {
                    "tissu" => t += 1,
                    "cuir" => l += 1,
                    "plaque" => p += 1,
                    _ => {}
                }
            }
        }
        Self::new(ch.masteries.level(LINE_CLIMB), t, l, p)
    }

    /// Un mot sur la tenue, pour le panneau.
    pub fn advice(&self) -> &'static str {
        let (t, l, p) = self.armor;
        if p >= 2 {
            "La plaque pèse : vos bras fatiguent vite."
        } else if t >= 2 {
            "Le tissu laisse passer le vent : vous aurez froid."
        } else if l >= 2 {
            "Le cuir : l'habit du grimpeur."
        } else {
            "Tenue sans avantage particulier."
        }
    }
}

/// Après une chute retenue : temps pendu à la corde (s).
pub const HANG_TIME: f32 = 3.0;
/// Au-delà, on vous redescend.
pub const MAX_FALLS: u32 = 4;

/// Risque de lâcher en prenant une prise verglacée.
pub const ICE_SLIP: f32 = 0.28;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClimbEnd {
    /// Au sommet.
    Top,
    /// Tombé au pied : `height` mètres de chute.
    Fell { height: f32 },
    /// Redescendu de soi-même (désescalade depuis le bas).
    GaveUp,
    /// Trop de chutes : les compagnons vous redescendent au bout de la corde.
    Lowered,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClimbEvent {
    Reached(usize),
    /// La prise verglacée a lâché.
    Slipped,
    /// Bras à bout.
    Pumped,
    /// Rattrapé par le piton / le relais après `height` m de chute.
    Caught { height: f32, at: usize },
    /// Chute jusqu'au pied.
    Grounded { height: f32 },
    PitonSet(usize),
    Top,
}

/// Une ascension en cours.
#[derive(Debug, Clone)]
pub struct Climb {
    pub route: Route,
    pub gear: ClimbGear,
    pub at: usize,
    /// Bras (0..100).
    pub grip: f32,
    /// Chaleur (0..100).
    pub warmth: f32,
    /// Mouvement en cours : (vers, temps restant).
    pub moving: Option<(usize, f32)>,
    /// Piton en cours de pose : temps restant.
    pub planting: Option<f32>,
    /// Suspendu à la corde après une chute : temps restant.
    pub hang: f32,
    /// Prise précédente (le robot évite d'y revenir).
    pub prev: usize,
    /// Point de protection le plus haut (prise).
    pub anchor: usize,
    pub pitons_left: u32,
    pub end: Option<ClimbEnd>,
    /// Hauteur cumulée des chutes arrêtées (pour les dégâts).
    pub fallen: f32,
    pub falls: u32,
    pub time: f32,
    pub best: f32,
    rng: Rng,
}

impl Climb {
    pub fn new(route: Route, gear: ClimbGear, seed: u64) -> Self {
        let pitons = gear.pitons;
        Self {
            route,
            gear,
            at: 0,
            grip: 100.0,
            warmth: 100.0,
            moving: None,
            planting: None,
            hang: 0.0,
            prev: 0,
            anchor: 0,
            pitons_left: pitons,
            end: None,
            fallen: 0.0,
            falls: 0,
            time: 0.0,
            best: 0.0,
            rng: Rng::new(seed),
        }
    }

    pub fn hold(&self) -> Hold {
        self.route.holds[self.at]
    }

    pub fn height(&self) -> f32 {
        self.hold().y
    }

    /// Doigts gourds : multiplicateur d'usure dû au froid.
    pub fn cold_factor(&self) -> f32 {
        if self.warmth <= 0.0 {
            2.2
        } else if self.warmth < 35.0 {
            1.5
        } else {
            1.0
        }
    }

    pub fn busy(&self) -> bool {
        self.moving.is_some() || self.planting.is_some() || self.hang > 0.0 || self.end.is_some()
    }

    /// Peut-on attraper cette prise maintenant ?
    pub fn can_reach(&self, k: usize) -> bool {
        !self.busy() && k != self.at && k < self.route.holds.len() && dist(self.hold(), self.route.holds[k]) <= REACH
    }

    /// Lance le mouvement vers la prise `k`. Faux si hors de portée.
    pub fn reach(&mut self, k: usize) -> bool {
        if !self.can_reach(k) {
            return false;
        }
        let d = dist(self.hold(), self.route.holds[k]);
        // Un jeté coûte plus cher.
        let cost = if d > REACH_EASY { 9.0 } else { 4.0 };
        self.grip -= cost * self.gear.drain * self.cold_factor();
        self.moving = Some((k, MOVE_TIME));
        true
    }

    pub fn can_plant(&self) -> bool {
        !self.busy() && self.pitons_left > 0 && self.hold().kind != HoldKind::Relais && self.at != self.anchor
    }

    pub fn plant(&mut self) -> bool {
        if !self.can_plant() {
            return false;
        }
        self.pitons_left -= 1;
        self.planting = Some(PITON_TIME);
        true
    }

    /// Redescendre (seulement depuis les premières prises, sans danger).
    pub fn can_give_up(&self) -> bool {
        !self.busy() && self.height() <= 2.0
    }

    pub fn give_up(&mut self) -> bool {
        if !self.can_give_up() {
            return false;
        }
        self.end = Some(ClimbEnd::GaveUp);
        true
    }

    /// Fait avancer le temps sur la paroi.
    pub fn tick(&mut self, dt: f32) -> Vec<ClimbEvent> {
        let mut ev = vec![];
        if self.end.is_some() {
            return ev;
        }
        self.time += dt;
        let kind = self.hold().kind;
        // Le froid.
        if kind.sheltered() {
            self.warmth = (self.warmth + 6.0 * dt).min(100.0);
        } else {
            let wind = 1.8 + 0.22 * self.route.tier as f32;
            self.warmth = (self.warmth - wind * self.gear.cold * dt).max(0.0);
        }
        // Pendu à la corde : on secoue les bras.
        if self.hang > 0.0 {
            self.hang -= dt;
            self.grip = (self.grip + 14.0 * dt).min(100.0);
            return ev;
        }
        // Les bras.
        let d = kind.drain();
        if d < 0.0 {
            // Récupérer : le froid freine aussi la récupération.
            let rec = -d / self.cold_factor();
            self.grip = (self.grip + rec * dt).min(100.0);
        } else {
            self.grip -= d * self.gear.drain * self.cold_factor() * dt;
        }
        if let Some(t) = self.planting.as_mut() {
            *t -= dt;
            if *t <= 0.0 {
                self.planting = None;
                self.anchor = self.at;
                ev.push(ClimbEvent::PitonSet(self.at));
            }
        }
        if let Some((to, t)) = self.moving.as_mut() {
            *t -= dt;
            if *t <= 0.0 {
                let to = *to;
                self.moving = None;
                self.prev = self.at;
                self.at = to;
                self.best = self.best.max(self.height());
                ev.push(ClimbEvent::Reached(to));
                let k = self.hold().kind;
                if k == HoldKind::Relais {
                    // Un relais protège : on s'y vache.
                    self.anchor = self.at;
                    self.planting = None;
                }
                if self.at == self.route.top() {
                    self.end = Some(ClimbEnd::Top);
                    ev.push(ClimbEvent::Top);
                    return ev;
                }
                if k == HoldKind::Verglas && self.rng.chance((ICE_SLIP * self.gear.slip).min(0.9)) {
                    ev.push(ClimbEvent::Slipped);
                    self.fall(&mut ev);
                    return ev;
                }
            }
        }
        if self.grip <= 0.0 {
            ev.push(ClimbEvent::Pumped);
            self.fall(&mut ev);
        }
        ev
    }

    fn fall(&mut self, ev: &mut Vec<ClimbEvent>) {
        self.falls += 1;
        self.moving = None;
        self.planting = None;
        let from = self.height();
        let a = self.route.holds[self.anchor];
        if self.anchor == 0 {
            // Personne pour vous retenir.
            self.end = Some(ClimbEnd::Fell { height: from });
            ev.push(ClimbEvent::Grounded { height: from });
            return;
        }
        // Rattrapé : on chute deux fois la distance au point + le mou de la corde.
        let h = 2.0 * (from - a.y).max(0.0) + 0.5;
        self.fallen += h;
        self.at = self.anchor;
        self.prev = self.anchor;
        self.grip = self.grip.max(20.0);
        ev.push(ClimbEvent::Caught { height: h, at: self.anchor });
        if self.falls >= MAX_FALLS {
            self.end = Some(ClimbEnd::Lowered);
        } else {
            self.hang = HANG_TIME;
        }
    }

    /// Part des PV max perdue pour l'ascension (chutes retenues + chute au sol).
    pub fn damage_frac(&self) -> f32 {
        let ground = match self.end {
            Some(ClimbEnd::Fell { height }) => 0.065 * height,
            _ => 0.0,
        };
        (0.025 * self.fallen + ground).min(0.85)
    }
}

/// Gloire d'Escalade : selon la hauteur gagnée et le tier.
pub fn climb_fame(route: &Route, best: f32, topped: bool) -> f64 {
    let base = 6.0 * best as f64 * tier_mult(route.tier) as f64;
    if topped { base * 1.5 } else { base * 0.5 }
}

/// Un grimpeur automatique (simulateur, démo) : monte vers la meilleure prise,
/// se repose sur les bacs, plante un piton tous les ~3 m.
pub fn bot_step(cl: &mut Climb) {
    if cl.busy() {
        return;
    }
    let here = cl.hold();
    // Se reposer.
    if (here.kind == HoldKind::Relais && (cl.grip < 95.0 || cl.warmth < 70.0)) || (here.kind == HoldKind::Bac && cl.grip < 55.0) {
        return;
    }
    // S'assurer.
    let anchor_y = cl.route.holds[cl.anchor].y;
    if cl.height() - anchor_y > 2.8 && cl.can_plant() && cl.grip > 30.0 {
        cl.plant();
        return;
    }
    // Itinéraire : le nombre de mouvements jusqu'au sommet depuis chaque prise.
    let hops = cl.route.hops_to_top();
    let opts = cl.route.reachable(cl.at);
    let score = |k: usize| {
        let h = cl.route.holds[k];
        let kind = match h.kind {
            HoldKind::Relais => 0.9,
            HoldKind::Bac => 0.6,
            HoldKind::Reglette => 0.3,
            HoldKind::Plat => 0.0,
            HoldKind::Verglas => -0.8,
        };
        let jete = if dist(here, h) > REACH_EASY { -0.3 } else { 0.0 };
        -(hops[k] as f32) + kind + jete
    };
    if let Some(best) = opts.into_iter().filter(|&k| hops[k] < hops[cl.at]).max_by(|a, b| score(*a).total_cmp(&score(*b))) {
        cl.reach(best);
    }
}

/// Temps de réflexion du robot entre deux gestes (s).
pub const THINK: f32 = 0.8;

/// Simule une ascension complète par le robot. Renvoie la fin.
pub fn simulate(route: &Route, gear: ClimbGear, seed: u64) -> (ClimbEnd, f32) {
    let mut cl = Climb::new(route.clone(), gear, seed);
    let dt = 0.05;
    // Un humain réfléchit avant chaque geste : ~0,8 s.
    let mut next = 0.0;
    while cl.end.is_none() && cl.time < 300.0 {
        if !cl.busy() && cl.time >= next {
            let before = (cl.at, cl.pitons_left);
            bot_step(&mut cl);
            if (cl.at, cl.pitons_left) != before || cl.busy() {
                next = cl.time + THINK;
            }
        }
        cl.tick(dt);
    }
    // Bloqué trop longtemps : on le redescend.
    (cl.end.unwrap_or(ClimbEnd::Lowered), cl.damage_frac())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rate(tier: u8, height: f32, ice: f32, gear: ClimbGear) -> f32 {
        let mut ok = 0;
        for s in 0..200 {
            let mut rng = Rng::new(s);
            let r = generate_route("t", tier, height, ice, &mut rng);
            if simulate(&r, gear, s * 7 + 1).0 == ClimbEnd::Top {
                ok += 1;
            }
        }
        ok as f32 / 200.0
    }

    #[test]
    fn voies_passables() {
        for s in 0..300 {
            let mut rng = Rng::new(s);
            let tier = 1 + (s % 8) as u8;
            let r = generate_route("t", tier, 8.0 + (s % 7) as f32, 0.2, &mut rng);
            assert!(r.passable(), "voie {s} impassable");
            assert!(r.height >= 7.5);
            assert!(r.holds.iter().all(|h| h.x >= 0.0 && h.x <= WALL_WIDTH));
        }
    }

    #[test]
    fn tenue_et_maitrise() {
        let novice = ClimbGear::new(0, 0, 0, 0);
        let cuir = ClimbGear::new(0, 0, 3, 0);
        let plaque = ClimbGear::new(0, 0, 0, 3);
        let expert = ClimbGear::new(40, 0, 3, 0);
        assert!(plaque.drain > novice.drain && cuir.drain < novice.drain);
        assert!(ClimbGear::new(0, 3, 0, 0).cold > novice.cold);
        assert!(expert.pitons > novice.pitons);
        // Le robot : le cuir passe mieux que la plaque, l'expert mieux que tous.
        let (rc, rp, re) = (rate(5, 12.0, 0.18, cuir), rate(5, 12.0, 0.18, plaque), rate(5, 12.0, 0.18, expert));
        assert!(rc > rp, "cuir {rc} plaque {rp}");
        assert!(re >= rc, "expert {re} cuir {rc}");
        assert!(re > 0.6, "expert {re}");
    }

    #[test]
    fn chute_retenue_par_le_piton() {
        let mut rng = Rng::new(4);
        let r = generate_route("t", 5, 12.0, 0.0, &mut rng);
        let mut cl = Climb::new(r, ClimbGear::new(0, 0, 0, 0), 1);
        // Monter un peu, planter, puis lâcher.
        while cl.height() < 3.0 {
            bot_step(&mut cl);
            cl.tick(0.05);
            if cl.end.is_some() {
                return;
            }
        }
        while !cl.can_plant() {
            cl.tick(0.05);
        }
        cl.plant();
        while cl.planting.is_some() {
            cl.tick(0.05);
        }
        let anchor = cl.at;
        cl.grip = 0.1;
        cl.moving = None;
        let ev = cl.tick(0.05);
        assert!(ev.iter().any(|e| matches!(e, ClimbEvent::Caught { .. })), "{ev:?}");
        assert_eq!(cl.at, anchor);
        assert!(cl.end.is_none());
        // Sans piton : au sol.
        let mut rng = Rng::new(4);
        let r = generate_route("t", 5, 12.0, 0.0, &mut rng);
        let mut cl = Climb::new(r, ClimbGear::new(0, 0, 0, 0), 1);
        while cl.height() < 3.0 {
            bot_step(&mut cl);
            // Le robot plante : on l'en empêche.
            cl.planting = None;
            cl.anchor = 0;
            cl.tick(0.05);
        }
        cl.grip = 0.1;
        cl.moving = None;
        cl.tick(0.05);
        assert!(matches!(cl.end, Some(ClimbEnd::Fell { .. })));
        assert!(cl.damage_frac() > 0.15);
    }
}
