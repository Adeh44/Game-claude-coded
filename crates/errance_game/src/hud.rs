//! L'interface de combat : joystick, boutons de skills, barres de vie.

use crate::field::{Field, MainCamera};
use crate::ui::{ButtonLook, Fonts, UiAction, bold, font, instant, label, pal};
use crate::AppState;
use bevy::input::touch::Touches;
use bevy::prelude::*;
use bevy::ui_widgets::Button;
use errance_core::combat::{FighterId, Team};
use errance_core::rules::Order;
use errance_core::skill::{SkillDef, SkillSlot, Tag};
use errance_core::stats::Stat;

/// Joystick flottant (moitié gauche de l'écran).
#[derive(Resource, Default)]
pub struct Stick {
    pub touch: Option<u64>,
    pub mouse: bool,
    pub origin: Vec2,
    pub cur: Vec2,
    pub dir: Vec2,
}

/// Zoom de la caméra (1 = normal).
#[derive(Resource)]
pub struct Zoom(pub f32);

impl Default for Zoom {
    fn default() -> Self {
        Zoom(0.8)
    }
}

#[derive(Component)]
struct StickBase;
#[derive(Component)]
struct StickKnob;
#[derive(Component)]
struct SkillCd(SkillSlot);
#[derive(Component)]
struct SkillName(SkillSlot);
#[derive(Component)]
struct SkillCdText(SkillSlot);
#[derive(Component)]
struct SkillBtn(SkillSlot);
#[derive(Component)]
struct HeroHp;
#[derive(Component)]
struct HeroShield;
#[derive(Component)]
struct HeroEnergy;
#[derive(Component)]
struct HeroHpText;
#[derive(Component)]
struct HeroStatus;
#[derive(Component)]
struct TargetFrame;
#[derive(Component)]
struct TargetName;
#[derive(Component)]
struct TargetHp;
#[derive(Component)]
struct TargetCast;
#[derive(Component)]
struct PartyFrame(#[allow(dead_code)] FighterId);
#[derive(Component)]
struct PartyHp(FighterId);
#[derive(Component)]
struct WorldBar(FighterId);
#[derive(Component)]
struct WorldBarFill(FighterId);
#[derive(Component)]
struct CastBar;
#[derive(Component)]
struct CastFill;
#[derive(Component)]
struct GoldText;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Stick>()
            .init_resource::<Zoom>()
            .add_systems(OnEnter(AppState::Field), build_hud.after(crate::field::setup_field))
            .add_systems(
                Update,
                (joystick, pinch_zoom, update_hud, update_world_bars, refresh_skill_names).run_if(in_state(AppState::Field)),
            );
    }
}

/// Couleur d'un bouton de skill selon son mot-clé principal.
pub fn skill_color(s: &SkillDef) -> Color {
    let tags = s.all_tags();
    let pick = |t: Tag| tags.contains(&t);
    let c = if pick(Tag::Heal) {
        (0.2, 0.5, 0.25)
    } else if pick(Tag::Taunt) {
        (0.55, 0.18, 0.15)
    } else if pick(Tag::Fire) {
        (0.6, 0.28, 0.08)
    } else if pick(Tag::Frost) {
        (0.15, 0.38, 0.55)
    } else if pick(Tag::Arcane) {
        (0.38, 0.2, 0.55)
    } else if pick(Tag::Holy) {
        (0.55, 0.48, 0.2)
    } else if pick(Tag::Shadow) {
        (0.32, 0.15, 0.38)
    } else if pick(Tag::Nature) || pick(Tag::Poison) {
        (0.22, 0.42, 0.18)
    } else if pick(Tag::Shield) || pick(Tag::Defense) {
        (0.3, 0.36, 0.48)
    } else if pick(Tag::Movement) {
        (0.38, 0.38, 0.32)
    } else {
        (0.36, 0.3, 0.24)
    };
    Color::srgba(c.0, c.1, c.2, 0.92)
}

fn bar(width: f32, height: f32, bg: Color) -> impl Bundle {
    (
        Node {
            width: px(width),
            height: px(height),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(3)),
            overflow: Overflow::clip(),
            ..default()
        },
        BorderColor::all(Color::srgba(0.0, 0.0, 0.0, 0.8)),
        BackgroundColor(bg),
    )
}

fn fill(color: Color) -> Node {
    let _ = color;
    Node { position_type: PositionType::Absolute, left: px(0), top: px(0), bottom: px(0), width: percent(100), ..default() }
}

fn build_hud(mut commands: Commands, fonts: Res<Fonts>, field: Res<Field>) {
    let scope = DespawnOnExit(AppState::Field);
    let hero = field.combat.get(field.hero).cloned();
    // ----- Joystick -----
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: px(110),
            height: px(110),
            border: UiRect::all(px(2)),
            border_radius: BorderRadius::MAX,
            ..default()
        },
        BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.35)),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.18)),
        Visibility::Hidden,
        StickBase,
        GlobalZIndex(10),
        scope.clone(),
    ));
    commands.spawn((
        Node { position_type: PositionType::Absolute, width: px(46), height: px(46), border_radius: BorderRadius::MAX, ..default() },
        BackgroundColor(Color::srgba(1.0, 0.95, 0.85, 0.55)),
        Visibility::Hidden,
        StickKnob,
        GlobalZIndex(11),
        scope.clone(),
    ));
    // ----- Fiche du héros (haut gauche) -----
    let name = hero.as_ref().map(|h| h.name.clone()).unwrap_or_default();
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(8),
                top: px(8),
                flex_direction: FlexDirection::Column,
                row_gap: px(3),
                padding: UiRect::all(px(6)),
                border_radius: BorderRadius::all(px(6)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
            GlobalZIndex(8),
            scope.clone(),
        ))
        .with_children(|p| {
            p.spawn(bold(&fonts, name, 13.0, pal::INK));
            p.spawn(bar(170.0, 13.0, Color::srgba(0.25, 0.05, 0.05, 0.9))).with_children(|b| {
                b.spawn((fill(pal::RED), BackgroundColor(Color::srgb(0.75, 0.15, 0.12)), HeroHp));
                b.spawn((
                    Node { position_type: PositionType::Absolute, left: px(0), bottom: px(0), height: px(3), width: percent(0), ..default() },
                    BackgroundColor(Color::srgba(0.9, 0.92, 1.0, 0.95)),
                    HeroShield,
                ));
                b.spawn((
                    Text::new(""),
                    font(&fonts.bold, 10.0),
                    TextColor(pal::INK),
                    Node { position_type: PositionType::Absolute, left: px(4), top: px(-1), ..default() },
                    HeroHpText,
                ));
            });
            p.spawn(bar(170.0, 7.0, Color::srgba(0.05, 0.08, 0.2, 0.9))).with_children(|b| {
                b.spawn((fill(pal::BLUE), BackgroundColor(Color::srgb(0.25, 0.45, 0.9)), HeroEnergy));
            });
            p.spawn((Text::new(""), font(&fonts.regular, 10.0), TextColor(Color::srgb(0.9, 0.8, 0.6)), HeroStatus));
            p.spawn((Text::new(""), font(&fonts.bold, 10.0), TextColor(pal::GOLD), GoldText));
        });
    // ----- Compagnons (sous la fiche) -----
    let party: Vec<_> = field.combat.fighters.iter().filter(|f| f.team == Team::Ally && f.id != field.hero).cloned().collect();
    if !party.is_empty() {
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(8),
                    top: px(92),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(3),
                    ..default()
                },
                GlobalZIndex(8),
                scope.clone(),
            ))
            .with_children(|p| {
                for f in &party {
                    p.spawn((Node { flex_direction: FlexDirection::Column, ..default() }, PartyFrame(f.id))).with_children(|r| {
                        r.spawn(label(&fonts, f.name.clone(), 10.0, pal::INK));
                        r.spawn(bar(110.0, 7.0, Color::srgba(0.25, 0.05, 0.05, 0.9))).with_children(|b| {
                            b.spawn((fill(pal::GREEN), BackgroundColor(Color::srgb(0.3, 0.7, 0.3)), PartyHp(f.id)));
                        });
                    });
                }
                // Ordres rapides.
                p.spawn(Node { flex_direction: FlexDirection::Row, column_gap: px(3), margin: UiRect::top(px(3)), ..default() }).with_children(|r| {
                    for o in [Order::Assist, Order::Defensive, Order::Regroup] {
                        r.spawn(small_button(&fonts, o.label(), UiAction::SetOrder(o)));
                    }
                });
            });
    }
    // ----- Cible (haut centre) -----
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(8),
                left: percent(50),
                margin: UiRect::left(px(-95)),
                width: px(190),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(2),
                padding: UiRect::all(px(4)),
                border_radius: BorderRadius::all(px(6)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
            Visibility::Hidden,
            TargetFrame,
            GlobalZIndex(8),
            scope.clone(),
        ))
        .with_children(|p| {
            p.spawn((Text::new(""), font(&fonts.bold, 12.0), TextColor(pal::INK), TargetName));
            p.spawn(bar(180.0, 10.0, Color::srgba(0.25, 0.05, 0.05, 0.9))).with_children(|b| {
                b.spawn((fill(pal::RED), BackgroundColor(Color::srgb(0.8, 0.2, 0.15)), TargetHp));
            });
            p.spawn((Text::new(""), font(&fonts.regular, 10.0), TextColor(Color::srgb(1.0, 0.7, 0.4)), TargetCast));
        });
    // ----- Barre d'incantation du héros -----
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: px(150),
                left: percent(50),
                margin: UiRect::left(px(-70)),
                ..default()
            },
            Visibility::Hidden,
            CastBar,
            GlobalZIndex(8),
            scope.clone(),
        ))
        .with_children(|p| {
            p.spawn(bar(140.0, 8.0, Color::srgba(0.1, 0.1, 0.1, 0.8))).with_children(|b| {
                b.spawn((fill(pal::GOLD), BackgroundColor(Color::srgb(0.95, 0.8, 0.35)), CastFill));
            });
        });
    // ----- Boutons du haut à droite -----
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(8),
                top: px(8),
                flex_direction: FlexDirection::Row,
                column_gap: px(5),
                ..default()
            },
            GlobalZIndex(8),
            scope.clone(),
        ))
        .with_children(|p| {
            p.spawn(small_button(&fonts, "Sac", UiAction::OpenBag));
            p.spawn(small_button(&fonts, "Fiche", UiAction::OpenCharacter));
            p.spawn(small_button(&fonts, "Rentrer", UiAction::LeaveField));
        });
    // ----- Skills (bas droite) -----
    let big = |slot: SkillSlot| -> f32 {
        match slot {
            SkillSlot::Weapon3 => 70.0,
            SkillSlot::Weapon1 | SkillSlot::Weapon2 => 60.0,
            _ => 48.0,
        }
    };
    // Positions (right, bottom) depuis le coin bas droit.
    let layout: [(SkillSlot, f32, f32); 7] = [
        (SkillSlot::Weapon3, 14.0, 14.0),
        (SkillSlot::Weapon1, 94.0, 10.0),
        (SkillSlot::Weapon2, 24.0, 94.0),
        (SkillSlot::Head, 164.0, 8.0),
        (SkillSlot::Chest, 96.0, 82.0),
        (SkillSlot::Feet, 34.0, 166.0),
        (SkillSlot::Potion, 228.0, 8.0),
    ];
    for (slot, right, bottom) in layout {
        let size = big(slot);
        let skill = hero.as_ref().and_then(|h| h.skill(slot).cloned());
        let color = skill.as_ref().map(skill_color).unwrap_or(Color::srgba(0.15, 0.13, 0.1, 0.6));
        commands
            .spawn((
                Button,
                instant(),
                UiAction::Skill(slot),
                ButtonLook { normal: color, pressed: pal::BUTTON_ON },
                SkillBtn(slot),
                Node {
                    position_type: PositionType::Absolute,
                    right: px(right),
                    bottom: px(bottom),
                    width: px(size),
                    height: px(size),
                    border: UiRect::all(px(2)),
                    border_radius: BorderRadius::MAX,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    overflow: Overflow::clip(),
                    ..default()
                },
                BorderColor::all(Color::srgba(0.95, 0.85, 0.6, 0.7)),
                BackgroundColor(color),
                GlobalZIndex(9),
                scope.clone(),
            ))
            .with_children(|b| {
                b.spawn((
                    Node { position_type: PositionType::Absolute, left: px(0), right: px(0), top: px(0), bottom: px(0), border_radius: BorderRadius::MAX, justify_content: JustifyContent::Center, align_items: AlignItems::FlexEnd, ..default() },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                    SkillCd(slot),
                ))
                .with_child((Text::new(""), font(&fonts.bold, 10.0), TextColor(pal::GOLD), SkillCdText(slot)));
                b.spawn((
                    Text::new(skill.as_ref().map(|s| short(&s.name)).unwrap_or_default()),
                    font(&fonts.bold, if size > 55.0 { 11.0 } else { 9.5 }),
                    TextColor(pal::INK),
                    TextShadow { offset: Vec2::new(1.0, 1.0), color: Color::srgba(0.0, 0.0, 0.0, 0.85) },
                    TextLayout::justify(Justify::Center),
                    Node { max_width: px(size - 6.0), ..default() },
                    SkillName(slot),
                ));
            });
    }
    // Changer de cible.
    commands
        .spawn((
            Node { position_type: PositionType::Absolute, right: px(178), bottom: px(70), ..default() },
            GlobalZIndex(9),
            scope.clone(),
        ))
        .with_child(small_button(&fonts, "Cible", UiAction::CycleTarget));
    // Barres de vie au-dessus des têtes.
    for f in &field.combat.fighters {
        if f.id == field.hero {
            continue;
        }
        let color = if f.team == Team::Enemy { Color::srgb(0.85, 0.2, 0.15) } else { Color::srgb(0.3, 0.75, 0.3) };
        let w = if f.tier >= 3 && f.radius > 0.55 { 60.0 } else { 40.0 };
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: px(w),
                    height: px(5),
                    border: UiRect::all(px(1)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BorderColor::all(Color::srgba(0.0, 0.0, 0.0, 0.8)),
                BackgroundColor(Color::srgba(0.1, 0.0, 0.0, 0.7)),
                Visibility::Hidden,
                WorldBar(f.id),
                GlobalZIndex(4),
                scope.clone(),
            ))
            .with_children(|b| {
                b.spawn((fill(color), BackgroundColor(color), WorldBarFill(f.id)));
            });
    }
}

fn small_button(fonts: &Fonts, text: &str, action: UiAction) -> impl Bundle {
    (
        Button,
        action,
        ButtonLook { normal: Color::srgba(0.18, 0.14, 0.1, 0.85), pressed: pal::BUTTON_ON },
        Node {
            padding: UiRect::axes(px(8), px(4)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(5)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BorderColor::all(pal::EDGE),
        BackgroundColor(Color::srgba(0.18, 0.14, 0.1, 0.85)),
        children![(Text::new(text), font(&fonts.bold, 11.0), TextColor(pal::INK))],
    )
}

pub fn short(name: &str) -> String {
    if name.chars().count() <= 12 {
        name.to_string()
    } else {
        // Garde les mots principaux.
        let words: Vec<&str> = name.split(' ').filter(|w| w.len() > 2).collect();
        let s = words.join(" ");
        if s.chars().count() <= 14 { s } else { s.chars().take(12).collect::<String>() + "." }
    }
}

fn joystick(
    touches: Res<Touches>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut stick: ResMut<Stick>,
    scale: Res<UiScale>,
    mut base: Query<(&mut Node, &mut Visibility), (With<StickBase>, Without<StickKnob>)>,
    mut knob: Query<(&mut Node, &mut Visibility), (With<StickKnob>, Without<StickBase>)>,
    overlay: Res<crate::Overlay>,
) {
    let Ok(w) = windows.single() else { return };
    let width = w.width();
    let height = w.height();
    let in_zone = |p: Vec2| p.x < width * 0.45 && p.y > height * 0.28;
    if overlay.0.is_some() {
        stick.touch = None;
        stick.mouse = false;
    }
    // Nouveau doigt.
    if stick.touch.is_none() && !stick.mouse && overlay.0.is_none() {
        for t in touches.iter_just_pressed() {
            if in_zone(t.position()) {
                stick.touch = Some(t.id());
                stick.origin = t.position();
                stick.cur = t.position();
                break;
            }
        }
        if stick.touch.is_none() && mouse.just_pressed(MouseButton::Left) {
            if let Some(p) = w.cursor_position() {
                if in_zone(p) {
                    stick.mouse = true;
                    stick.origin = p;
                    stick.cur = p;
                }
            }
        }
    }
    if let Some(id) = stick.touch {
        match touches.get_pressed(id) {
            Some(t) => stick.cur = t.position(),
            None => stick.touch = None,
        }
    }
    if stick.mouse {
        if mouse.pressed(MouseButton::Left) {
            if let Some(p) = w.cursor_position() {
                stick.cur = p;
            }
        } else {
            stick.mouse = false;
        }
    }
    let active = stick.touch.is_some() || stick.mouse;
    let radius = 50.0 * scale.0;
    if active {
        let d = stick.cur - stick.origin;
        // Le joystick suit le doigt s'il s'éloigne trop.
        if d.length() > radius {
            let o = stick.cur - d.normalize() * radius;
            stick.origin = o;
        }
        let d = stick.cur - stick.origin;
        stick.dir = if d.length() > 6.0 { d / radius } else { Vec2::ZERO };
    } else {
        stick.dir = Vec2::ZERO;
    }
    let s = scale.0;
    if let Ok((mut n, mut v)) = base.single_mut() {
        *v = if active { Visibility::Visible } else { Visibility::Hidden };
        n.left = px(stick.origin.x / s - 55.0);
        n.top = px(stick.origin.y / s - 55.0);
    }
    if let Ok((mut n, mut v)) = knob.single_mut() {
        *v = if active { Visibility::Visible } else { Visibility::Hidden };
        let k = stick.origin + stick.dir * radius;
        n.left = px(k.x / s - 23.0);
        n.top = px(k.y / s - 23.0);
    }
}

/// Pincer pour zoomer (deux doigts hors joystick), molette sur PC.
fn pinch_zoom(touches: Res<Touches>, mut zoom: ResMut<Zoom>, stick: Res<Stick>, mut last: Local<Option<f32>>, mut wheel: MessageReader<bevy::input::mouse::MouseWheel>) {
    let pts: Vec<Vec2> = touches.iter().filter(|t| Some(t.id()) != stick.touch).map(|t| t.position()).collect();
    if pts.len() == 2 {
        let d = pts[0].distance(pts[1]);
        if let Some(l) = *last {
            zoom.0 = (zoom.0 * l / d.max(1.0)).clamp(0.6, 1.5);
        }
        *last = Some(d);
    } else {
        *last = None;
    }
    for w in wheel.read() {
        zoom.0 = (zoom.0 * (1.0 - w.y * 0.08)).clamp(0.6, 1.5);
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn update_hud(
    field: Res<Field>,
    game: Res<crate::Game>,
    mut cds: Query<(&SkillCd, &mut BackgroundColor)>,
    mut cd_texts: Query<(&SkillCdText, &mut Text), (Without<HeroHpText>, Without<HeroStatus>, Without<TargetName>, Without<TargetCast>, Without<GoldText>, Without<SkillName>)>,
    mut q: ParamSet<(
        Query<(&SkillCd, &mut Node)>,
        Query<&mut Node, With<HeroHp>>,
        Query<&mut Node, With<HeroEnergy>>,
        Query<&mut Node, With<HeroShield>>,
        Query<&mut Node, With<TargetHp>>,
        Query<(&PartyHp, &mut Node)>,
        Query<&mut Node, With<CastFill>>,
    )>,
    mut texts: ParamSet<(
        Query<&mut Text, With<HeroHpText>>,
        Query<&mut Text, With<HeroStatus>>,
        Query<&mut Text, With<TargetName>>,
        Query<&mut Text, With<TargetCast>>,
        Query<&mut Text, With<GoldText>>,
    )>,
    mut vis: ParamSet<(Query<&mut Visibility, With<TargetFrame>>, Query<&mut Visibility, With<CastBar>>)>,
    mut btns: Query<(&SkillBtn, &mut BorderColor)>,
) {
    let Some(h) = field.combat.get(field.hero) else { return };
    let _ = &mut q;
    for (cd, mut bg) in cds.iter_mut() {
        let frac = h.cooldown_frac(cd.0);
        let has = h.skill(cd.0).is_some();
        let ready = h.skill(cd.0).map(|s| h.energy >= s.cost).unwrap_or(false);
        let a = if !has { 0.0 } else if frac > 0.0 { 0.35 + 0.35 * frac } else if !ready { 0.55 } else { 0.0 };
        bg.0 = Color::srgba(0.0, 0.0, 0.0, a);
    }
    for (ct, mut t) in cd_texts.iter_mut() {
        let left = h.cooldown(ct.0);
        let want = if left > 0.05 && h.skill(ct.0).is_some() { format!("{:.0}", left.ceil()) } else { String::new() };
        if **t != want {
            **t = want;
        }
    }
    for (b, mut border) in btns.iter_mut() {
        let ok = h.is_ready(b.0);
        *border = BorderColor::all(if ok { Color::srgba(0.95, 0.85, 0.6, 0.9) } else { Color::srgba(0.3, 0.3, 0.3, 0.6) });
    }
    if let Ok(mut n) = q.p1().single_mut() {
        n.width = percent(h.hp_frac() * 100.0);
    }
    if let Ok(mut n) = q.p2().single_mut() {
        n.width = percent(h.energy_frac() * 100.0);
    }
    if let Ok(mut n) = q.p3().single_mut() {
        let max = h.stats.get(Stat::MaxHp).max(1.0);
        n.width = percent((h.shield_total() / max * 100.0).min(100.0));
    }
    if let Ok(mut t) = texts.p0().single_mut() {
        **t = format!("{:.0} / {:.0}", h.hp.max(0.0), h.stats.get(Stat::MaxHp));
    }
    if let Ok(mut t) = texts.p1().single_mut() {
        let st: Vec<String> = h.statuses.iter().map(|s| if s.stacks > 1 { format!("{}×{}", s.kind.label(), s.stacks) } else { s.kind.label().to_string() }).collect();
        **t = st.join(" · ");
    }
    if let Ok(mut t) = texts.p4().single_mut() {
        let carried = field.gold;
        **t = format!("{} po  (+{} en poche)  · butin : {}", game.0.gold, carried, field.loot.len());
    }
    // Cible.
    let target = h.target.and_then(|t| field.combat.get(t)).filter(|t| t.alive);
    if let Ok(mut v) = vis.p0().single_mut() {
        *v = if target.is_some() { Visibility::Visible } else { Visibility::Hidden };
    }
    if let Some(t) = target {
        if let Ok(mut n) = q.p4().single_mut() {
            n.width = percent(t.hp_frac() * 100.0);
        }
        if let Ok(mut tx) = texts.p2().single_mut() {
            **tx = format!("{}  ·  T{}  ·  {:.0} PV", t.name, t.tier, t.hp.max(0.0));
        }
        if let Ok(mut tx) = texts.p3().single_mut() {
            **tx = t
                .cast
                .as_ref()
                .and_then(|c| t.skill(c.slot))
                .map(|s| format!("Incante : {}", s.name))
                .unwrap_or_default();
        }
    }
    for (p, mut n) in q.p5().iter_mut() {
        if let Some(f) = field.combat.get(p.0) {
            n.width = percent(if f.alive { f.hp_frac() * 100.0 } else { 0.0 });
        }
    }
    // Incantation.
    if let Ok(mut v) = vis.p1().single_mut() {
        *v = if h.cast.is_some() { Visibility::Visible } else { Visibility::Hidden };
    }
    if let Some(c) = &h.cast {
        if let Ok(mut n) = q.p6().single_mut() {
            n.width = percent((1.0 - c.remaining / c.total.max(0.01)) * 100.0);
        }
    }
}

fn refresh_skill_names(field: Res<Field>, mut names: Query<(&SkillName, &mut Text)>, mut btns: Query<(&SkillBtn, &mut ButtonLook, &mut Visibility)>) {
    if !field.is_changed() {
        return;
    }
    let Some(h) = field.combat.get(field.hero) else { return };
    for (n, mut t) in names.iter_mut() {
        let want = h.skill(n.0).map(|s| short(&s.name)).unwrap_or_default();
        if **t != want {
            **t = want;
        }
    }
    for (b, mut look, mut vis) in btns.iter_mut() {
        let want = if h.skill(b.0).is_some() { Visibility::Inherited } else { Visibility::Hidden };
        if *vis != want {
            *vis = want;
        }
        let c = h.skill(b.0).map(skill_color).unwrap_or(Color::srgba(0.15, 0.13, 0.1, 0.6));
        if look.normal != c {
            look.normal = c;
        }
    }
}

fn update_world_bars(
    field: Res<Field>,
    cam: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    scale: Res<UiScale>,
    mut bars: Query<(&WorldBar, &mut Node, &mut Visibility)>,
    mut fills: Query<(&WorldBarFill, &mut Node), Without<WorldBar>>,
) {
    let Ok((camera, ct)) = cam.single() else { return };
    let hero_target = field.combat.get(field.hero).and_then(|h| h.target);
    for (b, mut n, mut v) in bars.iter_mut() {
        let Some(f) = field.combat.get(b.0) else { continue };
        let show = f.alive && (f.hp_frac() < 0.999 || hero_target == Some(f.id) || !f.threat.is_empty() || f.team == Team::Ally);
        *v = if show { Visibility::Visible } else { Visibility::Hidden };
        if !show {
            continue;
        }
        let head = Vec3::new(f.pos.x, 1.9 * f.look.scale + 0.2, f.pos.y);
        if let Ok(p) = camera.world_to_viewport(ct, head) {
            let w = if f.tier >= 3 && f.radius > 0.55 { 60.0 } else { 40.0 };
            n.left = px(p.x / scale.0 - w / 2.0);
            n.top = px(p.y / scale.0 - 6.0);
        }
    }
    for (fl, mut n) in fills.iter_mut() {
        if let Some(f) = field.combat.get(fl.0) {
            n.width = percent(f.hp_frac() * 100.0);
        }
    }
}
