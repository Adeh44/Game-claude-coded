//! Simulateur d'équilibrage d'Errance.
//!
//! Lance des centaines de combats sans écran et affiche le taux de victoire
//! de builds types contre des monstres. Usage :
//!   cargo run -p errance_sim --release
//!   cargo run -p errance_sim --release -- 500      (nombre de combats par duel)

use errance_core::character::{Character, enemy_spec};
use errance_core::combat::{Arena, Combat, Controller, Outcome, Team};
use errance_core::content::content;
use errance_core::item::{Item, Quality};
use errance_core::math::Vec2;
use errance_core::rules::{Action, Condition, Rule};
use errance_core::skill::SkillSlot;

struct Build {
    name: &'static str,
    items: Vec<(&'static str, u8)>,
    /// Niveau de maîtrise dans toutes les voies portées.
    mastery: f64,
    quality: Quality,
}

fn make(b: &Build) -> Character {
    let c = content();
    let mut h = Character::new(b.name);
    for l in c.lines.iter() {
        if b.mastery > 0.0 {
            let fame = errance_core::mastery::total_fame_for(b.mastery as u32) + 1.0;
            h.masteries.add_fame(&l.id, fame);
        }
    }
    for (k, (base, tier)) in b.items.iter().enumerate() {
        let mut it = Item::new(k as u64 + 1, base, *tier);
        it.quality = b.quality;
        if let Err((e, _)) = h.equip(c, it) {
            eprintln!("{} : impossible d'équiper {base} T{tier} : {e:?}", b.name);
        }
    }
    // Remplit les talents disponibles (les premiers).
    let ids: Vec<String> = h.unlocked_talents(c).iter().map(|p| p.id.clone()).collect();
    for id in ids {
        h.toggle_talent(c, &id);
    }
    h.rules = vec![
        Rule::new(Condition::SelfHpBelow(0.35), Action::Use(SkillSlot::Potion)),
        Rule::new(Condition::SelfHpBelow(0.5), Action::Use(SkillSlot::Chest)),
        Rule::new(Condition::Always, Action::Use(SkillSlot::Weapon3)),
        Rule::new(Condition::Always, Action::Use(SkillSlot::Weapon1)),
        Rule::new(Condition::Always, Action::Use(SkillSlot::Weapon2)),
        Rule::new(Condition::Always, Action::Use(SkillSlot::Head)),
    ];
    h
}

/// Un duel : le héros (piloté par ses règles, comme un compagnon) contre `n` monstres.
fn duel(h: &Character, enemy: &str, n: usize, seed: u64) -> (Outcome, f32, f32) {
    let c = content();
    let mut combat = Combat::new(seed, Arena::default());
    let spec = h.fighter_spec(c, Team::Ally, Controller::Companion { rules: h.rules.clone() });
    let hid = combat.spawn(spec, Vec2::ZERO);
    combat.leader = Some(hid);
    let e = c.enemy(enemy).expect("ennemi inconnu");
    for k in 0..n {
        let s = enemy_spec(c, e);
        combat.spawn(s, Vec2::new(4.0 + k as f32 * 0.8, (k as f32 - 1.0) * 1.2));
    }
    let dt = 1.0 / 30.0;
    let mut t = 0.0;
    while t < 180.0 {
        combat.tick(dt);
        t += dt;
        let hero_alive = combat.get(hid).map(|f| f.alive).unwrap_or(false);
        let enemies = combat.fighters.iter().any(|f| f.alive && f.team == Team::Enemy);
        if !hero_alive {
            return (Outcome::Defeat, t, 0.0);
        }
        if !enemies {
            let hp = combat.get(hid).unwrap().hp_frac();
            return (Outcome::Victory, t, hp);
        }
    }
    (Outcome::Ongoing, t, 0.0)
}

fn main() {
    let n_fights: u64 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(200);
    let builds = vec![
        Build { name: "Vagabond (mains nues, haillons)", items: vec![("haillons", 1)], mastery: 0.0, quality: Quality::Ordinary },
        Build {
            name: "Débutant (gourdin, haillons)",
            items: vec![("haillons", 1), ("gourdin_rouille", 1)],
            mastery: 0.0,
            quality: Quality::Ordinary,
        },
        Build {
            name: "Recrue T2 (épée, cuir)",
            items: vec![("epee_courte", 2), ("capuche", 2), ("veste_cuir", 2), ("bottes", 2), ("potion_soin", 2)],
            mastery: 6.0,
            quality: Quality::Ordinary,
        },
        Build {
            name: "Mage-tank T3 (givre + plaques)",
            items: vec![("baton_givre", 3), ("casque", 3), ("cotte", 3), ("solerets", 3), ("potion_soin", 3)],
            mastery: 20.0,
            quality: Quality::Good,
        },
        Build {
            name: "Aguerri T5 (épée + bouclier + cuir)",
            items: vec![("epee_longue", 5), ("bouclier_rond", 5), ("capuche", 5), ("brigandine", 5), ("bottes", 5), ("potion_soin", 5)],
            mastery: 40.0,
            quality: Quality::Remarkable,
        },
        Build {
            name: "Expert T6 (bâton sacré + tissu)",
            items: vec![("baton_sacre", 6), ("chapeau_pointu", 6), ("robe", 6), ("sandales", 6), ("potion_soin", 6)],
            mastery: 55.0,
            quality: Quality::Excellent,
        },
        Build {
            name: "Vétéran T8 (hache + plaques)",
            items: vec![("grande_hache", 8), ("casque_cornes", 8), ("plastron", 8), ("solerets", 8), ("cape_guerre", 8)],
            mastery: 100.0,
            quality: Quality::Masterpiece,
        },
    ];
    let duels: Vec<(&str, usize)> = vec![
        ("rat_geant", 1),
        ("rat_geant", 2),
        ("loup_affame", 1),
        ("va_nu_pieds", 1),
        ("sanglier", 1),
        ("loup_gris", 2),
        ("brigand_cogneur", 1),
        ("ours_brun", 1),
        ("gerald_le_borgne", 1),
        ("troll_des_marais", 1),
        ("loup_des_neiges", 2),
        ("golem_de_givre", 1),
        ("bete_de_givrecol", 1),
        ("chevalier_dechu", 2),
        ("roi_sans_couronne", 1),
        ("cultiste_braise", 2),
        ("wyrm_de_cendre", 1),
        ("chevalier_du_neant", 2),
        ("devoreur", 1),
    ];
    println!("Simulateur Errance — {n_fights} combats par duel\n");
    for b in &builds {
        let h = make(b);
        let c = content();
        let s = h.stats(c);
        println!(
            "== {} — PV {:.0}, armure {:.0}, puiss. phys {:.0}, mag {:.0} — rôle : {}",
            b.name,
            s.get(errance_core::stats::Stat::MaxHp),
            s.get(errance_core::stats::Stat::Armor),
            s.get(errance_core::stats::Stat::PhysicalPower),
            s.get(errance_core::stats::Stat::MagicPower),
            h.role(c).title
        );
        let r = h.role(c);
        println!("   (tank {:.1} soin {:.1} dégâts {:.1} contrôle {:.1} soutien {:.1})", r.tank, r.healer, r.damage, r.control, r.support);
        for (enemy, n) in &duels {
            let mut wins = 0;
            let mut time = 0.0;
            let mut hp_left = 0.0;
            for seed in 0..n_fights {
                let (o, t, hp) = duel(&h, enemy, *n, seed * 7919 + 13);
                if o == Outcome::Victory {
                    wins += 1;
                    time += t;
                    hp_left += hp;
                }
            }
            let rate = wins as f32 / n_fights as f32 * 100.0;
            let avg_t = if wins > 0 { time / wins as f32 } else { 0.0 };
            let avg_hp = if wins > 0 { hp_left / wins as f32 * 100.0 } else { 0.0 };
            println!("   vs {n} × {enemy:<18} victoire {rate:5.1} %   durée {avg_t:5.1} s   PV restants {avg_hp:4.0} %");
        }
        println!();
    }
}
