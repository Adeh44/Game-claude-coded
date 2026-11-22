//! Briques d'interface : polices, couleurs, boutons, panneaux.
//!
//! Chaque bouton porte un `UiAction`. Un seul observateur range les clics
//! dans `UiQueue` ; chaque écran lit la file et réagit.

use bevy::prelude::*;
use bevy::ui::Pressed;
use bevy::ui_widgets::{Activate, ActivateOnPress, Button};
use errance_core::item::EquipSlot;
use errance_core::rules::Order;
use errance_core::skill::SkillSlot;

#[derive(Resource, Clone)]
pub struct Fonts {
    pub regular: Handle<Font>,
    pub bold: Handle<Font>,
    pub title: Handle<Font>,
}

impl FromWorld for Fonts {
    fn from_world(world: &mut World) -> Self {
        let mut fonts = world.resource_mut::<Assets<Font>>();
        Fonts {
            regular: fonts.add(Font::from_bytes(include_bytes!("../assets/fonts/AlegreyaSans-Regular.ttf").to_vec())),
            bold: fonts.add(Font::from_bytes(include_bytes!("../assets/fonts/AlegreyaSans-Bold.ttf").to_vec())),
            title: fonts.add(Font::from_bytes(include_bytes!("../assets/fonts/Cinzel.ttf").to_vec())),
        }
    }
}

/// Palette de l'interface : parchemin et bois sombre.
pub mod pal {
    use bevy::prelude::Color;
    pub const INK: Color = Color::srgb(0.95, 0.92, 0.84);
    pub const DIM: Color = Color::srgb(0.7, 0.66, 0.58);
    pub const GOLD: Color = Color::srgb(1.0, 0.82, 0.35);
    pub const RED: Color = Color::srgb(0.9, 0.25, 0.22);
    pub const GREEN: Color = Color::srgb(0.4, 0.85, 0.4);
    pub const BLUE: Color = Color::srgb(0.35, 0.6, 1.0);
    pub const PANEL: Color = Color::srgba(0.09, 0.075, 0.06, 0.985);
    pub const PANEL_LIGHT: Color = Color::srgba(0.17, 0.14, 0.11, 0.95);
    pub const BUTTON: Color = Color::srgba(0.26, 0.2, 0.14, 0.95);
    pub const BUTTON_ON: Color = Color::srgba(0.5, 0.38, 0.2, 0.98);
    pub const EDGE: Color = Color::srgb(0.55, 0.43, 0.25);
    pub const SHADOW: Color = Color::srgba(0.0, 0.0, 0.0, 0.55);
}

#[derive(Component, Clone, Debug, PartialEq)]
pub enum UiAction {
    // Titre
    NewGame,
    Continue,
    // Combat
    Skill(SkillSlot),
    CycleTarget,
    SetOrder(Order),
    LeaveField,
    // Panneaux
    OpenBag,
    OpenAtlas,
    OpenCharacter,
    Close,
    Tab(u8),
    SelectBag(usize),
    SelectSlot(EquipSlot),
    Equip(usize),
    Unequip(EquipSlot),
    Sell(usize),
    ChooseSkill(SkillSlot, String),
    PickSkillSlot(SkillSlot),
    ToggleTalent(String),
    SelectLine(String),
    // Ville
    Building(String),
    StartChore(String),
    ChoreTap(u32),
    Buy(usize),
    Travel(String),
    Hire(u32),
    Dismiss(u32),
    Recruit(u32),
    InspectNpc(u32),
    JoinCompany(u32),
    OpenCompany(u32),
    GiveItem(u32, usize),
    TakeItem(u32, EquipSlot),
    NpcPickSkill(u32, SkillSlot),
    NpcChooseSkill(u32, SkillSlot, String),
    NpcToggleTalent(u32, String),
    RuleUp(u32, usize),
    RuleDown(u32, usize),
    RuleToggle(u32, usize),
    RuleDelete(u32, usize),
    RuleCycle(u32, usize),
    RuleAdd(u32, usize),
    Rest,
    Save,
    ToggleMusic,
    ToggleSfx,
    FoundCompany,
    /// Rien (bouton décoratif / désactivé).
    None,
}

#[derive(Resource, Default)]
pub struct UiQueue(pub Vec<UiAction>);

impl UiQueue {
    pub fn take(&mut self) -> Vec<UiAction> {
        std::mem::take(&mut self.0)
    }
}

/// Couleurs normal / pressé d'un bouton.
#[derive(Component, Clone, Copy)]
pub struct ButtonLook {
    pub normal: Color,
    pub pressed: Color,
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Fonts>()
            .init_resource::<UiQueue>()
            .add_observer(on_activate)
            .add_observer(on_scroll)
            .add_systems(Update, (button_feedback, ui_scale, send_scroll_events))
            .add_systems(PostUpdate, |mut q: ResMut<UiQueue>| q.0.clear());
    }
}

fn on_activate(ev: On<Activate>, q: Query<&UiAction>, mut queue: ResMut<UiQueue>, mut sfx: MessageWriter<crate::audio::PlaySfx>) {
    if let Ok(a) = q.get(ev.entity) {
        if *a != UiAction::None {
            if !matches!(a, UiAction::Skill(_) | UiAction::ChoreTap(_)) {
                sfx.write(crate::audio::PlaySfx(crate::audio::Sfx::Click));
            }
            queue.0.push(a.clone());
        }
    }
}

fn button_feedback(mut q: Query<(&ButtonLook, Has<Pressed>, &mut BackgroundColor)>) {
    for (look, pressed, mut bg) in q.iter_mut() {
        let want = if pressed { look.pressed } else { look.normal };
        if bg.0 != want {
            bg.0 = want;
        }
    }
}

/// Hauteur de référence de l'interface : tout est dessiné pour 420 px de
/// haut puis agrandi/réduit selon l'écran.
pub const REF_HEIGHT: f32 = 420.0;

fn ui_scale(windows: Query<&Window>, mut scale: ResMut<UiScale>) {
    let Ok(w) = windows.single() else { return };
    let s = (w.height() / REF_HEIGHT).clamp(0.6, 3.0);
    if (scale.0 - s).abs() > 0.01 {
        scale.0 = s;
    }
}

pub fn font(f: &Handle<Font>, size: f32) -> TextFont {
    TextFont { font: f.clone().into(), font_size: FontSize::Px(size), ..default() }
}

pub fn label(fonts: &Fonts, s: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (Text::new(s), font(&fonts.regular, size), TextColor(color))
}

pub fn bold(fonts: &Fonts, s: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (Text::new(s), font(&fonts.bold, size), TextColor(color))
}

pub fn title(fonts: &Fonts, s: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (Text::new(s), font(&fonts.title, size), TextColor(color))
}

/// Bouton texte standard.
pub fn button(fonts: &Fonts, text: impl Into<String>, action: UiAction, width: Val, size: f32) -> impl Bundle {
    button_colored(fonts, text, action, width, size, pal::BUTTON)
}

pub fn button_colored(fonts: &Fonts, text: impl Into<String>, action: UiAction, width: Val, size: f32, color: Color) -> impl Bundle {
    (
        Button,
        action,
        ButtonLook { normal: color, pressed: pal::BUTTON_ON },
        Node {
            width,
            min_height: px(size * 2.0),
            padding: UiRect::axes(px(8), px(4)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(px(1.5)),
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        BorderColor::all(pal::EDGE),
        BackgroundColor(color),
        children![(Text::new(text), font(&fonts.bold, size), TextColor(pal::INK), TextLayout::justify(Justify::Center))],
    )
}

/// Bouton qui se déclenche dès l'appui (skills de combat).
pub fn instant() -> ActivateOnPress {
    ActivateOnPress
}

/// Panneau plein écran semi-opaque (menus).
pub fn overlay() -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            right: px(0),
            top: px(0),
            bottom: px(0),
            padding: UiRect::all(px(10)),
            flex_direction: FlexDirection::Column,
            ..default()
        },
        BackgroundColor(pal::PANEL),
        GlobalZIndex(50),
    )
}

pub fn row() -> Node {
    Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: px(6), ..default() }
}

pub fn col() -> Node {
    Node { flex_direction: FlexDirection::Column, row_gap: px(4), ..default() }
}

pub fn rgb(c: (f32, f32, f32)) -> Color {
    Color::srgb(c.0, c.1, c.2)
}

/// Défilement : molette (PC) ou glisser du doigt (téléphone).
#[derive(EntityEvent, Debug)]
#[entity_event(propagate, auto_propagate)]
struct Scroll {
    entity: Entity,
    delta: Vec2,
}

fn send_scroll_events(
    mut wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    touches: Res<bevy::input::touch::Touches>,
    hover_map: Res<bevy::picking::hover::HoverMap>,
    scale: Res<UiScale>,
    mut commands: Commands,
) {
    use bevy::picking::pointer::PointerId;
    for w in wheel.read() {
        let mut delta = -Vec2::new(w.x, w.y);
        if w.unit == bevy::input::mouse::MouseScrollUnit::Line {
            delta *= 28.0;
        }
        if let Some(map) = hover_map.get(&PointerId::Mouse) {
            for entity in map.keys().copied() {
                commands.trigger(Scroll { entity, delta });
            }
        }
    }
    for t in touches.iter() {
        let d = t.delta();
        if d.y.abs() < 0.5 {
            continue;
        }
        if let Some(map) = hover_map.get(&PointerId::Touch(t.id())) {
            for entity in map.keys().copied() {
                commands.trigger(Scroll { entity, delta: Vec2::new(0.0, -d.y / scale.0) });
            }
        }
    }
}

fn on_scroll(mut scroll: On<Scroll>, mut query: Query<(&mut ScrollPosition, &Node, &ComputedNode)>) {
    let Ok((mut pos, node, computed)) = query.get_mut(scroll.entity) else { return };
    let max = (computed.content_size() - computed.size()) * computed.inverse_scale_factor();
    let delta = &mut scroll.delta;
    if node.overflow.y == OverflowAxis::Scroll && delta.y != 0.0 {
        let at_end = if delta.y > 0.0 { pos.y >= max.y } else { pos.y <= 0.0 };
        if !at_end {
            pos.y = (pos.y + delta.y).clamp(0.0, max.y.max(0.0));
            delta.y = 0.0;
        }
    }
}
