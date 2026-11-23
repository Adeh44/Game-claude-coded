//! Quêtes écrites : une offre à la taverne, des étapes (chacune une mission
//! aux monstres fixés, parfois une embuscade), une récompense.
//!
//! L'avancement vit dans `Profile.quests` : absent = pas encore acceptée,
//! `n` = étape en cours, `steps.len()` = terminée.

use crate::content::{Content, ZoneDef};
use crate::expedition::Layout;
use crate::item::{Item, Quality};
use crate::math::Vec2;
use crate::profile::Profile;
use crate::rng::Rng;
use serde::{Deserialize, Serialize};

/// Une embuscade : des ennemis qui surgissent autour du héros quand il a
/// parcouru une fraction `at` du chemin (0 = départ, 1 = fond de la zone).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ambush {
    pub at: f32,
    pub enemies: Vec<(String, u32)>,
    pub line: String,
}

/// Une expédition écrite : la zone sert de décor, les monstres sont fixés.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mission {
    pub name: String,
    pub zone: String,
    #[serde(default)]
    pub size: Option<f32>,
    pub packs: Vec<Vec<(String, u32)>>,
    #[serde(default)]
    pub boss: Option<String>,
    #[serde(default)]
    pub ambush: Option<Ambush>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuestStep {
    pub id: String,
    pub title: String,
    pub objective: String,
    /// Dialogue au moment où l'étape commence (vide pour la première : c'est l'offre).
    #[serde(default)]
    pub intro: Vec<String>,
    /// Ce qu'on raconte quand l'étape est réussie.
    pub done: Vec<String>,
    pub mission: Mission,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct QuestReward {
    #[serde(default)]
    pub gold: u32,
    #[serde(default)]
    pub unique: Option<String>,
    #[serde(default)]
    pub reputation: Vec<(String, i32)>,
    #[serde(default)]
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuestDef {
    pub id: String,
    pub name: String,
    pub giver: String,
    /// Phrase d'accroche à la taverne.
    pub hook: String,
    /// Expéditions déjà faites avant que la rumeur circule.
    #[serde(default)]
    pub min_expeditions: u32,
    pub offer: Vec<String>,
    pub steps: Vec<QuestStep>,
    pub epilogue: Vec<String>,
    pub reward: QuestReward,
}

impl QuestDef {
    /// Tier de l'étape (celui de la zone qui lui sert de décor).
    pub fn step_tier(&self, c: &Content, step: usize) -> u8 {
        self.steps.get(step).and_then(|s| c.zone(&s.mission.zone)).map(|z| z.tier).unwrap_or(1)
    }
}

/// Où en est le joueur dans une quête.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestStatus {
    /// Pas encore de rumeur (il faut avoir fait ses preuves).
    Locked,
    /// La rumeur attend à la taverne.
    Offered,
    /// Étape n° en cours.
    Active(usize),
    Done,
}

pub fn status(p: &Profile, q: &QuestDef) -> QuestStatus {
    match p.quests.get(&q.id) {
        None if p.counters.expeditions < q.min_expeditions => QuestStatus::Locked,
        None => QuestStatus::Offered,
        Some(&n) if (n as usize) < q.steps.len() => QuestStatus::Active(n as usize),
        Some(_) => QuestStatus::Done,
    }
}

/// Accepte la quête (seulement si elle est proposée).
pub fn accept(p: &mut Profile, q: &QuestDef) -> bool {
    if status(p, q) != QuestStatus::Offered {
        return false;
    }
    p.quests.insert(q.id.clone(), 0);
    true
}

/// L'étape `step` est réussie : on avance, et à la fin on donne la récompense.
/// Renvoie les lignes à afficher au bilan (vide si l'étape n'était pas en cours).
pub fn complete_step(c: &Content, p: &mut Profile, q: &QuestDef, step: usize) -> Vec<String> {
    if status(p, q) != QuestStatus::Active(step) {
        return vec![];
    }
    let mut lines = q.steps[step].done.clone();
    let next = step + 1;
    p.quests.insert(q.id.clone(), next as u32);
    match q.steps.get(next) {
        Some(s) => lines.push(format!("Quête « {} » : {}", q.name, s.objective)),
        None => {
            lines.push(format!("Quête terminée : {} !", q.name));
            lines.extend(give_reward(c, p, &q.reward));
            p.set_flag(&format!("quete:{}", q.id));
        }
    }
    lines
}

/// Crée l'objet unique (qualité Excellente, comme ceux des boss).
pub fn unique_item(c: &Content, p: &mut Profile, id: &str) -> Option<Item> {
    let u = c.unique(id)?;
    let mut it = p.new_item(&u.base, u.tier);
    it.quality = Quality::Excellent;
    it.unique = Some(u.id.clone());
    Some(it)
}

fn give_reward(c: &Content, p: &mut Profile, r: &QuestReward) -> Vec<String> {
    let mut lines = r.lines.clone();
    if r.gold > 0 {
        p.earn(r.gold);
        lines.push(format!("+{} pièces d'or.", r.gold));
    }
    if let Some(id) = &r.unique {
        if let Some(it) = unique_item(c, p, id) {
            let name = c.unique(id).map(|u| u.name.clone()).unwrap_or_default();
            // Une récompense de quête ne se perd pas : sac plein ou non, elle y va.
            if !p.add_to_bag(c, it.clone()) {
                p.bag.push(it);
            }
            lines.push(format!("+ {name} (unique)"));
        }
    }
    for (who, n) in &r.reputation {
        *p.reputation.entry(who.clone()).or_insert(0) += n;
        lines.push(format!("Réputation ({}) : +{n}.", reputation_label(who)));
    }
    lines
}

pub fn reputation_label(key: &str) -> &str {
    match key {
        "gue_rouge" => "Gué-Rouge",
        k => k,
    }
}

/// La zone telle que la mission la dessine (taille, boss, nombre de groupes).
pub fn mission_zone(c: &Content, m: &Mission) -> Option<ZoneDef> {
    let mut z = c.zone(&m.zone)?.clone();
    z.name = m.name.clone();
    if let Some(s) = m.size {
        z.size = s;
    }
    z.boss = m.boss.clone();
    z.pack_count = m.packs.len() as u32;
    Some(z)
}

/// Une mission en cours sur le terrain : suit l'embuscade et l'objectif.
#[derive(Debug, Clone, PartialEq)]
pub struct MissionRun {
    pub quest: String,
    pub step: usize,
    /// Embuscade pas encore déclenchée.
    pub ambush: Option<Ambush>,
    /// Le héros déclenche l'embuscade en passant au nord de cette ligne.
    pub trigger_y: f32,
    pub boss: Option<String>,
    pub done: bool,
}

impl MissionRun {
    pub fn new(quest: &QuestDef, step: usize, layout: &Layout) -> Option<Self> {
        let m = &quest.steps.get(step)?.mission;
        let at = m.ambush.as_ref().map(|a| a.at).unwrap_or(0.0).clamp(0.0, 1.0);
        let far = -layout.half;
        Some(MissionRun {
            quest: quest.id.clone(),
            step,
            ambush: m.ambush.clone(),
            trigger_y: layout.start.y + (far - layout.start.y) * at,
            boss: m.boss.clone(),
            done: false,
        })
    }

    /// Le héros avance : renvoie l'embuscade si elle se déclenche maintenant.
    pub fn check_ambush(&mut self, hero: Vec2) -> Option<Ambush> {
        if self.ambush.is_some() && hero.y <= self.trigger_y {
            return self.ambush.take();
        }
        None
    }

    /// Un ennemi est tombé. Le boss abattu = mission réussie.
    pub fn on_kill(&mut self, enemy: &str) {
        if self.boss.as_deref() == Some(enemy) {
            self.done = true;
        }
    }

    /// Plus aucun ennemi debout. Sans boss, c'est réussi... une fois l'embuscade passée.
    pub fn on_cleared(&mut self) {
        if self.boss.is_none() && self.ambush.is_none() {
            self.done = true;
        }
    }

    /// La zone peut-elle être annoncée « nettoyée » ? Pas avant l'embuscade.
    pub fn quiet(&self) -> bool {
        self.ambush.is_none()
    }
}

/// Où surgissent les embusqués : en cercle autour du héros, devant surtout,
/// sans sortir de la zone.
pub fn ambush_spots(hero: Vec2, n: usize, half: f32, rng: &mut Rng) -> Vec<Vec2> {
    let mut v = vec![];
    for k in 0..n {
        // Un arc de -200° à +20° autour du « nord » (vers -y) : surtout devant et sur les côtés.
        let a = -std::f32::consts::FRAC_PI_2 + (k as f32 / n.max(1) as f32 - 0.5) * 3.4 + rng.range_f32(-0.2, 0.2);
        let r = rng.range_f32(5.0, 6.5);
        let p = hero + Vec2::from_angle(a) * r;
        v.push(Vec2::new(p.x.clamp(-half + 1.0, half - 1.0), p.y.clamp(-half + 1.0, half - 1.0)));
    }
    v
}

/// Toutes les références d'une quête pointent-elles vers quelque chose ?
pub fn validate(c: &Content, q: &QuestDef) -> Vec<String> {
    let mut e = vec![];
    let enemy = |id: &str, e: &mut Vec<String>, step: &str| {
        if c.enemy(id).is_none() {
            e.push(format!("quête {} / {step} : ennemi inconnu {id}", q.id));
        }
    };
    if q.steps.is_empty() {
        e.push(format!("quête {} : aucune étape", q.id));
    }
    for s in &q.steps {
        let m = &s.mission;
        if c.zone(&m.zone).is_none() {
            e.push(format!("quête {} / {} : zone inconnue {}", q.id, s.id, m.zone));
        }
        if m.packs.is_empty() && m.ambush.is_none() && m.boss.is_none() {
            e.push(format!("quête {} / {} : mission vide", q.id, s.id));
        }
        for pack in &m.packs {
            for (id, _) in pack {
                enemy(id, &mut e, &s.id);
            }
        }
        if let Some(b) = &m.boss {
            enemy(b, &mut e, &s.id);
        }
        if let Some(a) = &m.ambush {
            for (id, _) in &a.enemies {
                enemy(id, &mut e, &s.id);
            }
            if !(0.0..=1.0).contains(&a.at) {
                e.push(format!("quête {} / {} : embuscade hors du chemin", q.id, s.id));
            }
        }
    }
    if let Some(u) = &q.reward.unique {
        if c.unique(u).is_none() {
            e.push(format!("quête {} : unique inconnu {u}", q.id));
        }
    }
    e
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::content;
    use crate::expedition::layout_mission;

    fn gerald() -> &'static QuestDef {
        content().quest("gerald").expect("quête gerald")
    }

    #[test]
    fn rumeur_puis_etapes_puis_recompense() {
        let c = content();
        let q = gerald();
        let mut p = Profile::new_game(c, "Adeh", 7);
        assert_eq!(status(&p, q), QuestStatus::Locked, "pas de rumeur avant la première expédition");
        assert!(!accept(&mut p, q));
        p.counters.expeditions = 1;
        assert_eq!(status(&p, q), QuestStatus::Offered);
        assert!(accept(&mut p, q));
        assert_eq!(status(&p, q), QuestStatus::Active(0));
        // Une mauvaise étape ne fait rien.
        assert!(complete_step(c, &mut p, q, 1).is_empty());
        let l = complete_step(c, &mut p, q, 0);
        assert!(l.iter().any(|s| s.contains("vieux fortin")));
        assert_eq!(status(&p, q), QuestStatus::Active(1));
        assert_eq!(p.gold, 0);
        let l = complete_step(c, &mut p, q, 1);
        assert_eq!(status(&p, q), QuestStatus::Done);
        assert!(l.iter().any(|s| s.contains("Quête terminée")));
        assert_eq!(p.gold, 150);
        assert_eq!(p.reputation.get("gue_rouge"), Some(&50));
        let hache = p.bag.iter().find(|i| i.unique.as_deref() == Some("eclat_de_gerald")).expect("la hache");
        assert_eq!(hache.quality, Quality::Excellent);
        // Rien ne se donne deux fois.
        assert!(complete_step(c, &mut p, q, 1).is_empty());
        assert_eq!(p.gold, 150);
        assert!(p.has_flag("quete:gerald"));
        // L'avancement survit à la sauvegarde.
        let back = Profile::from_ron(&p.to_ron()).unwrap();
        assert_eq!(status(&back, q), QuestStatus::Done);
    }

    #[test]
    fn recompense_meme_sac_plein() {
        let c = content();
        let q = gerald();
        let mut p = Profile::new_game(c, "Adeh", 8);
        p.counters.expeditions = 3;
        accept(&mut p, q);
        for _ in 0..crate::profile::BAG_SIZE {
            let it = p.new_item("gourdin_rouille", 1);
            p.add_to_bag(c, it);
        }
        complete_step(c, &mut p, q, 0);
        complete_step(c, &mut p, q, 1);
        assert!(p.bag.iter().any(|i| i.unique.as_deref() == Some("eclat_de_gerald")));
    }

    #[test]
    fn missions_dessinees() {
        let c = content();
        let q = gerald();
        for (k, s) in q.steps.iter().enumerate() {
            let z = mission_zone(c, &s.mission).unwrap();
            let mut rng = Rng::new(11 + k as u64);
            let l = layout_mission(&z, &s.mission.packs, &mut rng);
            // Tous les groupes écrits sont posés, dans l'ordre, au complet.
            assert_eq!(l.packs.len(), s.mission.packs.len(), "{}", s.id);
            for ((_, names), want) in l.packs.iter().zip(&s.mission.packs) {
                let n: u32 = want.iter().map(|(_, n)| n).sum();
                assert_eq!(names.len() as u32, n);
            }
            assert_eq!(l.boss.as_ref().map(|b| b.1.clone()), s.mission.boss);
            assert_eq!(l.half, s.mission.size.unwrap());
            if let Some((bp, _)) = &l.boss {
                // Le fortin : des murs autour du boss, une brèche au sud.
                let walls = l.decor.iter().filter(|d| d.0.distance(*bp) < 6.0 && d.1 > 0.0).count();
                assert!(walls >= 8, "{walls} murs");
                let gap = *bp + Vec2::new(0.0, 5.5);
                assert!(l.decor.iter().all(|d| d.1 == 0.0 || d.0.distance(gap) > 1.5), "brèche bouchée");
            }
        }
    }

    #[test]
    fn embuscade_au_milieu_du_chemin() {
        let c = content();
        let q = gerald();
        let s = &q.steps[0];
        let z = mission_zone(c, &s.mission).unwrap();
        let l = layout_mission(&z, &s.mission.packs, &mut Rng::new(3));
        let mut run = MissionRun::new(q, 0, &l).unwrap();
        assert!(!run.quiet());
        // Tant qu'on reste près du départ, rien.
        assert!(run.check_ambush(l.start).is_none());
        // La zone vidée avant l'embuscade ne compte pas.
        run.on_cleared();
        assert!(!run.done);
        let a = run.check_ambush(Vec2::new(0.0, run.trigger_y - 0.1)).expect("embuscade");
        assert!(a.enemies.iter().any(|(id, _)| id == "la_fouine"));
        assert!(run.check_ambush(Vec2::new(0.0, -l.half)).is_none(), "une seule fois");
        run.on_kill("brigand_cogneur");
        assert!(!run.done);
        run.on_cleared();
        assert!(run.done);
        // Les embusqués restent dans la zone.
        let spots = ambush_spots(Vec2::new(l.half - 0.5, 0.0), 4, l.half, &mut Rng::new(1));
        assert_eq!(spots.len(), 4);
        for p in spots {
            assert!(p.x.abs() < l.half && p.y.abs() < l.half);
        }
    }

    #[test]
    fn le_boss_fait_la_mission() {
        let c = content();
        let q = gerald();
        let s = &q.steps[1];
        let z = mission_zone(c, &s.mission).unwrap();
        let l = layout_mission(&z, &s.mission.packs, &mut Rng::new(5));
        let mut run = MissionRun::new(q, 1, &l).unwrap();
        assert!(run.quiet());
        run.on_cleared();
        assert!(!run.done, "il faut Gérald");
        run.on_kill("gerald_le_borgne");
        assert!(run.done);
        assert_eq!(q.step_tier(c, 0), 2);
        assert_eq!(q.step_tier(c, 1), 3);
    }

    #[test]
    fn la_hache_ne_tombe_pas_au_hasard() {
        let c = content();
        let e = c.enemy("gerald_le_borgne").unwrap();
        let mut rng = Rng::new(1);
        let mut uid = 1;
        for _ in 0..300 {
            let (_, items) = crate::loot::enemy_drop(c, &mut rng, e, &mut uid);
            assert!(items.iter().all(|i| i.unique.as_deref() != Some("eclat_de_gerald")));
        }
    }
}
