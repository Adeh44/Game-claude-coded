//! Les corvées : quatre petits jeux tactiles qui paient mal mais forgent.
//!
//! - Caisses : touchez une caisse sur le tapis, puis la charrette.
//! - Écurie : touchez les taches de fumier jusqu'à ce qu'elles partent.
//! - Rythme (bois, forge) : frappez quand le curseur est dans la zone.
//! - Livraisons : touchez les portes dans l'ordre demandé.

use crate::ui::{self, ButtonLook, Fonts, UiAction, UiQueue, font, instant, pal};
use crate::{AppState, Game, Overlay, Panel, Toasts};
use bevy::prelude::*;
use bevy::ui_widgets::Button;
use errance_core::content::{MiniGame, content};
use errance_core::rng::Rng;

const SLOTS: usize = 8;
const AREA_W: f32 = 560.0;
const AREA_H: f32 = 250.0;

#[derive(Clone, Default)]
struct Slot {
    active: bool,
    x: f32,
    y: f32,
    val: f32,
}

#[derive(Resource)]
pub struct ChoreRun {
    id: String,
    game: MiniGame,
    time_left: f32,
    score: f32,
    goal: f32,
    slots: Vec<Slot>,
    carried: Option<usize>,
    marker: f32,
    speed: f32,
    zone: (f32, f32),
    strikes: u32,
    seq: Vec<usize>,
    seq_pos: usize,
    done: bool,
    spawn_t: f32,
    rng: Rng,
    feedback: String,
    rewarded: bool,
}

#[derive(Component)]
struct ChoreSlot(usize);
#[derive(Component)]
struct ChoreSlotText(usize);
#[derive(Component)]
struct ChoreInfo;
#[derive(Component)]
struct RhythmMarker;
#[derive(Component)]
struct RhythmZone;
#[derive(Component)]
struct ChoreResult;

pub struct MiniGamesPlugin;

impl Plugin for MiniGamesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (chore_actions, chore_tick).chain().run_if(in_state(AppState::Town)));
    }
}

pub fn build_chore_list(commands: &mut Commands, body: Entity, fonts: &Fonts, p: &errance_core::profile::Profile) {
    let c = content();
    let labor = p.hero.masteries.level("labeur");
    commands.entity(body).with_children(|b| {
        b.spawn(Node { flex_direction: FlexDirection::Column, row_gap: px(5), width: px(470), ..default() }).with_children(|col| {
            col.spawn(ui::label(fonts, format!("Votre Labeur : niveau {labor}. Les corvées montent cette Voie (talents de PV, d'énergie, d'armure)."), 11.0, pal::DIM));
            for ch in &c.chores {
                let ok = labor >= ch.labor;
                col.spawn((
                    Button,
                    if ok { UiAction::StartChore(ch.id.clone()) } else { UiAction::None },
                    ButtonLook { normal: pal::PANEL_LIGHT, pressed: pal::BUTTON_ON },
                    Node { padding: UiRect::all(px(6)), border_radius: BorderRadius::all(px(5)), flex_direction: FlexDirection::Column, ..default() },
                    BackgroundColor(pal::PANEL_LIGHT),
                    children![
                        (
                            Text::new(if ok { format!("{} — {} po · {} gloire", ch.name, ch.pay, ch.fame) } else { format!("(verrou) {} — Labeur niv. {}", ch.name, ch.labor) }),
                            font(&fonts.bold, 12.5),
                            TextColor(if ok { pal::INK } else { pal::DIM }),
                        ),
                        (Text::new(format!("{} — {}", ch.giver, ch.description)), font(&fonts.regular, 10.5), TextColor(pal::DIM)),
                    ],
                ));
            }
        });
    });
}

pub fn build_chore_game(commands: &mut Commands, body: Entity, fonts: &Fonts, id: &str) {
    let c = content();
    let Some(ch) = c.chore(id) else { return };
    commands.entity(body).with_children(|b| {
        b.spawn(Node { flex_direction: FlexDirection::Column, row_gap: px(6), align_items: AlignItems::Center, flex_grow: 1.0, ..default() }).with_children(|col| {
            col.spawn(ui::bold(fonts, ch.name.clone(), 15.0, pal::GOLD));
            col.spawn((Text::new(""), font(&fonts.bold, 12.0), TextColor(pal::INK), ChoreInfo));
            // Aire de jeu.
            col.spawn((
                Node { width: px(AREA_W), height: px(AREA_H), border: UiRect::all(px(2)), border_radius: BorderRadius::all(px(8)), overflow: Overflow::clip(), ..default() },
                BorderColor::all(pal::EDGE),
                BackgroundColor(match ch.game {
                    MiniGame::Carry => Color::srgb(0.28, 0.33, 0.4),
                    MiniGame::Scrub => Color::srgb(0.6, 0.52, 0.3),
                    MiniGame::Rhythm => Color::srgb(0.2, 0.17, 0.14),
                    MiniGame::Deliver => Color::srgb(0.45, 0.42, 0.38),
                }),
            ))
            .with_children(|area| {
                match ch.game {
                    MiniGame::Carry => {
                        // Tapis roulant + charrette.
                        area.spawn((Node { position_type: PositionType::Absolute, left: px(0), top: px(150), width: px(380), height: px(14), ..default() }, BackgroundColor(Color::srgb(0.35, 0.25, 0.15))));
                        area.spawn(big_target(fonts, "Charrette", 420.0, 120.0, 120.0, 90.0, Color::srgb(0.5, 0.33, 0.18)));
                    }
                    MiniGame::Rhythm => {
                        area.spawn((Node { position_type: PositionType::Absolute, left: px(30), top: px(60), width: px(500), height: px(30), ..default() }, BackgroundColor(Color::srgb(0.1, 0.08, 0.06))));
                        area.spawn((Node { position_type: PositionType::Absolute, left: px(250), top: px(60), width: px(60), height: px(30), ..default() }, BackgroundColor(Color::srgb(0.3, 0.7, 0.3)), RhythmZone));
                        area.spawn((Node { position_type: PositionType::Absolute, left: px(30), top: px(52), width: px(8), height: px(46), ..default() }, BackgroundColor(pal::GOLD), RhythmMarker));
                        area.spawn(big_target(fonts, "FRAPPER !", 200.0, 130.0, 160.0, 90.0, Color::srgb(0.55, 0.2, 0.15)));
                    }
                    _ => {}
                }
                for i in 0..SLOTS {
                    area.spawn((
                        Button,
                        instant(),
                        UiAction::ChoreTap(i as u32),
                        ButtonLook { normal: Color::srgb(0.6, 0.42, 0.2), pressed: pal::BUTTON_ON },
                        Node {
                            position_type: PositionType::Absolute,
                            width: px(64),
                            height: px(56),
                            border: UiRect::all(px(2)),
                            border_radius: BorderRadius::all(px(6)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BorderColor::all(Color::srgba(0.0, 0.0, 0.0, 0.5)),
                        BackgroundColor(Color::srgb(0.6, 0.42, 0.2)),
                        Visibility::Hidden,
                        ChoreSlot(i),
                    ))
                    .with_child((Text::new(""), font(&fonts.bold, 13.0), TextColor(pal::INK), ChoreSlotText(i)));
                }
            });
            col.spawn((Node { flex_direction: FlexDirection::Row, column_gap: px(8), ..default() }, Visibility::Hidden, ChoreResult)).with_children(|r| {
                r.spawn(ui::button(fonts, "Encore une fois", UiAction::ChoreTap(101), px(150), 12.0));
                r.spawn(ui::button(fonts, "Toucher la paie", UiAction::ChoreTap(102), px(150), 12.0));
            });
        });
    });
}

fn big_target(fonts: &Fonts, text: &str, left: f32, top: f32, w: f32, h: f32, color: Color) -> impl Bundle {
    (
        Button,
        instant(),
        UiAction::ChoreTap(100),
        ButtonLook { normal: color, pressed: pal::BUTTON_ON },
        Node {
            position_type: PositionType::Absolute,
            left: px(left),
            top: px(top),
            width: px(w),
            height: px(h),
            border: UiRect::all(px(2)),
            border_radius: BorderRadius::all(px(8)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BorderColor::all(pal::EDGE),
        BackgroundColor(color),
        children![(Text::new(text), font(&fonts.bold, 15.0), TextColor(pal::INK))],
    )
}

fn new_run(id: &str, seed: u64) -> Option<ChoreRun> {
    let ch = content().chore(id)?;
    let d = ch.difficulty;
    let (time, goal) = match ch.game {
        MiniGame::Carry => (25.0, 7.0 * d),
        MiniGame::Scrub => (22.0, 10.0 * d),
        MiniGame::Rhythm => (999.0, 7.0),
        MiniGame::Deliver => (28.0, 4.0 * d),
    };
    let mut run = ChoreRun {
        id: id.to_string(),
        game: ch.game,
        time_left: time,
        score: 0.0,
        goal: goal.round(),
        slots: vec![Slot::default(); SLOTS],
        carried: None,
        marker: 0.0,
        speed: 0.9 * d,
        zone: (0.45, 0.6 - 0.04 * d),
        strikes: 0,
        seq: vec![],
        seq_pos: 0,
        done: false,
        spawn_t: 0.0,
        rng: Rng::new(seed),
        feedback: String::new(),
        rewarded: false,
    };
    if run.game == MiniGame::Deliver {
        // Six portes en deux rangées.
        for i in 0..6 {
            run.slots[i] = Slot { active: true, x: 60.0 + (i % 3) as f32 * 160.0, y: 40.0 + (i / 3) as f32 * 120.0, val: 0.0 };
        }
        new_sequence(&mut run);
    }
    Some(run)
}

fn new_sequence(run: &mut ChoreRun) {
    let n = 3 + (run.score as usize).min(2);
    run.seq = (0..n).map(|_| run.rng.index(6)).collect();
    run.seq_pos = 0;
}

#[allow(clippy::too_many_arguments)]
fn chore_actions(
    mut commands: Commands,
    mut queue: ResMut<UiQueue>,
    mut ov: ResMut<Overlay>,
    run: Option<ResMut<ChoreRun>>,
    mut game: ResMut<Game>,
    mut toasts: ResMut<Toasts>,
    mut menu: ResMut<crate::menus::MenuState>,
) {
    let mut keep = vec![];
    let mut run = run;
    for a in queue.take() {
        match a {
            UiAction::StartChore(id) => {
                let seed = game.0.rng.next_u64();
                if let Some(r) = new_run(&id, seed) {
                    commands.insert_resource(r);
                    ov.0 = Some(Panel::Chore(id));
                }
            }
            UiAction::ChoreTap(i) => {
                let Some(r) = run.as_mut() else { continue };
                match i {
                    101 => {
                        let id = r.id.clone();
                        let seed = game.0.rng.next_u64();
                        if let Some(nr) = new_run(&id, seed) {
                            commands.insert_resource(nr);
                            menu.dirty = true;
                        }
                    }
                    102 => {
                        commands.remove_resource::<ChoreRun>();
                        ov.0 = Some(Panel::Chores);
                    }
                    _ if r.done => {}
                    100 => tap_main(r),
                    i => tap_slot(r, i as usize),
                }
                if r.done && !r.rewarded {
                    r.rewarded = true;
                    reward(r, &mut game, &mut toasts);
                }
            }
            other => keep.push(other),
        }
    }
    queue.0.extend(keep);
}

fn tap_main(r: &mut ChoreRun) {
    match r.game {
        MiniGame::Carry => {
            if let Some(i) = r.carried.take() {
                r.slots[i].active = false;
                r.score += 1.0;
                r.feedback = "Une de plus !".into();
            }
        }
        MiniGame::Rhythm => {
            r.strikes += 1;
            if r.marker >= r.zone.0 && r.marker <= r.zone.1 {
                r.score += 1.0;
                r.feedback = "Clac ! Bûche fendue.".into();
            } else {
                r.feedback = "Raté... la hache rebondit.".into();
            }
            // La zone bouge, ça accélère.
            let w = r.zone.1 - r.zone.0;
            let start = r.rng.range_f32(0.1, 0.9 - w);
            r.zone = (start, start + w);
            r.speed *= 1.06;
            if r.strikes >= 10 {
                r.done = true;
            }
        }
        _ => {}
    }
}

fn tap_slot(r: &mut ChoreRun, i: usize) {
    if i >= r.slots.len() || !r.slots[i].active {
        return;
    }
    match r.game {
        MiniGame::Carry => {
            r.carried = Some(i);
        }
        MiniGame::Scrub => {
            r.slots[i].val -= 1.0;
            if r.slots[i].val <= 0.0 {
                r.slots[i].active = false;
                r.score += 1.0;
            }
        }
        MiniGame::Deliver => {
            if r.seq.get(r.seq_pos) == Some(&i) {
                r.seq_pos += 1;
                if r.seq_pos >= r.seq.len() {
                    r.score += 1.0;
                    r.feedback = "Livré, encore chaud !".into();
                    new_sequence(r);
                }
            } else {
                r.feedback = "Mauvaise porte ! On recommence la tournée.".into();
                r.seq_pos = 0;
                r.time_left -= 2.0;
            }
        }
        _ => {}
    }
}

fn reward(r: &ChoreRun, game: &mut Game, toasts: &mut Toasts) {
    let c = content();
    let Some(ch) = c.chore(&r.id) else { return };
    let ratio = (r.score / r.goal.max(1.0)).clamp(0.0, 1.5);
    let pay = ((ch.pay as f32) * (0.3 + 0.7 * ratio)).round() as u32;
    let fame = ch.fame * (0.4 + 0.6 * ratio);
    game.0.earn(pay);
    game.0.counters.chores += 1;
    let ups = game.0.hero.masteries.add_fame("labeur", fame as f64);
    game.0.hero.fix_loadout(c);
    toasts.push(format!("{} : +{pay} po, +{:.0} gloire de Labeur", ch.giver, fame));
    if let Some(u) = ups {
        toasts.push(format!("* Labeur niveau {} !", u.to));
        if let Some(t) = c.passives.iter().find(|t| t.line == "labeur" && t.mastery > u.from && t.mastery <= u.to) {
            toasts.push(format!("Talent débloqué : {} (Fiche → Talents)", t.name));
        }
    }
}

#[allow(clippy::type_complexity)]
fn chore_tick(
    time: Res<Time>,
    run: Option<ResMut<ChoreRun>>,
    ov: Res<Overlay>,
    mut slots: Query<(&ChoreSlot, &mut Node, &mut Visibility, &mut ButtonLook), (Without<RhythmMarker>, Without<RhythmZone>, Without<ChoreResult>)>,
    mut texts: Query<(&ChoreSlotText, &mut Text), Without<ChoreInfo>>,
    mut info: Query<&mut Text, With<ChoreInfo>>,
    mut marker: Query<&mut Node, (With<RhythmMarker>, Without<ChoreSlot>, Without<RhythmZone>)>,
    mut zone: Query<&mut Node, (With<RhythmZone>, Without<ChoreSlot>, Without<RhythmMarker>)>,
    mut result: Query<&mut Visibility, (With<ChoreResult>, Without<ChoreSlot>)>,
    mut game: ResMut<Game>,
    mut toasts: ResMut<Toasts>,
) {
    let Some(mut r) = run else { return };
    if !matches!(ov.0, Some(Panel::Chore(_))) {
        return;
    }
    let dt = time.delta_secs();
    if !r.done {
        r.time_left -= dt;
        r.spawn_t -= dt;
        match r.game {
            MiniGame::Carry => {
                // Les caisses avancent sur le tapis ; tombent au bout.
                for i in 0..SLOTS {
                    if r.slots[i].active && r.carried != Some(i) {
                        r.slots[i].x += 55.0 * dt;
                        if r.slots[i].x > 330.0 {
                            r.slots[i].active = false;
                            r.feedback = "Une caisse est tombée à l'eau !".into();
                        }
                    }
                }
                if r.spawn_t <= 0.0 {
                    r.spawn_t = 1.8;
                    if let Some(i) = (0..SLOTS).find(|i| !r.slots[*i].active) {
                        r.slots[i] = Slot { active: true, x: -60.0, y: 96.0, val: 0.0 };
                    }
                }
            }
            MiniGame::Scrub => {
                if r.spawn_t <= 0.0 {
                    r.spawn_t = 1.3;
                    if let Some(i) = (0..SLOTS).find(|i| !r.slots[*i].active) {
                        let x = r.rng.range_f32(10.0, AREA_W - 80.0);
                        let y = r.rng.range_f32(10.0, AREA_H - 70.0);
                        r.slots[i] = Slot { active: true, x, y, val: 3.0 };
                    }
                }
            }
            MiniGame::Rhythm => {
                r.marker += r.speed * dt * if (r.strikes % 2) == 0 { 1.0 } else { 1.0 };
                if r.marker > 1.0 {
                    r.marker -= 1.0;
                }
            }
            MiniGame::Deliver => {}
        }
        if r.time_left <= 0.0 && r.game != MiniGame::Rhythm {
            r.done = true;
        }
        if r.done && !r.rewarded {
            r.rewarded = true;
            let rr = &*r;
            reward(rr, &mut game, &mut toasts);
        }
    }
    // Affichage.
    for (s, mut node, mut vis, mut look) in slots.iter_mut() {
        let slot = &r.slots[s.0];
        let show = slot.active && !r.done;
        *vis = if show { Visibility::Visible } else { Visibility::Hidden };
        let (mut x, mut y) = (slot.x, slot.y);
        if r.game == MiniGame::Carry && r.carried == Some(s.0) {
            x = 330.0;
            y = 20.0;
        }
        node.left = px(x);
        node.top = px(y);
        let color = match r.game {
            MiniGame::Carry => Color::srgb(0.62, 0.45, 0.22),
            MiniGame::Scrub => Color::srgb(0.3 + 0.05 * (3.0 - slot.val), 0.22, 0.1),
            MiniGame::Deliver => {
                if r.seq.get(r.seq_pos) == Some(&s.0) { Color::srgb(0.45, 0.32, 0.18) } else { Color::srgb(0.38, 0.27, 0.17) }
            }
            _ => Color::srgb(0.5, 0.5, 0.5),
        };
        look.normal = color;
    }
    for (s, mut t) in texts.iter_mut() {
        let slot = &r.slots[s.0];
        **t = match r.game {
            MiniGame::Carry => if r.carried == Some(s.0) { "Porté".into() } else { "Caisse".into() },
            MiniGame::Scrub => "~".repeat(slot.val.max(0.0) as usize),
            MiniGame::Deliver => format!("Porte {}", s.0 + 1),
            _ => String::new(),
        };
    }
    if let Ok(mut t) = info.single_mut() {
        **t = if r.done {
            format!("Terminé ! Score {:.0} / {:.0}. {}", r.score, r.goal, if r.score >= r.goal { "Le patron hoche la tête." } else { "Le patron soupire." })
        } else {
            match r.game {
                MiniGame::Rhythm => format!("Coups : {}/10 · Réussis : {:.0}   {}", r.strikes, r.score, r.feedback),
                MiniGame::Deliver => {
                    let order: Vec<String> = r.seq.iter().enumerate().map(|(k, d)| if k < r.seq_pos { "ok".into() } else { format!("{}", d + 1) }).collect();
                    format!("Tournée : {}   · {:.0}s · Livraisons {:.0}/{:.0}   {}", order.join(" → "), r.time_left.max(0.0), r.score, r.goal, r.feedback)
                }
                _ => format!("{:.0} s · {:.0} / {:.0}   {}", r.time_left.max(0.0), r.score, r.goal, r.feedback),
            }
        };
    }
    if let Ok(mut n) = marker.single_mut() {
        n.left = px(30.0 + r.marker * 492.0);
    }
    if let Ok(mut n) = zone.single_mut() {
        n.left = px(30.0 + r.zone.0 * 500.0);
        n.width = px((r.zone.1 - r.zone.0) * 500.0);
    }
    if let Ok(mut v) = result.single_mut() {
        *v = if r.done { Visibility::Visible } else { Visibility::Hidden };
    }
}
