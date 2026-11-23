//! Escalade en expédition (Givrecol) : la barre rocheuse en 3D, le bouton
//! « Grimper » au pied des voies, le rappel au sommet, et le panneau de la
//! paroi (prises à toucher, bras, chaleur, pitons).
//!
//! Les règles sont dans `errance_core::climb` ; ici on dessine et on relaie
//! les gestes.

use crate::audio::{PlaySfx, Sfx};
use crate::field::Field;
use crate::models::{Finish, Lib, Painter, spawn_rock};
use crate::ui::{self, ButtonLook, Fonts, UiAction, UiQueue, font, instant, pal};
use crate::{AppState, Game, Overlay, Panel, Toasts};
use bevy::prelude::*;
use bevy::ui_widgets::Button;
use errance_core::climb::{Climb, ClimbEnd, ClimbEvent, ClimbGear, HoldKind, LINE_CLIMB, MAX_FALLS, WALL_WIDTH, climb_fame, generate_route};
use errance_core::combat::Team;
use errance_core::content::content;
use errance_core::math::Vec2 as V2;
use errance_core::stats::Stat;

/// Taille de la fenêtre sur la paroi (px) et mètres visibles.
const WALL_W: f32 = 330.0;
const WALL_H: f32 = 300.0;
const VIEW_M: f32 = 5.6;
const SCALE: f32 = WALL_H / VIEW_M;

/// L'ascension en cours.
#[derive(Resource)]
pub struct ClimbRun {
    pub spot: usize,
    pub climb: Climb,
    /// Pitons plantés (prises).
    pub pitons: Vec<usize>,
    pub feedback: String,
    /// Hauteur affichée en bas de la fenêtre (suit le grimpeur en douceur).
    pub cam: f32,
    pub applied: bool,
    pub result: String,
    /// Démo : le robot grimpe tout seul.
    pub auto: bool,
}

#[derive(Component)]
struct ClimbPrompt;
#[derive(Component)]
struct ClimbPromptText;
#[derive(Component)]
struct HoldButton(usize);
#[derive(Component)]
struct ClimberMark;
#[derive(Component)]
struct PitonMark(usize);
#[derive(Component)]
struct GripFill;
#[derive(Component)]
struct WarmFill;
#[derive(Component)]
struct ClimbInfo;
#[derive(Component)]
struct ClimbFeedback;
#[derive(Component)]
struct PitonButtonText;
#[derive(Component)]
struct ClimbEndRow;
#[derive(Component)]
struct ClimbEndText;

pub struct ClimbPlugin;

impl Plugin for ClimbPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Field), (draw_cliff, build_prompt).after(crate::field::setup_field))
            .add_systems(Update, (climb_actions, climb_tick, update_prompt).chain().run_if(in_state(AppState::Field)))
            .add_systems(OnExit(AppState::Field), |mut c: Commands| c.remove_resource::<ClimbRun>());
    }
}

fn v3(p: V2, y: f32) -> Vec3 {
    Vec3::new(p.x, y, p.y)
}

/// La barre rocheuse, une corde fixe et un repère à chaque voie.
fn draw_cliff(mut commands: Commands, field: Option<Res<Field>>, mut lib: ResMut<Lib>, mut mats: ResMut<Assets<StandardMaterial>>) {
    let Some(field) = field else { return };
    let Some(cl) = &field.cliff else { return };
    let c = content();
    let rock = c.zones.iter().find(|z| z.name == field.zone_name).map(|z| z.rock).unwrap_or((0.55, 0.57, 0.62));
    let mut p = Painter { lib: &mut lib, mats: &mut mats };
    let scope = DespawnOnExit(AppState::Field);
    for (k, (pos, s)) in cl.rocks.iter().enumerate() {
        let rot = k as f32 * 1.37;
        let e = spawn_rock(&mut p, &mut commands, v3(*pos, 0.0), rock, Vec3::new(1.2 * s, 1.25 * s, 1.1 * s), rot);
        // Neige sur le dessus.
        p.ball(&mut commands, e, (0.93, 0.95, 0.98), Finish::Matte, Vec3::new(0.0, 0.62 * s, 0.0), Vec3::new(0.9 * s, 0.3 * s, 0.8 * s));
        commands.entity(e).insert(scope.clone());
    }
    for k in 0..cl.spots.len() {
        let foot = cl.foot(k);
        let x = foot.x;
        // Corde fixe le long de la paroi (face sud) + piquet au sommet.
        let root = commands.spawn((Transform::from_translation(Vec3::new(x, 0.0, cl.y)), Visibility::default(), scope.clone())).id();
        p.cube(&mut commands, root, (0.85, 0.25, 0.2), Finish::Matte, Vec3::new(0.0, 1.3, 0.95), Vec3::new(0.06, 2.6, 0.06));
        p.cube(&mut commands, root, (0.85, 0.25, 0.2), Finish::Matte, Vec3::new(0.0, 2.6, 0.2), Vec3::new(0.06, 0.06, 1.5));
        p.cube(&mut commands, root, (0.4, 0.28, 0.17), Finish::Matte, Vec3::new(0.0, 2.3, -0.6), Vec3::new(0.1, 1.0, 0.1));
        // Repères lumineux au pied et au sommet.
        for at in [foot, cl.top(k)] {
            let m = p.lib.ico.clone();
            let e = commands.spawn((Transform::from_translation(v3(at, 1.7)).with_scale(Vec3::new(0.2, 0.32, 0.2)), Visibility::default(), scope.clone())).id();
            p.part(&mut commands, e, &m, (0.6, 0.85, 1.0), Finish::Glow, Transform::default());
        }
    }
}

fn build_prompt(mut commands: Commands, fonts: Res<Fonts>) {
    commands
        .spawn((
            Button,
            UiAction::ClimbPrompt,
            ButtonLook { normal: Color::srgba(0.2, 0.32, 0.45, 0.92), pressed: pal::BUTTON_ON },
            Node {
                position_type: PositionType::Absolute,
                top: px(64),
                left: percent(50),
                margin: UiRect::left(px(-110)),
                width: px(220),
                padding: UiRect::axes(px(8), px(6)),
                border: UiRect::all(px(1.5)),
                border_radius: BorderRadius::all(px(7)),
                justify_content: JustifyContent::Center,
                ..default()
            },
            BorderColor::all(pal::EDGE),
            BackgroundColor(Color::srgba(0.2, 0.32, 0.45, 0.92)),
            Visibility::Hidden,
            GlobalZIndex(9),
            ClimbPrompt,
            DespawnOnExit(AppState::Field),
        ))
        .with_child((Text::new(""), font(&fonts.bold, 12.0), TextColor(pal::INK), TextLayout::justify(Justify::Center), ClimbPromptText));
}

/// Le bouton n'apparaît qu'au pied d'une voie ou à son sommet.
fn update_prompt(
    field: Option<Res<Field>>,
    ov: Res<Overlay>,
    mut q: Query<&mut Visibility, With<ClimbPrompt>>,
    mut t: Query<&mut Text, With<ClimbPromptText>>,
) {
    let Some(field) = field else { return };
    let mut text = None;
    if let (Some(cl), Some(h), None, None) = (&field.cliff, field.combat.get(field.hero), &ov.0, field.end) {
        if h.alive {
            let c = content();
            let zone = c.zones.iter().find(|z| z.name == field.zone_name);
            if let Some(k) = cl.foot_near(h.pos) {
                let d = zone.and_then(|z| z.climbs.get(cl.spots[k].def));
                text = d.map(|d| format!("Grimper : {} (T{}, {:.0} m)", d.name, d.tier, d.height));
            } else if cl.top_near(h.pos).is_some() {
                text = Some("Descendre en rappel".to_string());
            }
        }
    }
    if let Ok(mut v) = q.single_mut() {
        let want = if text.is_some() { Visibility::Inherited } else { Visibility::Hidden };
        if *v != want {
            *v = want;
        }
    }
    if let (Some(s), Ok(mut tt)) = (text, t.single_mut()) {
        if **tt != s {
            **tt = s;
        }
    }
}

/// Déplace toute la cordée (héros et alliés) à un point.
fn move_party(field: &mut Field, to: V2) {
    let mut k = 0;
    for f in field.combat.fighters.iter_mut() {
        if f.team == Team::Ally && f.alive {
            let off = if f.id == field.hero { V2::new(0.0, 0.0) } else { k += 1; V2::new((k as f32 - 1.5) * 1.1, if to.y < field.cliff.as_ref().map(|c| c.y).unwrap_or(0.0) { -1.0 } else { 1.0 }) };
            f.pos = to + off;
            f.home = f.pos;
            f.move_intent = V2::new(0.0, 0.0);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn climb_actions(
    mut commands: Commands,
    mut queue: ResMut<UiQueue>,
    mut ov: ResMut<Overlay>,
    field: Option<ResMut<Field>>,
    run: Option<ResMut<ClimbRun>>,
    mut game: ResMut<Game>,
    mut toasts: ResMut<Toasts>,
    mut sfx: MessageWriter<PlaySfx>,
) {
    let Some(mut field) = field else { return };
    let mut run = run;
    let mut keep = vec![];
    for a in queue.take() {
        match a {
            UiAction::ClimbPrompt => {
                let Some(cl) = field.cliff.clone() else { continue };
                let Some(h) = field.combat.get(field.hero) else { continue };
                let pos = h.pos;
                if h.in_combat() {
                    toasts.push("Pas en plein combat !".into());
                } else if let Some(k) = cl.foot_near(pos) {
                    let c = content();
                    let Some(def) = c.zones.iter().find(|z| z.name == field.zone_name).and_then(|z| z.climbs.get(cl.spots[k].def)) else { continue };
                    let mut rng = errance_core::rng::Rng::new(cl.spots[k].seed);
                    let route = generate_route(&def.name, def.tier, def.height, def.ice, &mut rng);
                    let gear = ClimbGear::of(c, &game.0.hero);
                    let seed = game.0.rng.next_u64();
                    let auto = std::env::var("ERRANCE_DEMO").map(|d| d.contains("autoclimb")).unwrap_or(false);
                    commands.insert_resource(ClimbRun {
                        spot: k,
                        climb: Climb::new(route, gear, seed),
                        pitons: vec![],
                        feedback: gear.advice().to_string(),
                        cam: -0.7,
                        applied: false,
                        result: String::new(),
                        auto,
                    });
                    ov.0 = Some(Panel::Climb);
                } else if let Some(k) = cl.top_near(pos) {
                    move_party(&mut field, cl.foot(k));
                    toasts.push("Vous descendez en rappel.".into());
                    sfx.write(PlaySfx(Sfx::Swing));
                }
            }
            UiAction::ClimbHold(i) => {
                if let Some(r) = run.as_mut() {
                    if r.climb.reach(i as usize) {
                        sfx.write(PlaySfx(Sfx::Click));
                    } else if !r.climb.busy() {
                        r.feedback = "Trop loin : choisissez une prise cerclée d'or.".into();
                    }
                }
            }
            UiAction::ClimbPiton => {
                if let Some(r) = run.as_mut() {
                    if r.climb.plant() {
                        r.feedback = "Vous plantez un piton... tenez bon.".into();
                        sfx.write(PlaySfx(Sfx::Hit));
                    } else if r.climb.pitons_left == 0 {
                        r.feedback = "Plus de piton !".into();
                    } else if r.climb.hold().kind == HoldKind::Relais {
                        r.feedback = "Le relais vous protège déjà.".into();
                    }
                }
            }
            UiAction::ClimbGiveUp => {
                if let Some(r) = run.as_mut() {
                    if !r.climb.give_up() {
                        r.feedback = "Trop haut pour désescalader : montez ou tombez !".into();
                    }
                }
            }
            UiAction::ClimbDone => {
                if run.as_ref().is_some_and(|r| r.applied) {
                    commands.remove_resource::<ClimbRun>();
                    ov.0 = None;
                }
            }
            other => keep.push(other),
        }
    }
    queue.0.extend(keep);
    // Fin de l'ascension : appliquer une seule fois.
    let Some(r) = run.as_mut() else { return };
    let Some(end) = r.climb.end else { return };
    if r.applied {
        return;
    }
    r.applied = true;
    let c = content();
    let cl = field.cliff.clone();
    let hero = field.hero;
    let dmg = r.climb.damage_frac();
    let mut lost = 0.0;
    if let Some(h) = field.combat.get_mut(hero) {
        let max = h.stats.get(Stat::MaxHp);
        lost = (dmg * max).min(h.hp - 1.0).max(0.0);
        h.hp -= lost;
    }
    let topped = end == ClimbEnd::Top;
    if let (true, Some(cl)) = (topped, &cl) {
        move_party(&mut field, cl.top(r.spot));
    }
    let fame = if end == ClimbEnd::GaveUp { 0.0 } else { climb_fame(&r.climb.route, r.climb.best, topped) };
    if fame > 0.0 {
        if let Some(up) = game.0.hero.masteries.add_fame(LINE_CLIMB, fame) {
            sfx.write(PlaySfx(Sfx::LevelUp));
            toasts.push(format!("* Escalade : niveau {} !", up.to));
            game.0.hero.fix_loadout(c);
        }
    }
    let hurt = if lost >= 1.0 { format!(" Vous perdez {lost:.0} PV.") } else { String::new() };
    r.result = match end {
        ClimbEnd::Top => {
            sfx.write(PlaySfx(Sfx::LevelUp));
            format!("Au sommet ! {} vaincue en {:.0} s. +{fame:.0} gloire d'Escalade.{hurt}", r.climb.route.name, r.climb.time)
        }
        ClimbEnd::Fell { height } => {
            sfx.write(PlaySfx(Sfx::Death));
            format!("Chute de {height:.1} m jusqu'au pied de la voie, sans piton pour vous retenir.{hurt} Leçon de montagne : toujours s'assurer.")
        }
        ClimbEnd::Lowered => format!("{MAX_FALLS} chutes : vos bras ne répondent plus. On vous redescend au bout de la corde.{hurt} +{fame:.0} gloire."),
        ClimbEnd::GaveUp => "Vous désescaladez. La paroi attendra.".to_string(),
    };
}

/// Bouge la paroi, applique le temps, met à jour les jauges.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn climb_tick(
    time: Res<Time>,
    ov: Res<Overlay>,
    run: Option<ResMut<ClimbRun>>,
    mut sfx: MessageWriter<PlaySfx>,
    mut holds: Query<(&HoldButton, &mut Node, &mut Visibility, &mut BorderColor), (Without<ClimberMark>, Without<PitonMark>)>,
    mut climber: Query<(&mut Node, &mut Visibility), (With<ClimberMark>, Without<HoldButton>, Without<PitonMark>)>,
    mut pitons: Query<(&PitonMark, &mut Node, &mut Visibility), (Without<HoldButton>, Without<ClimberMark>)>,
    mut grip: Query<(&mut Node, &mut BackgroundColor), (With<GripFill>, Without<HoldButton>, Without<ClimberMark>, Without<PitonMark>, Without<WarmFill>)>,
    mut warm: Query<(&mut Node, &mut BackgroundColor), (With<WarmFill>, Without<HoldButton>, Without<ClimberMark>, Without<PitonMark>, Without<GripFill>)>,
    mut texts: ParamSet<(
        Query<&mut Text, With<ClimbInfo>>,
        Query<&mut Text, With<ClimbFeedback>>,
        Query<&mut Text, With<PitonButtonText>>,
        Query<&mut Text, With<ClimbEndText>>,
    )>,
    mut end_row: Query<&mut Visibility, (With<ClimbEndRow>, Without<HoldButton>, Without<ClimberMark>, Without<PitonMark>)>,
    mut menu: ResMut<crate::menus::MenuState>,
) {
    let Some(mut r) = run else { return };
    if ov.0 != Some(Panel::Climb) {
        return;
    }
    // Le panneau a pu être dessiné avant que l'ascension n'existe : on le refait.
    if holds.is_empty() {
        menu.dirty = true;
        return;
    }
    let dt = time.delta_secs().min(0.1);
    if r.auto {
        errance_core::climb::bot_step(&mut r.climb);
    }
    let was_cold = r.climb.cold_factor() > 1.0;
    for e in r.climb.tick(dt) {
        match e {
            ClimbEvent::Reached(k) => {
                r.feedback = match r.climb.route.holds[k].kind {
                    HoldKind::Bac => "Un bac ! Restez-y pour secouer les bras.".into(),
                    HoldKind::Reglette => "Réglette : le bout des doigts...".into(),
                    HoldKind::Plat => "Un plat fuyant : ne traînez pas.".into(),
                    HoldKind::Verglas => "Ouf, la glace a tenu.".into(),
                    HoldKind::Relais => "Relais : vous vous vachez. Repos et chaleur.".into(),
                };
            }
            ClimbEvent::Slipped => r.feedback = "La glace cède sous vos doigts !".into(),
            ClimbEvent::Pumped => r.feedback = "Vos bras lâchent !".into(),
            ClimbEvent::Caught { height, .. } => {
                sfx.write(PlaySfx(Sfx::Hit));
                r.feedback = format!("Chute de {height:.1} m... la corde vous retient ! Secouez les bras.");
            }
            ClimbEvent::Grounded { .. } => {}
            ClimbEvent::PitonSet(k) => {
                r.pitons.push(k);
                sfx.write(PlaySfx(Sfx::Shield));
                r.feedback = "Piton planté : une chute s'arrêtera ici.".into();
            }
            ClimbEvent::Top => {}
        }
    }
    if !was_cold && r.climb.cold_factor() > 1.0 {
        r.feedback = "Le vent vous glace : doigts gourds, les bras s'usent plus vite.".into();
    }
    // La fenêtre suit le grimpeur (il reste dans le tiers bas).
    let cl = &r.climb;
    let want = (cl.height() - 1.6).clamp(-0.7, (cl.route.height + 0.8 - VIEW_M).max(-0.7));
    let cam = r.cam + (want - r.cam) * (dt * 5.0).min(1.0);
    r.cam = cam;
    let cl = &r.climb;
    let to_px = |x: f32, y: f32| (x / WALL_WIDTH * WALL_W, WALL_H - (y - cam) * SCALE);
    for (h, mut n, mut v, mut b) in holds.iter_mut() {
        let hold = cl.route.holds[h.0];
        let (x, y) = to_px(hold.x, hold.y);
        let (w, hh) = hold_size(hold.kind);
        n.left = px(x - w / 2.0);
        n.top = px(y - hh / 2.0);
        let inside = y > -20.0 && y < WALL_H + 20.0;
        let want = if inside { Visibility::Inherited } else { Visibility::Hidden };
        if *v != want {
            *v = want;
        }
        let reach = cl.can_reach(h.0) || (cl.busy() && cl.end.is_none() && errance_core::climb::dist(cl.hold(), hold) <= errance_core::climb::REACH && h.0 != cl.at);
        *b = BorderColor::all(if reach { pal::GOLD } else { Color::srgba(0.0, 0.0, 0.0, 0.45) });
    }
    // Le grimpeur : sur sa prise, ou entre deux pendant un mouvement.
    let here = cl.hold();
    let (mut gx, mut gy) = (here.x, here.y);
    if let Some((to, t)) = cl.moving {
        let dest = cl.route.holds[to];
        let f = 1.0 - (t / errance_core::climb::MOVE_TIME).clamp(0.0, 1.0);
        gx += (dest.x - gx) * f;
        gy += (dest.y - gy) * f;
    }
    if let Ok((mut n, mut v)) = climber.single_mut() {
        let (x, y) = to_px(gx, gy);
        n.left = px(x - 13.0);
        n.top = px(y - 30.0);
        *v = Visibility::Inherited;
    }
    for (p, mut n, mut v) in pitons.iter_mut() {
        match r.pitons.get(p.0) {
            Some(&k) => {
                let h = cl.route.holds[k];
                let (x, y) = to_px(h.x, h.y);
                n.left = px(x + 14.0);
                n.top = px(y - 6.0);
                *v = Visibility::Inherited;
            }
            None => *v = Visibility::Hidden,
        }
    }
    if let Ok((mut n, mut bg)) = grip.single_mut() {
        let g = cl.grip.clamp(0.0, 100.0);
        n.width = percent(g);
        bg.0 = if g < 30.0 { pal::RED } else if g < 60.0 { Color::srgb(0.9, 0.7, 0.25) } else { pal::GREEN };
    }
    if let Ok((mut n, mut bg)) = warm.single_mut() {
        let w = cl.warmth.clamp(0.0, 100.0);
        n.width = percent(w);
        bg.0 = if w < 35.0 { Color::srgb(0.6, 0.85, 1.0) } else { Color::srgb(1.0, 0.55, 0.25) };
    }
    if let Ok(mut t) = texts.p0().single_mut() {
        **t = format!("Hauteur {:.1} / {:.0} m · Chutes {} · {:.0} s", cl.height(), cl.route.height, cl.falls, cl.time).replace('.', ",");
    }
    let fb = r.feedback.clone();
    if let Ok(mut t) = texts.p1().single_mut() {
        if **t != fb {
            **t = fb;
        }
    }
    if let Ok(mut t) = texts.p2().single_mut() {
        **t = format!("Planter un piton ({})", cl.pitons_left);
    }
    let res = r.result.clone();
    if let Ok(mut t) = texts.p3().single_mut() {
        if **t != res {
            **t = res;
        }
    }
    if let Ok(mut v) = end_row.single_mut() {
        *v = if r.applied { Visibility::Inherited } else { Visibility::Hidden };
    }
}

fn hold_size(k: HoldKind) -> (f32, f32) {
    match k {
        HoldKind::Bac => (38.0, 30.0),
        HoldKind::Reglette => (34.0, 16.0),
        HoldKind::Plat => (32.0, 26.0),
        HoldKind::Verglas => (30.0, 24.0),
        HoldKind::Relais => (84.0, 18.0),
    }
}

fn hold_color(k: HoldKind) -> Color {
    match k {
        HoldKind::Bac => Color::srgb(0.55, 0.38, 0.22),
        HoldKind::Reglette => Color::srgb(0.5, 0.48, 0.45),
        HoldKind::Plat => Color::srgb(0.38, 0.37, 0.4),
        HoldKind::Verglas => Color::srgb(0.7, 0.88, 1.0),
        HoldKind::Relais => Color::srgb(0.65, 0.52, 0.3),
    }
}

fn gauge(fonts: &Fonts, name: &str, marker: impl Component) -> impl Bundle {
    (
        Node { flex_direction: FlexDirection::Column, row_gap: px(2), ..default() },
        children![
            (Text::new(name), font(&fonts.bold, 11.0), TextColor(pal::DIM)),
            (
                Node { width: px(220), height: px(10), border: UiRect::all(px(1)), border_radius: BorderRadius::all(px(3)), overflow: Overflow::clip(), ..default() },
                BorderColor::all(Color::srgba(0.0, 0.0, 0.0, 0.8)),
                BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.8)),
                children![(Node { position_type: PositionType::Absolute, left: px(0), top: px(0), bottom: px(0), width: percent(100), ..default() }, BackgroundColor(pal::GREEN), marker)],
            ),
        ],
    )
}

/// Le panneau de la paroi.
pub fn build_climb(commands: &mut Commands, body: Entity, fonts: &Fonts, run: Option<&ClimbRun>) {
    let Some(r) = run else { return };
    let route = &r.climb.route;
    commands.entity(body).with_children(|b| {
        // La paroi.
        b.spawn((
            Node { width: px(WALL_W), height: px(WALL_H), border: UiRect::all(px(2)), border_radius: BorderRadius::all(px(6)), overflow: Overflow::clip(), flex_shrink: 0.0, ..default() },
            BorderColor::all(pal::EDGE),
            BackgroundColor(Color::srgb(0.33, 0.35, 0.4)),
        ))
        .with_children(|w| {
            // Quelques veines de roche et de neige pour donner du relief.
            for k in 0..7 {
                let x = (k as f32 * 53.0 + 17.0) % WALL_W;
                w.spawn((
                    Node { position_type: PositionType::Absolute, left: px(x), top: px(0), width: px(14.0 + (k % 3) as f32 * 8.0), bottom: px(0), ..default() },
                    BackgroundColor(if k % 2 == 0 { Color::srgba(0.0, 0.0, 0.0, 0.12) } else { Color::srgba(1.0, 1.0, 1.0, 0.07) }),
                ));
            }
            for (i, h) in route.holds.iter().enumerate() {
                let (wd, hh) = hold_size(h.kind);
                let color = hold_color(h.kind);
                let label = if i == route.top() {
                    "SOMMET"
                } else if i == 0 {
                    "Pied"
                } else if h.kind == HoldKind::Relais {
                    "relais"
                } else {
                    ""
                };
                w.spawn((
                    Button,
                    instant(),
                    UiAction::ClimbHold(i as u32),
                    ButtonLook { normal: color, pressed: pal::BUTTON_ON },
                    Node {
                        position_type: PositionType::Absolute,
                        width: px(wd),
                        height: px(hh),
                        border: UiRect::all(px(2.5)),
                        border_radius: BorderRadius::all(px(if h.kind == HoldKind::Plat || h.kind == HoldKind::Bac { 12.0 } else { 4.0 })),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BorderColor::all(Color::srgba(0.0, 0.0, 0.0, 0.45)),
                    BackgroundColor(color),
                    Visibility::Hidden,
                    HoldButton(i),
                ))
                .with_child((Text::new(label), font(&fonts.bold, 9.0), TextColor(pal::INK)));
            }
            for k in 0..8 {
                w.spawn((
                    Node { position_type: PositionType::Absolute, width: px(10), height: px(12), border_radius: BorderRadius::all(px(2)), ..default() },
                    BackgroundColor(Color::srgb(0.95, 0.5, 0.15)),
                    Visibility::Hidden,
                    PitonMark(k),
                ));
            }
            // Le grimpeur : un petit bonhomme (tête + corps).
            w.spawn((Node { position_type: PositionType::Absolute, width: px(26), height: px(34), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, ..default() }, Visibility::Hidden, ClimberMark))
                .with_children(|m| {
                    m.spawn((Node { width: px(12), height: px(12), border_radius: BorderRadius::all(px(6)), ..default() }, BackgroundColor(Color::srgb(0.95, 0.8, 0.6))));
                    m.spawn((Node { width: px(16), height: px(18), border_radius: BorderRadius::all(px(4)), ..default() }, BackgroundColor(Color::srgb(0.85, 0.25, 0.2))));
                });
        });
        // Colonne d'infos.
        b.spawn(Node { flex_direction: FlexDirection::Column, row_gap: px(6), width: px(250), ..default() }).with_children(|col| {
            col.spawn(ui::bold(fonts, format!("{} — T{}", route.name, route.tier), 14.0, pal::GOLD));
            col.spawn((Text::new(""), font(&fonts.regular, 11.0), TextColor(pal::INK), ClimbInfo));
            col.spawn(gauge(fonts, "Bras", GripFill));
            col.spawn(gauge(fonts, "Chaleur", WarmFill));
            col.spawn((Text::new(""), font(&fonts.bold, 11.0), TextColor(pal::GOLD), ClimbFeedback, Node { min_height: px(30), ..default() }));
            col.spawn((
                Button,
                instant(),
                UiAction::ClimbPiton,
                ButtonLook { normal: Color::srgb(0.55, 0.32, 0.15), pressed: pal::BUTTON_ON },
                Node { width: px(220), padding: UiRect::axes(px(8), px(6)), border: UiRect::all(px(1.5)), border_radius: BorderRadius::all(px(6)), justify_content: JustifyContent::Center, ..default() },
                BorderColor::all(pal::EDGE),
                BackgroundColor(Color::srgb(0.55, 0.32, 0.15)),
                children![(Text::new(""), font(&fonts.bold, 12.0), TextColor(pal::INK), PitonButtonText)],
            ));
            col.spawn(ui::button(fonts, "Redescendre (près du pied)", UiAction::ClimbGiveUp, px(220), 11.0));
            col.spawn(ui::label(
                fonts,
                "Touchez une prise cerclée d'or. Bac : on récupère. Réglette, Plat : les bras s'usent. Verglas : peut lâcher. Relais : repos, chaleur et protection.",
                9.5,
                pal::DIM,
            ));
            col.spawn((Node { flex_direction: FlexDirection::Column, row_gap: px(4), ..default() }, Visibility::Hidden, ClimbEndRow)).with_children(|e| {
                e.spawn((Text::new(""), font(&fonts.bold, 11.5), TextColor(pal::INK), ClimbEndText));
                e.spawn(ui::button(fonts, "Continuer", UiAction::ClimbDone, px(150), 12.0));
            });
        });
    });
}
