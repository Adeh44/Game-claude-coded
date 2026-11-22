//! Le moteur de combat temps réel.
//!
//! Tout se passe sur un plan 2D. Le jeu appelle `tick(dt)` à chaque image,
//! lui donne les gestes du joueur (`set_move`, `request_skill`), puis lit
//! `fighters` pour placer les modèles et `drain_events()` pour les effets
//! visuels (chiffres de dégâts, explosions, zones au sol).

use crate::look::Look;
use crate::math::{Vec2, in_cone, in_line};
use crate::passive::{PassiveCond, PassiveEffect};
use crate::rng::Rng;
use crate::rules::{Action, Condition, Order, Rule};
use crate::skill::{
    BAR_SIZE, DamageKind, Delivery, Effect, SkillDef, SkillSlot, StatusKind, Tag, Targeting,
};
use crate::stats::{Stat, StatBlock, mitigation};
use serde::{Deserialize, Serialize};

pub type FighterId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Team {
    Ally,
    Enemy,
}

impl Team {
    pub fn other(self) -> Team {
        match self {
            Team::Ally => Team::Enemy,
            Team::Enemy => Team::Ally,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Controller {
    /// Piloté par les doigts du joueur.
    Player,
    /// Compagnon : suit le chef et applique ses règles.
    Companion { rules: Vec<Rule> },
    /// Monstre : menace + règles, revient chez lui s'il s'éloigne trop.
    Monster { rules: Vec<Rule>, aggro: f32, leash: f32 },
    /// Figurant : suit le chef, ne combat pas (porteur de torche...).
    Bystander,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatusInstance {
    pub kind: StatusKind,
    pub remaining: f32,
    pub duration: f32,
    /// Fraction (ralenti...) ou dégâts/soins par seconde par cumul.
    pub magnitude: f32,
    pub stacks: u32,
    pub source: FighterId,
    tick: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShieldInstance {
    pub amount: f32,
    pub remaining: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cast {
    pub slot: SkillSlot,
    pub target: Option<FighterId>,
    pub point: Vec2,
    pub remaining: f32,
    pub total: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Forced {
    velocity: Vec2,
    remaining: f32,
    /// Skill à résoudre à l'arrivée (bond).
    landing: Option<(SkillDef, Option<FighterId>)>,
}

/// Bilan d'un combattant pour la gloire et les statistiques.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Tally {
    pub damage_dealt: f32,
    pub healing_done: f32,
    pub damage_taken: f32,
    pub damage_mitigated: f32,
    pub kills: u32,
    pub fame_earned: f32,
}

/// Tout ce qu'il faut pour faire entrer quelqu'un dans un combat.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FighterSpec {
    pub name: String,
    pub team: Team,
    pub stats: StatBlock,
    pub skills: [Option<SkillDef>; BAR_SIZE],
    pub passives: Vec<PassiveEffect>,
    pub controller: Controller,
    pub look: Look,
    pub radius: f32,
    /// Gloire donnée à ceux qui l'abattent.
    pub fame: f32,
    /// Identifiant externe (id d'ennemi, de compagnon...).
    pub key: String,
    /// Tier affiché (ennemis).
    pub tier: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fighter {
    pub id: FighterId,
    pub name: String,
    pub key: String,
    pub team: Team,
    pub tier: u8,
    pub pos: Vec2,
    pub facing: Vec2,
    pub radius: f32,
    pub look: Look,
    pub base: StatBlock,
    pub stats: StatBlock,
    pub hp: f32,
    pub energy: f32,
    pub shields: Vec<ShieldInstance>,
    pub statuses: Vec<StatusInstance>,
    pub skills: [Option<SkillDef>; BAR_SIZE],
    pub cooldowns: [f32; BAR_SIZE],
    pub passives: Vec<PassiveEffect>,
    passive_cds: Vec<f32>,
    pub cast: Option<Cast>,
    pub controller: Controller,
    pub target: Option<FighterId>,
    /// Joystick (joueur).
    pub move_intent: Vec2,
    /// Skill demandé par le joueur, exécuté dès que possible.
    pub queued: Option<SkillSlot>,
    pub threat: Vec<(FighterId, f32)>,
    pub alive: bool,
    pub attack_timer: f32,
    /// Secondes depuis la dernière action de combat (0 = en plein combat).
    pub idle_time: f32,
    pub tally: Tally,
    pub fame: f32,
    pub home: Vec2,
    pub moving: bool,
    /// Pour l'animation : temps restant de l'animation d'attaque.
    pub swing: f32,
    think: f32,
    ai_move: Vec2,
    forced: Option<Forced>,
    returning: bool,
}

impl Fighter {
    pub fn hp_frac(&self) -> f32 {
        let m = self.stats.get(Stat::MaxHp).max(1.0);
        (self.hp / m).clamp(0.0, 1.0)
    }

    pub fn energy_frac(&self) -> f32 {
        let m = self.stats.get(Stat::MaxEnergy).max(1.0);
        (self.energy / m).clamp(0.0, 1.0)
    }

    pub fn shield_total(&self) -> f32 {
        self.shields.iter().map(|s| s.amount).sum()
    }

    pub fn has(&self, k: StatusKind) -> bool {
        self.statuses.iter().any(|s| s.kind == k)
    }

    pub fn status(&self, k: StatusKind) -> Option<&StatusInstance> {
        self.statuses.iter().find(|s| s.kind == k)
    }

    pub fn stacks(&self, k: StatusKind) -> u32 {
        self.status(k).map(|s| s.stacks).unwrap_or(0)
    }

    pub fn magnitude(&self, k: StatusKind) -> f32 {
        self.status(k).map(|s| s.magnitude).unwrap_or(0.0)
    }

    /// Ne peut ni bouger ni agir.
    pub fn incapacitated(&self) -> bool {
        self.has(StatusKind::Stun) || self.has(StatusKind::Frozen)
    }

    pub fn can_move(&self) -> bool {
        !self.incapacitated() && !self.has(StatusKind::Root)
    }

    pub fn can_cast(&self) -> bool {
        !self.incapacitated() && !self.has(StatusKind::Silence) && !self.has(StatusKind::Fear)
    }

    pub fn skill(&self, slot: SkillSlot) -> Option<&SkillDef> {
        self.skills[slot.bar_index()].as_ref()
    }

    pub fn cooldown(&self, slot: SkillSlot) -> f32 {
        self.cooldowns[slot.bar_index()]
    }

    /// Fraction de recharge restante (pour l'interface).
    pub fn cooldown_frac(&self, slot: SkillSlot) -> f32 {
        let Some(s) = self.skill(slot) else { return 0.0 };
        if s.cooldown <= 0.0 {
            return 0.0;
        }
        (self.cooldown(slot) / s.cooldown).clamp(0.0, 1.0)
    }

    pub fn is_ready(&self, slot: SkillSlot) -> bool {
        match self.skill(slot) {
            Some(s) => self.cooldown(slot) <= 0.0 && self.energy >= s.cost,
            None => false,
        }
    }

    pub fn in_combat(&self) -> bool {
        self.idle_time < 4.0
    }

    pub fn move_speed(&self) -> f32 {
        let mut slow: f32 = self.magnitude(StatusKind::Slow);
        slow = slow.max(self.stacks(StatusKind::Chill) as f32 * 0.1);
        let haste = self.magnitude(StatusKind::Haste);
        self.stats.get(Stat::MoveSpeed) * (1.0 - slow.min(0.8)) * (1.0 + haste)
    }

    fn top_threat(&self) -> Option<FighterId> {
        self.threat
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|t| t.0)
    }

    fn add_threat(&mut self, who: FighterId, amount: f32) {
        if let Some(t) = self.threat.iter_mut().find(|t| t.0 == who) {
            t.1 += amount;
        } else {
            self.threat.push((who, amount));
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ZoneShape {
    Circle { radius: f32 },
    Cone { dir: Vec2, range: f32, half_angle: f32 },
    Line { dir: Vec2, length: f32, half_width: f32 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Zone {
    pub id: u32,
    pub owner: FighterId,
    pub team: Team,
    pub center: Vec2,
    pub shape: ZoneShape,
    pub remaining: f32,
    pub total: f32,
    pub skill: SkillDef,
    pub target: Option<FighterId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Projectile {
    pub id: u32,
    pub owner: FighterId,
    pub team: Team,
    pub pos: Vec2,
    pub target: FighterId,
    pub speed: f32,
    pub skill: SkillDef,
    pub kind: Option<DamageKind>,
    /// Le projectile soigne (vert) ou blesse.
    pub friendly: bool,
}

/// Ce qui s'est passé, pour l'affichage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Event {
    Damage { src: FighterId, dst: FighterId, amount: f32, kind: DamageKind, crit: bool, absorbed: f32 },
    Heal { src: FighterId, dst: FighterId, amount: f32 },
    Shield { dst: FighterId, amount: f32 },
    Dodge { dst: FighterId },
    Block { dst: FighterId },
    StatusOn { dst: FighterId, kind: StatusKind },
    CastStart { who: FighterId, slot: SkillSlot, total: f32 },
    CastInterrupted { who: FighterId },
    SkillUsed { who: FighterId, slot: SkillSlot, skill: String, from: Vec2, to: Vec2, tags: Vec<Tag> },
    ProjectileSpawned { id: u32 },
    ProjectileHit { id: u32, at: Vec2 },
    ZoneSpawned { id: u32 },
    ZoneResolved { id: u32, center: Vec2 },
    Death { who: FighterId, killer: Option<FighterId> },
    Taunt { src: FighterId, dst: FighterId },
    /// Petit message au-dessus de quelqu'un ("Hors de portée !").
    Say { who: FighterId, text: String },
    Aggro { who: FighterId },
    Reset { who: FighterId },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Ongoing,
    Victory,
    Defeat,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Arena {
    /// Demi-taille du terrain (x, y) autour de l'origine.
    pub half_extent: Vec2,
    /// Obstacles circulaires (arbres, rochers) : (centre, rayon).
    pub obstacles: Vec<(Vec2, f32)>,
}

impl Default for Arena {
    fn default() -> Self {
        Self { half_extent: Vec2::new(20.0, 20.0), obstacles: vec![] }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Combat {
    pub fighters: Vec<Fighter>,
    pub zones: Vec<Zone>,
    pub projectiles: Vec<Projectile>,
    pub time: f32,
    pub arena: Arena,
    pub order: Order,
    pub rng: Rng,
    events: Vec<Event>,
    next_id: u32,
    /// Le combattant piloté par le joueur.
    pub player: Option<FighterId>,
    /// Chef du groupe allié (le joueur, ou le chef de compagnie PNJ).
    pub leader: Option<FighterId>,
}

const THINK_PERIOD: f32 = 0.2;
const DOT_PERIOD: f32 = 0.5;

impl Combat {
    pub fn new(seed: u64, arena: Arena) -> Self {
        Self {
            fighters: vec![],
            zones: vec![],
            projectiles: vec![],
            time: 0.0,
            arena,
            order: Order::Assist,
            rng: Rng::new(seed),
            events: vec![],
            next_id: 1,
            player: None,
            leader: None,
        }
    }

    fn new_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn spawn(&mut self, spec: FighterSpec, pos: Vec2) -> FighterId {
        let id = self.new_id();
        let hp = spec.stats.get(Stat::MaxHp);
        let energy = spec.stats.get(Stat::MaxEnergy);
        let is_player = spec.controller == Controller::Player;
        let n_passives = spec.passives.len();
        let facing = if spec.team == Team::Ally { Vec2::new(0.0, -1.0) } else { Vec2::new(0.0, 1.0) };
        self.fighters.push(Fighter {
            id,
            name: spec.name,
            key: spec.key,
            team: spec.team,
            tier: spec.tier,
            pos,
            facing,
            radius: spec.radius,
            look: spec.look,
            base: spec.stats.clone(),
            stats: spec.stats,
            hp,
            energy,
            shields: vec![],
            statuses: vec![],
            skills: spec.skills,
            cooldowns: [0.0; BAR_SIZE],
            passives: spec.passives,
            passive_cds: vec![0.0; n_passives],
            cast: None,
            controller: spec.controller,
            target: None,
            move_intent: Vec2::ZERO,
            queued: None,
            threat: vec![],
            alive: true,
            attack_timer: 0.0,
            idle_time: 99.0,
            tally: Tally::default(),
            fame: spec.fame,
            home: pos,
            moving: false,
            swing: 0.0,
            think: 0.0,
            ai_move: Vec2::ZERO,
            forced: None,
            returning: false,
        });
        if is_player {
            self.player = Some(id);
            if self.leader.is_none() {
                self.leader = Some(id);
            }
        }
        self.recompute_stats(self.fighters.len() - 1);
        id
    }

    /// Met à jour un combattant après un changement de fiche (niveau gagné,
    /// équipement changé) sans toucher à sa vie en pourcentage.
    pub fn refresh(&mut self, id: FighterId, spec: &FighterSpec) {
        let Some(i) = self.idx(id) else { return };
        let f = &mut self.fighters[i];
        let frac = f.hp_frac();
        f.base = spec.stats.clone();
        f.skills = spec.skills.clone();
        f.passives = spec.passives.clone();
        f.passive_cds = vec![0.0; f.passives.len()];
        f.look = spec.look.clone();
        self.recompute_stats(i);
        let f = &mut self.fighters[i];
        f.hp = frac * f.stats.get(Stat::MaxHp);
    }

    pub fn idx(&self, id: FighterId) -> Option<usize> {
        self.fighters.iter().position(|f| f.id == id)
    }

    pub fn get(&self, id: FighterId) -> Option<&Fighter> {
        self.fighters.iter().find(|f| f.id == id)
    }

    pub fn get_mut(&mut self, id: FighterId) -> Option<&mut Fighter> {
        self.fighters.iter_mut().find(|f| f.id == id)
    }

    pub fn drain_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    // ------------------------------------------------------------------
    // Gestes du joueur
    // ------------------------------------------------------------------

    pub fn set_move(&mut self, id: FighterId, dir: Vec2) {
        if let Some(f) = self.get_mut(id) {
            f.move_intent = dir.clamp_length(1.0);
        }
    }

    pub fn request_skill(&mut self, id: FighterId, slot: SkillSlot) {
        if let Some(f) = self.get_mut(id) {
            f.queued = Some(slot);
        }
    }

    pub fn set_target(&mut self, id: FighterId, target: Option<FighterId>) {
        if let Some(f) = self.get_mut(id) {
            f.target = target;
        }
    }

    /// Cible suivante (bouton « changer de cible ») : la plus proche après l'actuelle.
    pub fn cycle_target(&mut self, id: FighterId) {
        let Some(me) = self.get(id) else { return };
        let pos = me.pos;
        let team = me.team;
        let cur = me.target;
        let mut enemies: Vec<(FighterId, f32)> = self
            .fighters
            .iter()
            .filter(|f| f.alive && f.team != team && f.pos.distance(pos) < 14.0)
            .map(|f| (f.id, f.pos.distance(pos)))
            .collect();
        enemies.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        if enemies.is_empty() {
            return;
        }
        let next = match cur.and_then(|c| enemies.iter().position(|e| e.0 == c)) {
            Some(i) => enemies[(i + 1) % enemies.len()].0,
            None => enemies[0].0,
        };
        self.set_target(id, Some(next));
    }

    pub fn outcome(&self) -> Outcome {
        let enemies_alive = self.fighters.iter().any(|f| f.alive && f.team == Team::Enemy);
        let player_alive = match self.player {
            Some(p) => self.get(p).map(|f| f.alive).unwrap_or(false),
            None => self.fighters.iter().any(|f| f.alive && f.team == Team::Ally),
        };
        if !player_alive {
            Outcome::Defeat
        } else if !enemies_alive {
            Outcome::Victory
        } else {
            Outcome::Ongoing
        }
    }

    // ------------------------------------------------------------------
    // Boucle
    // ------------------------------------------------------------------

    pub fn tick(&mut self, dt: f32) {
        let dt = dt.min(0.1);
        self.time += dt;
        for i in 0..self.fighters.len() {
            if !self.fighters[i].alive {
                continue;
            }
            self.update_timers(i, dt);
        }
        for i in 0..self.fighters.len() {
            if !self.fighters[i].alive {
                continue;
            }
            self.update_statuses(i, dt);
        }
        for i in 0..self.fighters.len() {
            if !self.fighters[i].alive {
                continue;
            }
            self.recompute_stats(i);
            match self.fighters[i].controller.clone() {
                Controller::Player => self.player_decide(i),
                Controller::Companion { rules } => self.ai_decide(i, &rules, dt, false),
                Controller::Monster { rules, aggro, leash } => {
                    self.monster_aggro(i, aggro, leash);
                    self.ai_decide(i, &rules, dt, true)
                }
                Controller::Bystander => self.bystander(i),
            }
        }
        for i in 0..self.fighters.len() {
            if !self.fighters[i].alive {
                continue;
            }
            self.update_cast(i, dt);
            self.update_basic_attack(i);
            self.update_movement(i, dt);
        }
        self.separate();
        self.update_projectiles(dt);
        self.update_zones(dt);
    }

    fn update_timers(&mut self, i: usize, dt: f32) {
        let f = &mut self.fighters[i];
        for c in f.cooldowns.iter_mut() {
            *c = (*c - dt).max(0.0);
        }
        for c in f.passive_cds.iter_mut() {
            *c = (*c - dt).max(0.0);
        }
        f.attack_timer = (f.attack_timer - dt).max(0.0);
        f.swing = (f.swing - dt).max(0.0);
        f.idle_time += dt;
        for s in f.shields.iter_mut() {
            s.remaining -= dt;
        }
        f.shields.retain(|s| s.remaining > 0.0 && s.amount > 0.01);
        let max_e = f.stats.get(Stat::MaxEnergy);
        f.energy = (f.energy + f.stats.get(Stat::EnergyRegen) * dt).min(max_e);
        // Régénération de vie : forte hors combat.
        let max_hp = f.stats.get(Stat::MaxHp);
        let regen = f.stats.get(Stat::HpRegen) * if f.in_combat() { 1.0 } else { 6.0 };
        f.hp = (f.hp + regen * dt).min(max_hp);
    }

    fn update_statuses(&mut self, i: usize, dt: f32) {
        let mut dots: Vec<(FighterId, DamageKind, f32)> = vec![];
        let mut heals: Vec<(FighterId, f32)> = vec![];
        {
            let f = &mut self.fighters[i];
            for s in f.statuses.iter_mut() {
                s.remaining -= dt;
                s.tick += dt;
                while s.tick >= DOT_PERIOD {
                    s.tick -= DOT_PERIOD;
                    let amount = s.magnitude * s.stacks as f32 * DOT_PERIOD;
                    match s.kind {
                        StatusKind::Burn => dots.push((s.source, DamageKind::Fire, amount)),
                        StatusKind::Poison => dots.push((s.source, DamageKind::Nature, amount)),
                        StatusKind::Bleed => dots.push((s.source, DamageKind::Physical, amount)),
                        StatusKind::Regen => heals.push((s.source, amount)),
                        _ => {}
                    }
                }
            }
            f.statuses.retain(|s| s.remaining > 0.0);
        }
        let id = self.fighters[i].id;
        for (src, kind, amount) in dots {
            self.deal_damage(src, id, amount, kind, false, None, true);
        }
        for (src, amount) in heals {
            self.heal(src, id, amount, &[], true);
        }
    }

    /// Stats courantes = base + talents conditionnels + conversions.
    fn recompute_stats(&mut self, i: usize) {
        let near_enemies = {
            let f = &self.fighters[i];
            self.fighters
                .iter()
                .filter(|o| o.alive && o.team != f.team && o.pos.distance(f.pos) < 4.0)
                .count() as u32
        };
        let f = &mut self.fighters[i];
        let mut s = f.base.clone();
        let hp_frac = f.hp_frac();
        let shielded = !f.shields.is_empty();
        for p in &f.passives {
            if let PassiveEffect::StatWhile { cond, modifier } = p {
                let ok = match *cond {
                    PassiveCond::Always => true,
                    PassiveCond::HpBelow(t) => hp_frac < t,
                    PassiveCond::HpAbove(t) => hp_frac > t,
                    PassiveCond::Shielded => shielded,
                    PassiveCond::While(k) => f.statuses.iter().any(|st| st.kind == k),
                    PassiveCond::Surrounded(n) => near_enemies >= n,
                };
                if ok {
                    s.add(*modifier);
                }
            }
        }
        // Conversions (après les autres bonus).
        let snapshot = s.clone();
        for p in &f.passives {
            if let PassiveEffect::Convert { from, to, ratio } = p {
                s.add_flat(*to, snapshot.get(*from) * ratio);
            }
        }
        if let Some(ev) = f.status(StatusKind::Evasion) {
            s.add_flat(Stat::DodgeChance, ev.magnitude);
        }
        let old_max = f.stats.get(Stat::MaxHp);
        f.stats = s;
        let new_max = f.stats.get(Stat::MaxHp);
        if (new_max - old_max).abs() > 0.01 && old_max > 0.0 {
            // Garde la même fraction de vie quand le max change.
            f.hp = (f.hp / old_max * new_max).min(new_max);
        }
        f.hp = f.hp.min(new_max);
        f.energy = f.energy.min(f.stats.get(Stat::MaxEnergy));
    }

    // ------------------------------------------------------------------
    // Décisions
    // ------------------------------------------------------------------

    fn nearest_enemy(&self, i: usize, max_dist: f32) -> Option<FighterId> {
        let f = &self.fighters[i];
        self.fighters
            .iter()
            .filter(|o| o.alive && o.team != f.team && o.pos.distance(f.pos) <= max_dist)
            .min_by(|a, b| a.pos.distance(f.pos).partial_cmp(&b.pos.distance(f.pos)).unwrap())
            .map(|o| o.id)
    }

    fn lowest_ally(&self, i: usize, range: f32) -> Option<FighterId> {
        let f = &self.fighters[i];
        self.fighters
            .iter()
            .filter(|o| o.alive && o.team == f.team && o.pos.distance(f.pos) <= range)
            .min_by(|a, b| a.hp_frac().partial_cmp(&b.hp_frac()).unwrap())
            .map(|o| o.id)
    }

    fn target_valid(&self, i: usize) -> bool {
        let f = &self.fighters[i];
        match f.target.and_then(|t| self.get(t)) {
            Some(t) => t.alive && t.team != f.team,
            None => false,
        }
    }

    fn player_decide(&mut self, i: usize) {
        let taunter = self.taunter(i);
        if let Some(t) = taunter {
            self.fighters[i].target = Some(t);
        }
        if !self.target_valid(i) {
            let t = self.nearest_enemy(i, 9.0);
            self.fighters[i].target = t;
        }
        let f = &mut self.fighters[i];
        f.ai_move = f.move_intent;
        if f.move_intent.length() > 0.2
            && let Some(c) = &f.cast
                && c.total > 0.0 {
                    f.cast = None;
                    let id = f.id;
                    self.events.push(Event::CastInterrupted { who: id });
                }
        if let Some(slot) = self.fighters[i].queued.take() {
            self.try_use(i, slot, None, true);
        }
    }

    fn taunter(&self, i: usize) -> Option<FighterId> {
        self.fighters[i].status(StatusKind::Taunted).map(|s| s.source).filter(|src| {
            self.get(*src).map(|f| f.alive).unwrap_or(false)
        })
    }

    fn bystander(&mut self, i: usize) {
        let leader_pos = self.leader.and_then(|l| self.get(l)).map(|l| l.pos);
        let f = &mut self.fighters[i];
        f.ai_move = Vec2::ZERO;
        if let Some(lp) = leader_pos {
            let d = lp - f.pos;
            if d.length() > 2.5 {
                f.ai_move = d.normalize_or_zero();
            }
        }
    }

    fn monster_aggro(&mut self, i: usize, aggro: f32, leash: f32) {
        let (pos, home, team, has_threat, returning) = {
            let f = &self.fighters[i];
            (f.pos, f.home, f.team, !f.threat.is_empty(), f.returning)
        };
        // Trop loin de chez lui : il abandonne et rentre.
        if has_threat && pos.distance(home) > leash && self.taunter(i).is_none() {
            let f = &mut self.fighters[i];
            f.threat.clear();
            f.target = None;
            f.returning = true;
            f.cast = None;
            let id = f.id;
            self.events.push(Event::Reset { who: id });
            return;
        }
        if returning {
            let f = &mut self.fighters[i];
            if f.pos.distance(f.home) < 0.6 {
                f.returning = false;
                f.hp = f.stats.get(Stat::MaxHp);
                f.statuses.retain(|s| !s.kind.is_debuff());
            }
            return;
        }
        // Nettoie la menace des morts.
        let alive: Vec<FighterId> = self.fighters.iter().filter(|f| f.alive).map(|f| f.id).collect();
        self.fighters[i].threat.retain(|t| alive.contains(&t.0));
        if self.fighters[i].threat.is_empty()
            && let Some(intruder) = self
                .fighters
                .iter()
                .filter(|o| o.alive && o.team != team && o.pos.distance(pos) <= aggro)
                .filter(|o| o.controller != Controller::Bystander || o.pos.distance(pos) < aggro * 0.4)
                .min_by(|a, b| a.pos.distance(pos).partial_cmp(&b.pos.distance(pos)).unwrap())
                .map(|o| o.id)
            {
                self.fighters[i].add_threat(intruder, 1.0);
                let id = self.fighters[i].id;
                self.events.push(Event::Aggro { who: id });
                // La meute suit.
                for j in 0..self.fighters.len() {
                    let o = &self.fighters[j];
                    if j != i && o.alive && o.team == team && o.threat.is_empty() && o.pos.distance(pos) < 7.0
                        && matches!(o.controller, Controller::Monster { .. }) {
                            self.fighters[j].add_threat(intruder, 0.5);
                            let oid = self.fighters[j].id;
                            self.events.push(Event::Aggro { who: oid });
                        }
                }
            }
        let t = self.taunter(i).or_else(|| self.fighters[i].top_threat());
        self.fighters[i].target = t;
    }

    /// Cible du chef (pour « Attaquez ! »).
    fn leader_target(&self) -> Option<FighterId> {
        let l = self.get(self.leader?)?;
        let t = self.get(l.target?)?;
        (t.alive && (l.in_combat() || !t.threat.is_empty())).then_some(t.id)
    }

    /// Un ennemi qui s'en prend à un allié (autre que `i`).
    fn enemy_attacking_ally(&self, i: usize) -> Option<FighterId> {
        let me = &self.fighters[i];
        self.fighters
            .iter()
            .filter(|o| o.alive && o.team != me.team)
            .filter_map(|o| {
                let t = self.get(o.target?)?;
                (t.team == me.team && t.id != me.id && t.alive).then_some((o.id, o.pos.distance(me.pos)))
            })
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|x| x.0)
    }

    fn ai_decide(&mut self, i: usize, rules: &[Rule], dt: f32, monster: bool) {
        {
            let f = &mut self.fighters[i];
            f.think -= dt;
            if f.think > 0.0 {
                return;
            }
            f.think = THINK_PERIOD;
        }
        if self.fighters[i].returning {
            let f = &mut self.fighters[i];
            f.ai_move = (f.home - f.pos).normalize_or_zero();
            return;
        }
        // Choix de la cible pour les compagnons.
        if !monster {
            let taunter = self.taunter(i);
            let order = self.order;
            let t = taunter.or_else(|| match order {
                Order::Regroup => None,
                Order::Defensive => self.enemy_attacking_ally(i).or(if self.target_valid(i) && self.fighters[i].in_combat() {
                    self.fighters[i].target
                } else {
                    None
                }),
                Order::Assist => self
                    .leader_target()
                    .or_else(|| self.enemy_attacking_ally(i))
                    .or(if self.target_valid(i) { self.fighters[i].target } else { None })
                    .or_else(|| {
                        let any_fight = self.fighters.iter().any(|f| f.alive && f.team == Team::Enemy && !f.threat.is_empty());
                        if any_fight { self.nearest_enemy(i, 12.0) } else { None }
                    }),
            });
            // Un chef PNJ mène la marche : il va chercher le prochain ennemi.
            let is_npc_leader = self.leader == Some(self.fighters[i].id);
            // ... mais attend le porteur s'il traîne trop loin (hors combat).
            let player_far = self.player.and_then(|p| self.get(p)).filter(|p| p.alive).map(|p| p.pos.distance(self.fighters[i].pos) > 9.0).unwrap_or(false);
            let t = if t.is_none() && is_npc_leader && order != Order::Regroup && !player_far { self.nearest_enemy(i, 999.0) } else { t };
            if t.is_none() && is_npc_leader && player_far {
                let pp = self.player.and_then(|p| self.get(p)).map(|p| p.pos).unwrap_or(self.fighters[i].pos);
                let f = &mut self.fighters[i];
                f.target = None;
                f.ai_move = if pp.distance(f.pos) > 6.0 { (pp - f.pos).normalize_or_zero() * 0.6 } else { Vec2::ZERO };
                return;
            }
            self.fighters[i].target = t;
        }
        let leader_pos = self.leader.and_then(|l| self.get(l)).filter(|l| l.alive).map(|l| l.pos);
        let me_pos = self.fighters[i].pos;
        let is_leader = self.leader == Some(self.fighters[i].id);

        // Ordre « Regroupez » : on suit, point.
        if !monster && self.order == Order::Regroup && !is_leader {
            let f = &mut self.fighters[i];
            f.ai_move = Vec2::ZERO;
            if let Some(lp) = leader_pos
                && lp.distance(me_pos) > 2.5 {
                    f.ai_move = (lp - me_pos).normalize_or_zero();
                }
            return;
        }
        if self.fighters[i].cast.is_some() {
            self.fighters[i].ai_move = Vec2::ZERO;
            return;
        }

        // Règles.
        for rule in rules.iter().filter(|r| r.enabled) {
            let Some(ctx) = self.eval_condition(i, &rule.when) else { continue };
            match rule.then {
                Action::Use(slot) => {
                    if !self.fighters[i].is_ready(slot) || !self.fighters[i].can_cast() {
                        continue;
                    }
                    let Some(skill) = self.fighters[i].skill(slot).cloned() else { continue };
                    let target = self.pick_skill_target(i, &skill, ctx);
                    if skill.targeting.needs_enemy() && target.is_none() {
                        continue;
                    }
                    if matches!(skill.targeting, Targeting::Ally { .. } | Targeting::AreaAtAlly { .. }) && target.is_none() {
                        continue;
                    }
                    // Skills de zone autour de soi : seulement s'il y a du monde.
                    if let Targeting::AreaAroundSelf { radius } = skill.targeting {
                        let n = self.count_enemies_near(i, me_pos, radius);
                        if n == 0 {
                            continue;
                        }
                    }
                    if matches!(skill.targeting, Targeting::Cone { .. } | Targeting::Line { .. } | Targeting::Dash { .. })
                        && !self.target_valid(i)
                    {
                        continue;
                    }
                    let reach = skill.targeting.reach();
                    if let Some(tid) = target {
                        let tp = self.get(tid).map(|t| t.pos).unwrap_or(me_pos);
                        let tr = self.get(tid).map(|t| t.radius).unwrap_or(0.0);
                        let dist = tp.distance(me_pos) - tr - self.fighters[i].radius;
                        if reach > 0.0 && dist > reach && tid != self.fighters[i].id {
                            // Hors de portée : on s'approche, on s'engage sur cette règle.
                            let f = &mut self.fighters[i];
                            f.ai_move = (tp - me_pos).normalize_or_zero();
                            return;
                        }
                    }
                    if self.try_use(i, slot, target, false) {
                        self.fighters[i].ai_move = Vec2::ZERO;
                        return;
                    }
                }
                Action::Regroup => {
                    if let Some(lp) = leader_pos {
                        let f = &mut self.fighters[i];
                        f.ai_move = if lp.distance(me_pos) > 2.0 { (lp - me_pos).normalize_or_zero() } else { Vec2::ZERO };
                        return;
                    }
                }
                Action::Retreat => {
                    let threat_pos = self.fighters[i].target.and_then(|t| self.get(t)).map(|t| t.pos);
                    if let Some(tp) = threat_pos {
                        let f = &mut self.fighters[i];
                        f.ai_move = (me_pos - tp).normalize_or_zero();
                        return;
                    }
                }
            }
        }

        // Rien de spécial : se placer pour l'attaque de base, ou suivre le chef.
        let basic_reach = self.fighters[i]
            .skill(SkillSlot::Basic)
            .map(|s| s.targeting.reach())
            .unwrap_or(1.5);
        let target = self.fighters[i].target.and_then(|t| self.get(t)).filter(|t| t.alive).map(|t| (t.pos, t.radius));
        let my_r = self.fighters[i].radius;
        let f = &mut self.fighters[i];
        f.ai_move = Vec2::ZERO;
        if let Some((tp, tr)) = target {
            let gap = tp.distance(me_pos) - tr - my_r;
            if gap > basic_reach * 0.9 {
                f.ai_move = (tp - me_pos).normalize_or_zero();
            } else if basic_reach > 4.0 && gap < basic_reach * 0.35 && !monster {
                // Les tireurs gardent leurs distances.
                f.ai_move = (me_pos - tp).normalize_or_zero() * 0.7;
            }
        } else if !monster && !is_leader {
            if let Some(lp) = leader_pos
                && lp.distance(me_pos) > 3.0 {
                    f.ai_move = (lp - me_pos).normalize_or_zero();
                }
        } else if monster && f.pos.distance(f.home) > 1.0 && f.threat.is_empty() {
            f.ai_move = (f.home - f.pos).normalize_or_zero() * 0.5;
        }
    }

    fn count_enemies_near(&self, i: usize, p: Vec2, r: f32) -> usize {
        let team = self.fighters[i].team;
        self.fighters
            .iter()
            .filter(|o| o.alive && o.team != team && o.pos.distance(p) <= r + o.radius)
            .count()
    }

    /// Évalue une condition. `Some(cible éventuelle)` si elle est vraie.
    fn eval_condition(&self, i: usize, c: &Condition) -> Option<Option<FighterId>> {
        let me = &self.fighters[i];
        let target = me.target.and_then(|t| self.get(t)).filter(|t| t.alive);
        match *c {
            Condition::Always => Some(None),
            Condition::SelfHpBelow(t) => (me.hp_frac() < t).then_some(Some(me.id)),
            Condition::AllyHpBelow(t) => self
                .fighters
                .iter()
                .filter(|o| o.alive && o.team == me.team && o.hp_frac() < t && o.pos.distance(me.pos) < 16.0)
                .min_by(|a, b| a.hp_frac().partial_cmp(&b.hp_frac()).unwrap())
                .map(|o| Some(o.id)),
            Condition::SelfEnergyAbove(t) => (me.energy_frac() > t).then_some(None),
            Condition::EnemiesNear { count, radius } => {
                (self.count_enemies_near(i, me.pos, radius) as u32 >= count).then_some(None)
            }
            Condition::AllyAttacked => self.enemy_attacking_ally(i).map(Some),
            Condition::TargetHpBelow(t) => target.filter(|x| x.hp_frac() < t).map(|x| Some(x.id)),
            Condition::TargetCasting => target.filter(|x| x.cast.is_some()).map(|x| Some(x.id)),
            Condition::TargetMissing(k) => target.filter(|x| !x.has(k)).map(|x| Some(x.id)),
            Condition::SelfMissing(k) => (!me.has(k)).then_some(Some(me.id)),
            Condition::AllyDebuffed => self
                .fighters
                .iter()
                .filter(|o| o.alive && o.team == me.team && o.statuses.iter().any(|s| s.kind.is_debuff()))
                .map(|o| Some(o.id))
                .next(),
            Condition::SelfTargeted => self
                .fighters
                .iter()
                .any(|o| o.alive && o.team != me.team && o.target == Some(me.id))
                .then_some(None),
        }
    }

    /// Cible d'un skill selon le contexte de la règle.
    fn pick_skill_target(&self, i: usize, skill: &SkillDef, ctx: Option<FighterId>) -> Option<FighterId> {
        let me = &self.fighters[i];
        let ctx_f = ctx.and_then(|c| self.get(c));
        if skill.targeting.is_friendly() {
            match skill.targeting {
                Targeting::SelfOnly | Targeting::AlliesAroundSelf { .. } => Some(me.id),
                Targeting::Ally { range } | Targeting::AreaAtAlly { range, .. } => ctx_f
                    .filter(|c| c.team == me.team)
                    .map(|c| c.id)
                    .or_else(|| self.lowest_ally(i, range + 6.0)),
                _ => Some(me.id),
            }
        } else {
            ctx_f.filter(|c| c.team != me.team).map(|c| c.id).or(me.target)
        }
    }

    /// Lance (ou commence à incanter) un skill. Renvoie vrai si c'est parti.
    fn try_use(&mut self, i: usize, slot: SkillSlot, target: Option<FighterId>, player: bool) -> bool {
        let Some(skill) = self.fighters[i].skill(slot).cloned() else { return false };
        let id = self.fighters[i].id;
        if !self.fighters[i].can_cast() {
            if player {
                self.events.push(Event::Say { who: id, text: "Impossible !".into() });
            }
            return false;
        }
        if self.fighters[i].cast.is_some() {
            return false;
        }
        if self.fighters[i].cooldown(slot) > 0.0 {
            return false;
        }
        if self.fighters[i].energy < skill.cost {
            if player {
                self.events.push(Event::Say { who: id, text: "Pas assez d'énergie".into() });
            }
            return false;
        }
        let me_pos = self.fighters[i].pos;
        let my_r = self.fighters[i].radius;
        // Ciblage intelligent pour le joueur.
        let target = if player {
            match skill.targeting {
                Targeting::Ally { range } | Targeting::AreaAtAlly { range, .. } => {
                    // Le plus blessé à portée, sinon soi.
                    self.lowest_ally(i, range + 1.0).or(Some(id))
                }
                Targeting::SelfOnly | Targeting::AlliesAroundSelf { .. } => Some(id),
                _ => {
                    let reach = skill.targeting.reach();
                    let t = self.fighters[i].target.filter(|_| self.target_valid(i));
                    let in_reach = |c: &Combat, tid: FighterId| {
                        c.get(tid).map(|t| t.pos.distance(me_pos) - t.radius - my_r <= reach + 0.3).unwrap_or(false)
                    };
                    match t {
                        Some(t) if in_reach(self, t) => Some(t),
                        _ => self.nearest_enemy(i, reach + my_r + 1.0).or(t),
                    }
                }
            }
        } else {
            target
        };
        let target_f = target.and_then(|t| self.get(t)).cloned();
        if skill.targeting.needs_enemy() {
            let Some(t) = &target_f else {
                if player {
                    self.events.push(Event::Say { who: id, text: "Aucune cible".into() });
                }
                return false;
            };
            let reach = skill.targeting.reach();
            if t.pos.distance(me_pos) - t.radius - my_r > reach + 0.3 {
                if player {
                    self.events.push(Event::Say { who: id, text: "Hors de portée".into() });
                }
                return false;
            }
        }
        if let Some(t) = &target_f
            && t.id != id {
                let d = (t.pos - me_pos).normalize_or_zero();
                if d != Vec2::ZERO {
                    self.fighters[i].facing = d;
                }
            }
        let point = target_f.as_ref().map(|t| t.pos).unwrap_or(me_pos + self.fighters[i].facing * skill.targeting.reach().max(1.0));
        let haste = self.fighters[i].stats.get(Stat::Haste);
        let cast_time = skill.cast_time * (1.0 - haste * 0.5);
        if cast_time > 0.05 {
            self.fighters[i].cast = Some(Cast { slot, target, point, remaining: cast_time, total: cast_time });
            self.events.push(Event::CastStart { who: id, slot, total: cast_time });
        } else {
            self.execute(i, slot, target, point);
        }
        true
    }

    fn update_cast(&mut self, i: usize, dt: f32) {
        let f = &mut self.fighters[i];
        if f.incapacitated() || f.has(StatusKind::Silence) {
            if f.cast.take().is_some() {
                let id = f.id;
                self.events.push(Event::CastInterrupted { who: id });
            }
            return;
        }
        let done = match &mut f.cast {
            Some(c) => {
                c.remaining -= dt;
                c.remaining <= 0.0
            }
            None => false,
        };
        if done {
            let c = f.cast.take().unwrap();
            // La cible a pu bouger : on vise sa position actuelle.
            let point = c.target.and_then(|t| self.get(t)).map(|t| t.pos).unwrap_or(c.point);
            self.execute(i, c.slot, c.target, point);
        }
    }

    fn update_basic_attack(&mut self, i: usize) {
        let f = &self.fighters[i];
        if f.attack_timer > 0.0 || f.cast.is_some() || f.incapacitated() || f.has(StatusKind::Fear) {
            return;
        }
        if matches!(f.controller, Controller::Bystander) || f.returning {
            return;
        }
        if !matches!(f.controller, Controller::Player) && self.order == Order::Regroup && f.team == Team::Ally {
            return;
        }
        let Some(basic) = f.skill(SkillSlot::Basic).cloned() else { return };
        let Some(t) = f.target.and_then(|t| self.get(t)).filter(|t| t.alive && t.team != f.team) else { return };
        let reach = basic.targeting.reach();
        if t.pos.distance(f.pos) - t.radius - f.radius > reach + 0.2 {
            return;
        }
        // Monstres : n'attaquent que ce qui est dans leur table de menace.
        let tid = t.id;
        let tpos = t.pos;
        let speed = 1.0 + f.stats.get(Stat::AttackSpeed) + f.magnitude(StatusKind::Haste);
        let period = (basic.cooldown.max(0.4)) / speed.max(0.2);
        let fm = &mut self.fighters[i];
        fm.attack_timer = period;
        let d = (tpos - fm.pos).normalize_or_zero();
        if d != Vec2::ZERO {
            fm.facing = d;
        }
        self.execute(i, SkillSlot::Basic, Some(tid), tpos);
    }

    fn update_movement(&mut self, i: usize, dt: f32) {
        let half = self.arena.half_extent;
        let f = &mut self.fighters[i];
        // Mouvement forcé (bond, recul).
        if let Some(fc) = &mut f.forced {
            f.pos += fc.velocity * dt;
            fc.remaining -= dt;
            f.moving = true;
            if fc.remaining <= 0.0 {
                let landing = fc.landing.take();
                f.forced = None;
                if let Some((skill, target)) = landing {
                    let pos = f.pos;
                    let id = f.id;
                    let tags = skill.all_tags();
                    self.events.push(Event::SkillUsed {
                        who: id,
                        slot: skill.slot,
                        skill: skill.id.clone(),
                        from: pos,
                        to: pos,
                        tags,
                    });
                    self.resolve(id, &skill, pos, pos, target);
                }
            }
            self.clamp_to_arena(i, half);
            return;
        }
        let mut dir = f.ai_move;
        if let Some(fear) = f.status(StatusKind::Fear) {
            let src = fear.source;
            if let Some(s) = self.fighters.iter().find(|o| o.id == src).map(|o| o.pos) {
                let f = &mut self.fighters[i];
                dir = (f.pos - s).normalize_or_zero();
            }
        }
        let f = &mut self.fighters[i];
        if !f.can_move() || (f.cast.is_some() && !matches!(f.controller, Controller::Player)) {
            f.moving = false;
            return;
        }
        if f.cast.is_some() {
            f.moving = false;
            return;
        }
        let speed = f.move_speed();
        if dir.length() > 0.05 {
            f.pos += dir.clamp_length(1.0) * speed * dt;
            // Le joueur regarde où il va sauf s'il tape quelque chose.
            let has_target_close = matches!(f.controller, Controller::Player) && f.swing > 0.0;
            if !has_target_close {
                f.facing = dir.normalize_or_zero();
            }
            f.moving = true;
        } else {
            f.moving = false;
        }
        self.clamp_to_arena(i, half);
    }

    fn clamp_to_arena(&mut self, i: usize, half: Vec2) {
        let obstacles = self.arena.obstacles.clone();
        let f = &mut self.fighters[i];
        f.pos.x = f.pos.x.clamp(-half.x, half.x);
        f.pos.y = f.pos.y.clamp(-half.y, half.y);
        for (c, r) in obstacles {
            let d = f.pos - c;
            let min = r + f.radius;
            let l = d.length();
            if l < min {
                let n = if l > 1e-4 { d * (1.0 / l) } else { Vec2::X };
                f.pos = c + n * min;
            }
        }
    }

    /// Les combattants ne se marchent pas dessus.
    fn separate(&mut self) {
        let n = self.fighters.len();
        for a in 0..n {
            for b in (a + 1)..n {
                if !self.fighters[a].alive || !self.fighters[b].alive {
                    continue;
                }
                let d = self.fighters[b].pos - self.fighters[a].pos;
                let min = (self.fighters[a].radius + self.fighters[b].radius) * 0.9;
                let l = d.length();
                if l < min && l > 1e-5 {
                    let push = d * ((min - l) / l * 0.5);
                    self.fighters[a].pos -= push;
                    self.fighters[b].pos += push;
                } else if l <= 1e-5 {
                    self.fighters[b].pos.x += 0.05;
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Exécution des skills
    // ------------------------------------------------------------------

    fn execute(&mut self, i: usize, slot: SkillSlot, target: Option<FighterId>, point: Vec2) {
        let Some(skill) = self.fighters[i].skill(slot).cloned() else { return };
        let id = self.fighters[i].id;
        let team = self.fighters[i].team;
        let me_pos = self.fighters[i].pos;
        // Coût et recharge.
        {
            let f = &mut self.fighters[i];
            f.energy -= skill.cost;
            if slot != SkillSlot::Basic {
                let mut cd = skill.cooldown * (1.0 - f.stats.get(Stat::Haste));
                for p in &f.passives {
                    if let PassiveEffect::TagCooldown { tag, pct } = p
                        && skill.all_tags().contains(tag) {
                            cd *= 1.0 - pct;
                        }
                }
                f.cooldowns[slot.bar_index()] = cd.max(0.0);
            }
            f.idle_time = 0.0;
            f.swing = 0.35;
        }
        let tags = skill.all_tags();
        let dir = {
            let d = (point - me_pos).normalize_or_zero();
            if d == Vec2::ZERO { self.fighters[i].facing } else { d }
        };
        self.fighters[i].facing = dir;

        match skill.targeting {
            Targeting::Dash { distance, .. } => {
                let target_pos = target.and_then(|t| self.get(t)).map(|t| (t.pos, t.radius));
                let dest = match target_pos {
                    Some((tp, tr)) => {
                        let d = tp.distance(me_pos);
                        let travel = (d - tr - self.fighters[i].radius).clamp(0.0, distance);
                        me_pos + dir * travel
                    }
                    None => me_pos + dir * distance,
                };
                let dur = 0.18;
                self.fighters[i].forced = Some(Forced {
                    velocity: (dest - me_pos) * (1.0 / dur),
                    remaining: dur,
                    landing: Some((skill.clone(), target)),
                });
                self.fighters[i].cast = None;
                return;
            }
            Targeting::Leap { distance, .. } => {
                let dest = me_pos - dir * distance;
                let dur = 0.22;
                self.fighters[i].forced = Some(Forced {
                    velocity: (dest - me_pos) * (1.0 / dur),
                    remaining: dur,
                    landing: None,
                });
                self.events.push(Event::SkillUsed { who: id, slot, skill: skill.id.clone(), from: me_pos, to: me_pos, tags });
                self.resolve(id, &skill, me_pos, me_pos, target);
                return;
            }
            _ => {}
        }

        match skill.delivery {
            Delivery::Instant => {
                self.events.push(Event::SkillUsed { who: id, slot, skill: skill.id.clone(), from: me_pos, to: point, tags });
                self.resolve(id, &skill, me_pos, point, target);
            }
            Delivery::Projectile { speed } => {
                let Some(t) = target else {
                    self.resolve(id, &skill, me_pos, point, None);
                    return;
                };
                let pid = self.new_id();
                let kind = skill.effects.iter().find_map(|e| match e {
                    Effect::Damage { kind, .. } => Some(*kind),
                    _ => None,
                });
                let friendly = skill.targeting.is_friendly();
                self.projectiles.push(Projectile {
                    id: pid,
                    owner: id,
                    team,
                    pos: me_pos + dir * 0.5,
                    target: t,
                    speed,
                    skill: skill.clone(),
                    kind,
                    friendly,
                });
                self.events.push(Event::SkillUsed { who: id, slot, skill: skill.id.clone(), from: me_pos, to: point, tags });
                self.events.push(Event::ProjectileSpawned { id: pid });
            }
            Delivery::Delayed { delay } => {
                let zid = self.new_id();
                let (center, shape) = match skill.targeting {
                    Targeting::Cone { range, angle_deg } => (
                        me_pos,
                        ZoneShape::Cone { dir, range, half_angle: angle_deg.to_radians() * 0.5 },
                    ),
                    Targeting::Line { range, width } => {
                        (me_pos, ZoneShape::Line { dir, length: range, half_width: width * 0.5 })
                    }
                    Targeting::AreaAroundSelf { radius } | Targeting::AlliesAroundSelf { radius } => {
                        (me_pos, ZoneShape::Circle { radius })
                    }
                    Targeting::AreaAtTarget { radius, .. } | Targeting::AreaAtAlly { radius, .. } => {
                        (point, ZoneShape::Circle { radius })
                    }
                    _ => (point, ZoneShape::Circle { radius: 1.2 }),
                };
                self.zones.push(Zone {
                    id: zid,
                    owner: id,
                    team,
                    center,
                    shape,
                    remaining: delay,
                    total: delay,
                    skill: skill.clone(),
                    target,
                });
                self.events.push(Event::SkillUsed { who: id, slot, skill: skill.id.clone(), from: me_pos, to: center, tags });
                self.events.push(Event::ZoneSpawned { id: zid });
            }
        }
    }

    fn update_projectiles(&mut self, dt: f32) {
        let mut hits = vec![];
        for p in self.projectiles.iter_mut() {
            let Some(t) = self.fighters.iter().find(|f| f.id == p.target) else {
                hits.push((p.id, None));
                continue;
            };
            if !t.alive {
                hits.push((p.id, None));
                continue;
            }
            let d = t.pos - p.pos;
            let step = p.speed * dt;
            if d.length() <= step + t.radius * 0.5 {
                p.pos = t.pos;
                hits.push((p.id, Some(t.pos)));
            } else {
                p.pos += d.normalize_or_zero() * step;
            }
        }
        for (pid, at) in hits {
            let Some(idx) = self.projectiles.iter().position(|p| p.id == pid) else { continue };
            let p = self.projectiles.remove(idx);
            if let Some(at) = at {
                self.events.push(Event::ProjectileHit { id: pid, at });
                let origin = self.get(p.owner).map(|o| o.pos).unwrap_or(at);
                self.resolve(p.owner, &p.skill, origin, at, Some(p.target));
            } else {
                self.events.push(Event::ProjectileHit { id: pid, at: p.pos });
            }
        }
    }

    fn update_zones(&mut self, dt: f32) {
        let mut done = vec![];
        for z in self.zones.iter_mut() {
            z.remaining -= dt;
            if z.remaining <= 0.0 {
                done.push(z.id);
            }
        }
        for zid in done {
            let Some(idx) = self.zones.iter().position(|z| z.id == zid) else { continue };
            let z = self.zones.remove(idx);
            self.events.push(Event::ZoneResolved { id: zid, center: z.center });
            // Le lanceur peut être mort entre-temps : la zone part quand même.
            let victims = self.fighters_in_shape(z.team, z.center, &z.shape, z.skill.targeting.is_friendly());
            for v in victims {
                self.apply_effects(z.owner, &z.skill, v);
            }
        }
    }

    fn fighters_in_shape(&self, team: Team, center: Vec2, shape: &ZoneShape, friendly: bool) -> Vec<FighterId> {
        self.fighters
            .iter()
            .filter(|f| f.alive && (if friendly { f.team == team } else { f.team != team }))
            .filter(|f| match *shape {
                ZoneShape::Circle { radius } => f.pos.distance(center) <= radius + f.radius * 0.5,
                ZoneShape::Cone { dir, range, half_angle } => in_cone(center, dir, half_angle, range, f.pos, f.radius),
                ZoneShape::Line { dir, length, half_width } => in_line(center, dir, length, half_width, f.pos, f.radius),
            })
            .map(|f| f.id)
            .collect()
    }

    /// Trouve les victimes d'un skill et applique ses effets.
    fn resolve(&mut self, caster: FighterId, skill: &SkillDef, origin: Vec2, point: Vec2, target: Option<FighterId>) {
        let Some(c) = self.get(caster) else { return };
        let team = c.team;
        let facing = c.facing;
        let victims: Vec<FighterId> = match skill.targeting {
            Targeting::SelfOnly => vec![caster],
            Targeting::Enemy { .. } | Targeting::Ally { .. } => target.into_iter().filter(|t| self.get(*t).map(|f| f.alive).unwrap_or(false)).collect(),
            Targeting::AreaAtTarget { radius, .. } => self.fighters_in_shape(team, point, &ZoneShape::Circle { radius }, false),
            Targeting::AreaAtAlly { radius, .. } => self.fighters_in_shape(team, point, &ZoneShape::Circle { radius }, true),
            Targeting::AreaAroundSelf { radius } | Targeting::Dash { radius, .. } | Targeting::Leap { radius, .. } => {
                self.fighters_in_shape(team, origin, &ZoneShape::Circle { radius }, false)
            }
            Targeting::AlliesAroundSelf { radius } => self.fighters_in_shape(team, origin, &ZoneShape::Circle { radius }, true),
            Targeting::Cone { range, angle_deg } => {
                let dir = (point - origin).normalize_or_zero();
                let dir = if dir == Vec2::ZERO { facing } else { dir };
                self.fighters_in_shape(team, origin, &ZoneShape::Cone { dir, range, half_angle: angle_deg.to_radians() * 0.5 }, false)
            }
            Targeting::Line { range, width } => {
                let dir = (point - origin).normalize_or_zero();
                let dir = if dir == Vec2::ZERO { facing } else { dir };
                self.fighters_in_shape(team, origin, &ZoneShape::Line { dir, length: range, half_width: width * 0.5 }, false)
            }
        };
        // Effets « sur soi » appliqués une fois même sans victime.
        let self_effects: Vec<&Effect> = skill
            .effects
            .iter()
            .filter(|e| matches!(e, Effect::SelfStatus { .. } | Effect::SelfShield { .. } | Effect::SelfHeal { .. } | Effect::ReduceCooldowns { .. }))
            .collect();
        for e in self_effects {
            self.apply_effect(caster, caster, skill, e);
        }
        for v in victims {
            self.apply_effects(caster, skill, v);
        }
    }

    fn apply_effects(&mut self, caster: FighterId, skill: &SkillDef, victim: FighterId) {
        for e in &skill.effects {
            if matches!(e, Effect::SelfStatus { .. } | Effect::SelfShield { .. } | Effect::SelfHeal { .. } | Effect::ReduceCooldowns { .. }) {
                continue;
            }
            if !self.get(victim).map(|v| v.alive).unwrap_or(false) {
                return;
            }
            self.apply_effect(caster, victim, skill, e);
        }
        // Déclencheurs « quand je touche ».
        let hostile = self.get(caster).zip(self.get(victim)).map(|(c, v)| c.team != v.team).unwrap_or(false);
        if hostile {
            self.on_hit_triggers(caster, victim, skill);
        }
    }

    /// Bonus de puissance des talents pour un skill (mots-clés).
    fn tag_power(&self, caster: FighterId, skill: &SkillDef) -> f32 {
        let Some(c) = self.get(caster) else { return 0.0 };
        let tags = skill.all_tags();
        c.passives
            .iter()
            .filter_map(|p| match p {
                PassiveEffect::TagPower { tag, pct } if tags.contains(tag) => Some(*pct),
                _ => None,
            })
            .sum()
    }

    fn apply_effect(&mut self, caster: FighterId, victim: FighterId, skill: &SkillDef, e: &Effect) {
        let Some(c) = self.get(caster).cloned() else {
            // Lanceur disparu (zone retardée) : dégâts de base seulement.
            if let Effect::Damage { kind, base, .. } = e {
                self.deal_damage(caster, victim, *base, *kind, false, None, false);
            }
            return;
        };
        let power_bonus = 1.0 + self.tag_power(caster, skill) + c.magnitude(StatusKind::Empower) - c.magnitude(StatusKind::Weaken);
        match e {
            Effect::Damage { kind, base, scaling, bonus_vs, execute } => {
                let Some(v) = self.get(victim) else { return };
                let mut amount = (base + scaling * c.stats.get(kind.power_stat())) * power_bonus.max(0.1);
                if let Some((st, m)) = bonus_vs
                    && v.has(*st) {
                        amount *= m;
                    }
                if let Some((th, m)) = execute
                    && v.hp_frac() < *th {
                        amount *= m;
                    }
                let crit = self.rng.chance(c.stats.get(Stat::CritChance));
                if crit {
                    amount *= 1.0 + c.stats.get(Stat::CritDamage);
                }
                self.deal_damage(caster, victim, amount, *kind, crit, Some(skill), false);
            }
            Effect::Heal { base, scaling } => {
                let amount = (base + scaling * c.stats.get(Stat::HealPower)) * power_bonus.max(0.1);
                let tags = skill.all_tags();
                self.heal(caster, victim, amount, &tags, false);
                // Talents « quand je soigne ».
                let on_heal: Vec<Effect> = c
                    .passives
                    .iter()
                    .filter_map(|p| match p {
                        PassiveEffect::OnHeal { effect } => Some(effect.clone()),
                        _ => None,
                    })
                    .collect();
                for oe in on_heal {
                    self.apply_effect(caster, victim, skill, &oe);
                }
            }
            Effect::Shield { base, scaling, duration } => {
                let amount = (base + scaling * c.stats.get(Stat::HealPower)) * power_bonus.max(0.1) * (1.0 + c.stats.get(Stat::ShieldPower));
                self.add_shield(victim, amount, *duration);
            }
            Effect::SelfShield { base, scaling, duration } => {
                let amount = (base + scaling * c.stats.get(Stat::MaxHp)) * power_bonus.max(0.1) * (1.0 + c.stats.get(Stat::ShieldPower));
                self.add_shield(caster, amount, *duration);
            }
            Effect::SelfHeal { base, scaling } => {
                let amount = (base + scaling * c.stats.get(Stat::MaxHp)) * power_bonus.max(0.1);
                self.heal(caster, caster, amount, &[], false);
            }
            Effect::Status { kind, duration, magnitude, chance, stacks } => {
                if self.rng.chance(*chance) {
                    self.add_status(caster, victim, *kind, *duration, *magnitude, *stacks);
                }
            }
            Effect::SelfStatus { kind, duration, magnitude } => {
                self.add_status(caster, caster, *kind, *duration, *magnitude, 1);
            }
            Effect::Threat { amount } => {
                let mult = c.stats.get(Stat::ThreatMult).max(0.1);
                if let Some(v) = self.get_mut(victim)
                    && v.team != c.team {
                        v.add_threat(caster, amount * mult);
                    }
            }
            Effect::Taunt { duration } => {
                let Some(v) = self.get_mut(victim) else { return };
                if v.team == c.team {
                    return;
                }
                let top = v.threat.iter().map(|t| t.1).fold(0.0, f32::max);
                let mine = v.threat.iter().find(|t| t.0 == caster).map(|t| t.1).unwrap_or(0.0);
                if mine < top * 1.1 + 10.0 {
                    v.add_threat(caster, top * 1.1 + 10.0 - mine);
                }
                let duration = *duration;
                self.add_status(caster, victim, StatusKind::Taunted, duration, 0.0, 1);
                self.events.push(Event::Taunt { src: caster, dst: victim });
            }
            Effect::Knockback { distance } | Effect::Pull { distance } => {
                let pull = matches!(e, Effect::Pull { .. });
                let cpos = c.pos;
                let Some(v) = self.get_mut(victim) else { return };
                if v.team == c.team {
                    return;
                }
                let mut dir = (v.pos - cpos).normalize_or_zero();
                if dir == Vec2::ZERO {
                    dir = c.facing;
                }
                let mut dist = *distance;
                if pull {
                    dir = -dir;
                    dist = dist.min((v.pos.distance(cpos) - v.radius - c.radius - 0.2).max(0.0));
                }
                let dur = 0.2;
                v.forced = Some(Forced { velocity: dir * (dist / dur), remaining: dur, landing: None });
                v.cast = None;
            }
            Effect::RestoreEnergy { amount } => {
                if let Some(v) = self.get_mut(victim) {
                    v.energy = (v.energy + amount).min(v.stats.get(Stat::MaxEnergy));
                }
            }
            Effect::Cleanse => {
                if let Some(v) = self.get_mut(victim) {
                    v.statuses.retain(|s| !s.kind.is_debuff());
                }
            }
            Effect::Detonate { status, per_stack, kind } => {
                let stacks = self.get(victim).map(|v| v.stacks(*status)).unwrap_or(0);
                if stacks > 0 {
                    let amount = (per_stack * stacks as f32) * (1.0 + c.stats.get(kind.power_stat()) / 40.0) * power_bonus.max(0.1);
                    if let Some(v) = self.get_mut(victim) {
                        v.statuses.retain(|s| s.kind != *status);
                    }
                    self.deal_damage(caster, victim, amount, *kind, false, Some(skill), false);
                }
            }
            Effect::ReduceCooldowns { seconds } => {
                if let Some(cm) = self.get_mut(caster) {
                    for (k, cd) in cm.cooldowns.iter_mut().enumerate() {
                        if k != skill.slot.bar_index() {
                            *cd = (*cd - seconds).max(0.0);
                        }
                    }
                }
            }
        }
    }

    fn on_hit_triggers(&mut self, caster: FighterId, victim: FighterId, skill: &SkillDef) {
        let Some(c) = self.get(caster) else { return };
        let tags = skill.all_tags();
        let mut to_target = vec![];
        let mut to_self = vec![];
        for (k, p) in c.passives.iter().enumerate() {
            match p {
                PassiveEffect::OnHit { tag, chance, effect } => {
                    if tag.is_none_or(|t| tags.contains(&t)) {
                        to_target.push((*chance, effect.clone()));
                    }
                }
                PassiveEffect::OnHitSelf { tag, chance, effect, internal_cd }
                    if tag.is_none_or(|t| tags.contains(&t)) && c.passive_cds[k] <= 0.0 => {
                        to_self.push((k, *chance, effect.clone(), *internal_cd));
                    }
                _ => {}
            }
        }
        // Pas de chaîne infinie : les effets déclenchés n'en déclenchent pas d'autres.
        let dummy = SkillDef { tags: vec![], ..skill.clone() };
        for (chance, e) in to_target {
            if self.rng.chance(chance) {
                self.apply_effect(caster, victim, &dummy, &e);
            }
        }
        for (k, chance, e, icd) in to_self {
            if self.rng.chance(chance) {
                if let Some(cm) = self.get_mut(caster) {
                    cm.passive_cds[k] = icd;
                }
                self.apply_effect(caster, caster, &dummy, &e);
            }
        }
    }

    fn add_shield(&mut self, victim: FighterId, amount: f32, duration: f32) {
        if let Some(v) = self.get_mut(victim) {
            if !v.alive {
                return;
            }
            v.shields.push(ShieldInstance { amount, remaining: duration });
            self.events.push(Event::Shield { dst: victim, amount });
        }
    }

    pub fn add_status(&mut self, src: FighterId, dst: FighterId, kind: StatusKind, duration: f32, magnitude: f32, stacks: u32) {
        let (control, dur_mod, src_power) = match self.get(src) {
            Some(s) => {
                let dm: f32 = s
                    .passives
                    .iter()
                    .filter_map(|p| match p {
                        PassiveEffect::StatusDuration { status, pct } if *status == kind => Some(*pct),
                        _ => None,
                    })
                    .sum();
                let power = match kind {
                    StatusKind::Bleed => s.stats.get(Stat::PhysicalPower),
                    StatusKind::Regen => s.stats.get(Stat::HealPower),
                    _ => s.stats.get(Stat::MagicPower),
                };
                (s.stats.get(Stat::ControlPower), dm, power)
            }
            None => (0.0, 0.0, 0.0),
        };
        let Some(v) = self.get_mut(dst) else { return };
        if !v.alive {
            return;
        }
        let mut duration = duration * (1.0 + dur_mod);
        if kind.is_debuff() {
            duration *= 1.0 + control;
            if kind.is_hard_cc() || matches!(kind, StatusKind::Root | StatusKind::Slow | StatusKind::Chill | StatusKind::Silence) {
                duration *= 1.0 - v.stats.get(Stat::Tenacity);
            }
        }
        // Les dégâts / soins sur la durée grandissent avec la puissance.
        let magnitude = match kind {
            StatusKind::Burn | StatusKind::Poison | StatusKind::Bleed | StatusKind::Regen => magnitude + src_power * 0.12,
            _ => magnitude,
        };
        let max = kind.max_stacks();
        if let Some(s) = v.statuses.iter_mut().find(|s| s.kind == kind) {
            if kind.stacks() {
                s.stacks = (s.stacks + stacks).min(max);
            }
            s.remaining = s.remaining.max(duration);
            s.duration = s.duration.max(duration);
            s.magnitude = s.magnitude.max(magnitude);
            s.source = src;
        } else {
            v.statuses.push(StatusInstance {
                kind,
                remaining: duration,
                duration,
                magnitude,
                stacks: stacks.min(max).max(1),
                source: src,
                tick: 0.0,
            });
        }
        // Transi ×5 → Gelé.
        let mut froze = false;
        if kind == StatusKind::Chill && v.stacks(StatusKind::Chill) >= 5 {
            v.statuses.retain(|s| s.kind != StatusKind::Chill);
            froze = true;
        }
        if kind.is_hard_cc() && v.cast.is_some() {
            v.cast = None;
            self.events.push(Event::CastInterrupted { who: dst });
        }
        self.events.push(Event::StatusOn { dst, kind });
        if froze {
            self.add_status(src, dst, StatusKind::Frozen, 1.6, 0.0, 1);
        }
    }

    /// Inflige des dégâts : esquive, blocage, résistances, boucliers, menace.
    #[allow(clippy::too_many_arguments)]
    fn deal_damage(
        &mut self,
        src: FighterId,
        dst: FighterId,
        amount: f32,
        kind: DamageKind,
        crit: bool,
        skill: Option<&SkillDef>,
        is_dot: bool,
    ) {
        let Some(vi) = self.idx(dst) else { return };
        if !self.fighters[vi].alive {
            return;
        }
        let src_team = self.get(src).map(|s| s.team);
        if src_team == Some(self.fighters[vi].team) {
            return;
        }
        let melee = skill.map(|s| s.all_tags().contains(&Tag::Melee)).unwrap_or(false);
        let aoe = skill.map(|s| s.all_tags().contains(&Tag::Aoe)).unwrap_or(false);
        let mut amount = amount;
        if !is_dot {
            let dodge = self.fighters[vi].stats.get(Stat::DodgeChance);
            if !aoe && self.rng.chance(dodge) {
                self.events.push(Event::Dodge { dst });
                return;
            }
            let block = self.fighters[vi].stats.get(Stat::BlockChance);
            if kind == DamageKind::Physical && self.rng.chance(block) {
                amount *= 0.5;
                self.events.push(Event::Block { dst });
            }
        }
        let v = &self.fighters[vi];
        let resist = match kind {
            DamageKind::Physical => v.stats.get(Stat::Armor),
            DamageKind::True => 0.0,
            _ => v.stats.get(Stat::MagicResist),
        };
        let raw = amount;
        amount *= 1.0 - mitigation(resist);
        amount *= 1.0 + v.magnitude(StatusKind::Vulnerable) - v.magnitude(StatusKind::Fortify);
        amount = amount.max(0.0);
        let mitigated = (raw - amount).max(0.0);
        // Boucliers d'abord.
        let v = &mut self.fighters[vi];
        let mut absorbed = 0.0;
        let mut left = amount;
        for s in v.shields.iter_mut() {
            let take = s.amount.min(left);
            s.amount -= take;
            left -= take;
            absorbed += take;
            if left <= 0.0 {
                break;
            }
        }
        v.shields.retain(|s| s.amount > 0.01);
        v.hp -= left;
        v.idle_time = 0.0;
        v.tally.damage_taken += amount;
        v.tally.damage_mitigated += mitigated;
        // Menace pour l'attaquant.
        if let Some(si) = self.idx(src) {
            let mult = self.fighters[si].stats.get(Stat::ThreatMult).max(0.1);
            self.fighters[vi].add_threat(src, amount * mult);
            let s = &mut self.fighters[si];
            s.tally.damage_dealt += amount;
            s.idle_time = 0.0;
            // Vol de vie.
            let ls = s.stats.get(Stat::Lifesteal);
            if ls > 0.0 && !is_dot {
                let max = s.stats.get(Stat::MaxHp);
                s.hp = (s.hp + amount * ls).min(max);
            }
        }
        self.events.push(Event::Damage { src, dst, amount, kind, crit, absorbed });
        // Épines.
        if melee && !is_dot {
            let thorns = self.fighters[vi].magnitude(StatusKind::Thorns);
            if thorns > 0.0 {
                self.deal_damage(dst, src, amount * thorns, DamageKind::Physical, false, None, true);
            }
        }
        // Talents « quand je suis touché ».
        if !is_dot {
            let triggers: Vec<(usize, f32, Effect, bool, f32)> = self.fighters[vi]
                .passives
                .iter()
                .enumerate()
                .filter_map(|(k, p)| match p {
                    PassiveEffect::OnDamaged { chance, effect, to_attacker, internal_cd } => {
                        Some((k, *chance, effect.clone(), *to_attacker, *internal_cd))
                    }
                    _ => None,
                })
                .collect();
            let dummy = skill.cloned().unwrap_or_else(|| SkillDef {
                id: "reflexe".into(),
                name: String::new(),
                description: String::new(),
                line: String::new(),
                slot: SkillSlot::Basic,
                mastery: 0,
                cost: 0.0,
                cooldown: 0.0,
                cast_time: 0.0,
                targeting: Targeting::SelfOnly,
                delivery: Delivery::Instant,
                effects: vec![],
                tags: vec![],
                icon: String::new(),
            });
            for (k, chance, e, to_attacker, icd) in triggers {
                if self.fighters[vi].passive_cds.get(k).copied().unwrap_or(0.0) > 0.0 {
                    continue;
                }
                if self.fighters[vi].alive && self.rng.chance(chance) {
                    self.fighters[vi].passive_cds[k] = icd;
                    let tgt = if to_attacker { src } else { dst };
                    self.apply_effect(dst, tgt, &dummy, &e);
                }
            }
        }
        // Mort ?
        if self.fighters[vi].hp <= 0.0 && self.fighters[vi].alive {
            self.kill(vi, Some(src));
        }
    }

    fn kill(&mut self, vi: usize, killer: Option<FighterId>) {
        let dst = self.fighters[vi].id;
        let fame = self.fighters[vi].fame;
        let team = self.fighters[vi].team;
        {
            let v = &mut self.fighters[vi];
            v.alive = false;
            v.hp = 0.0;
            v.cast = None;
            v.statuses.clear();
            v.shields.clear();
            v.forced = None;
        }
        self.projectiles.retain(|p| p.target != dst);
        self.events.push(Event::Death { who: dst, killer });
        // Gloire partagée entre tous les vivants du camp adverse ayant participé.
        let participants: Vec<usize> = self
            .fighters
            .iter()
            .enumerate()
            .filter(|(_, f)| f.alive && f.team != team && (f.tally.damage_dealt > 0.0 || f.tally.healing_done > 0.0 || f.tally.damage_taken > 0.0 || f.in_combat()))
            .map(|(k, _)| k)
            .collect();
        let n = participants.len().max(1) as f32;
        for k in participants {
            // Bonus de groupe : à plusieurs on gagne un peu plus au total.
            self.fighters[k].tally.fame_earned += fame / n * (1.0 + 0.15 * (n - 1.0));
        }
        if let Some(ki) = killer.and_then(|k| self.idx(k)) {
            self.fighters[ki].tally.kills += 1;
            let on_kill: Vec<Effect> = self.fighters[ki]
                .passives
                .iter()
                .filter_map(|p| match p {
                    PassiveEffect::OnKill { effect } => Some(effect.clone()),
                    _ => None,
                })
                .collect();
            let kid = self.fighters[ki].id;
            if let Some(sk) = self.fighters[ki].skill(SkillSlot::Basic).cloned() {
                for e in on_kill {
                    self.apply_effect(kid, kid, &sk, &e);
                }
            }
        }
        for f in self.fighters.iter_mut() {
            f.threat.retain(|t| t.0 != dst);
            if f.target == Some(dst) {
                f.target = None;
            }
        }
    }

    fn heal(&mut self, src: FighterId, dst: FighterId, amount: f32, _tags: &[Tag], is_hot: bool) {
        let Some(vi) = self.idx(dst) else { return };
        if !self.fighters[vi].alive {
            return;
        }
        let v = &mut self.fighters[vi];
        let max = v.stats.get(Stat::MaxHp);
        let real = amount.min(max - v.hp).max(0.0);
        v.hp += real;
        let team = v.team;
        if let Some(si) = self.idx(src) {
            let s = &mut self.fighters[si];
            s.tally.healing_done += real;
            if !is_hot {
                s.idle_time = 0.0;
            }
            // Les soins attirent l'attention des ennemis engagés.
            let mult = s.stats.get(Stat::ThreatMult).max(0.1);
            for f in self.fighters.iter_mut() {
                if f.alive && f.team != team && !f.threat.is_empty() {
                    f.add_threat(src, real * 0.5 * mult);
                }
            }
        }
        if real > 0.5 && !is_hot || real > 0.0 && is_hot && real >= 1.0 {
            self.events.push(Event::Heal { src, dst, amount: real });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::Modifier;

    fn basic_melee(dmg: f32) -> SkillDef {
        SkillDef {
            id: "coup".into(),
            name: "Coup".into(),
            description: String::new(),
            line: "test".into(),
            slot: SkillSlot::Basic,
            mastery: 0,
            cost: 0.0,
            cooldown: 1.0,
            cast_time: 0.0,
            targeting: Targeting::Enemy { range: 1.5 },
            delivery: Delivery::Instant,
            effects: vec![Effect::Damage { kind: DamageKind::Physical, base: dmg, scaling: 0.0, bonus_vs: None, execute: None }],
            tags: vec![Tag::Melee, Tag::Basic],
            icon: String::new(),
        }
    }

    fn spec(name: &str, team: Team, hp: f32, dmg: f32, controller: Controller) -> FighterSpec {
        let mut stats = StatBlock::new();
        stats.add(Modifier::flat(Stat::MaxHp, hp));
        stats.add(Modifier::flat(Stat::MoveSpeed, 4.0));
        stats.add(Modifier::flat(Stat::MaxEnergy, 50.0));
        stats.add(Modifier::flat(Stat::ThreatMult, 1.0));
        let mut skills: [Option<SkillDef>; BAR_SIZE] = Default::default();
        skills[0] = Some(basic_melee(dmg));
        FighterSpec {
            name: name.into(),
            team,
            stats,
            skills,
            passives: vec![],
            controller,
            look: Look::default(),
            radius: 0.4,
            fame: 10.0,
            key: name.into(),
            tier: 1,
        }
    }

    fn monster() -> Controller {
        Controller::Monster { rules: vec![], aggro: 8.0, leash: 30.0 }
    }

    #[test]
    fn le_plus_fort_gagne() {
        let mut c = Combat::new(1, Arena::default());
        c.spawn(spec("Héros", Team::Ally, 100.0, 10.0, Controller::Companion { rules: vec![] }), Vec2::new(0.0, 0.0));
        c.spawn(spec("Rat", Team::Enemy, 30.0, 2.0, monster()), Vec2::new(3.0, 0.0));
        for _ in 0..600 {
            c.tick(1.0 / 30.0);
        }
        let rat = &c.fighters[1];
        assert!(!rat.alive, "le rat aurait dû mourir");
        assert!(c.fighters[0].alive);
        assert!(c.fighters[0].tally.fame_earned > 0.0);
    }

    #[test]
    fn menace_et_provocation() {
        let mut c = Combat::new(2, Arena::default());
        let dps = c.spawn(spec("Dps", Team::Ally, 100.0, 10.0, Controller::Companion { rules: vec![] }), Vec2::new(0.0, 0.0));
        let tank = c.spawn(spec("Tank", Team::Ally, 300.0, 1.0, Controller::Companion { rules: vec![] }), Vec2::new(0.5, 0.0));
        let ogre = c.spawn(spec("Ogre", Team::Enemy, 10_000.0, 5.0, monster()), Vec2::new(2.0, 0.0));
        for _ in 0..60 {
            c.tick(1.0 / 30.0);
        }
        // Le dps tape plus fort : l'ogre le vise.
        assert_eq!(c.get(ogre).unwrap().target, Some(dps));
        // Le tank provoque.
        let sk = SkillDef {
            effects: vec![Effect::Taunt { duration: 3.0 }],
            targeting: Targeting::AreaAroundSelf { radius: 5.0 },
            ..basic_melee(0.0)
        };
        c.apply_effects(tank, &sk, ogre);
        c.tick(1.0 / 30.0);
        assert_eq!(c.get(ogre).unwrap().target, Some(tank));
    }

    #[test]
    fn transi_cinq_fois_gele() {
        let mut c = Combat::new(3, Arena::default());
        let a = c.spawn(spec("Mage", Team::Ally, 100.0, 1.0, Controller::Player), Vec2::ZERO);
        let b = c.spawn(spec("Cible", Team::Enemy, 100.0, 1.0, monster()), Vec2::new(5.0, 0.0));
        for _ in 0..5 {
            c.add_status(a, b, StatusKind::Chill, 3.0, 0.0, 1);
        }
        let t = c.get(b).unwrap();
        assert!(t.has(StatusKind::Frozen));
        assert!(!t.has(StatusKind::Chill));
        assert!(t.incapacitated());
    }

    #[test]
    fn bouclier_absorbe() {
        let mut c = Combat::new(4, Arena::default());
        let a = c.spawn(spec("A", Team::Ally, 100.0, 1.0, Controller::Player), Vec2::ZERO);
        let b = c.spawn(spec("B", Team::Enemy, 100.0, 1.0, monster()), Vec2::new(5.0, 0.0));
        c.add_shield(a, 30.0, 5.0);
        c.deal_damage(b, a, 20.0, DamageKind::True, false, None, true);
        assert_eq!(c.get(a).unwrap().hp, 100.0);
        c.deal_damage(b, a, 20.0, DamageKind::True, false, None, true);
        assert!((c.get(a).unwrap().hp - 90.0).abs() < 1e-3);
    }

    #[test]
    fn zone_retardee_esquivable() {
        let mut c = Combat::new(5, Arena::default());
        let a = c.spawn(spec("A", Team::Ally, 100.0, 1.0, Controller::Player), Vec2::ZERO);
        let b = c.spawn(spec("B", Team::Enemy, 100.0, 1.0, monster()), Vec2::new(10.0, 0.0));
        let slam = SkillDef {
            targeting: Targeting::AreaAtTarget { range: 20.0, radius: 2.0 },
            delivery: Delivery::Delayed { delay: 1.0 },
            effects: vec![Effect::Damage { kind: DamageKind::True, base: 50.0, scaling: 0.0, bonus_vs: None, execute: None }],
            slot: SkillSlot::Weapon1,
            ..basic_melee(0.0)
        };
        c.get_mut(b).unwrap().skills[1] = Some(slam);
        let bi = c.idx(b).unwrap();
        c.try_use(bi, SkillSlot::Weapon1, Some(a), false);
        assert_eq!(c.zones.len(), 1);
        // Le joueur s'enfuit.
        c.set_move(a, Vec2::new(-1.0, 0.0));
        for _ in 0..40 {
            c.tick(1.0 / 30.0);
        }
        assert!(c.get(a).unwrap().hp > 99.0, "le joueur aurait dû esquiver");
    }

    #[test]
    fn reproductible() {
        let run = || {
            let mut c = Combat::new(77, Arena::default());
            c.spawn(spec("A", Team::Ally, 100.0, 7.0, Controller::Companion { rules: vec![] }), Vec2::ZERO);
            c.spawn(spec("B", Team::Enemy, 120.0, 6.0, monster()), Vec2::new(3.0, 0.0));
            for _ in 0..300 {
                c.tick(1.0 / 30.0);
            }
            (c.fighters[0].hp, c.fighters[1].hp)
        };
        assert_eq!(run(), run());
    }
}
