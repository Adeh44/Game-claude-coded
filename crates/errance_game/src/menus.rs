//! Panneaux : sac, fiche, Atlas, marché, taverne, porte, auberge, bilan.

use crate::field::Field;
use crate::ui::{self, ButtonLook, Fonts, UiAction, UiQueue, font, pal, rgb};
use crate::{AppState, Game, Overlay, Panel, Toasts};
use bevy::prelude::*;
use bevy::ui_widgets::Button;
use errance_core::character::{item_mods_with_bonus, item_name};
use errance_core::combat::{Controller, Team};
use errance_core::content::content;
use errance_core::item::{EquipSlot, Item};
use errance_core::loot::{item_value, random_item};
use errance_core::mastery::{LineCategory, level_for_tier};
use errance_core::skill::SkillSlot;
use errance_core::stats::Stat;

#[derive(Resource, Default)]
pub struct MenuState {
    pub dirty: bool,
    pub bag_sel: Option<usize>,
    pub slot_sel: Option<EquipSlot>,
    pub skill_slot: Option<SkillSlot>,
    pub line: Option<String>,
    pub tab: u8,
    pub summary: Vec<String>,
    pub shop: Vec<Item>,
    pub shop_day: u32,
    pub scroll: f32,
}

#[derive(Component)]
struct PanelRoot;

pub struct MenusPlugin;

impl Plugin for MenusPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuState>()
            .add_systems(Update, (menu_actions, rebuild_panel).chain().run_if(not(in_state(AppState::Title))))
            .add_systems(OnExit(AppState::Field), close_overlay)
            .add_systems(OnExit(AppState::Town), close_overlay);
    }
}

fn close_overlay(mut ov: ResMut<Overlay>, mut menu: ResMut<MenuState>) {
    ov.0 = None;
    menu.dirty = true;
}

/// Rafraîchit le combattant du héros après un changement d'équipement.
fn refresh_hero(game: &Game, field: &mut Option<ResMut<Field>>) {
    if let Some(f) = field {
        let spec = game.0.hero.fighter_spec(content(), Team::Ally, Controller::Player);
        let hero = f.hero;
        f.combat.refresh(hero, &spec);
        f.set_changed();
    }
}

#[allow(clippy::too_many_arguments)]
fn menu_actions(
    mut queue: ResMut<UiQueue>,
    mut ov: ResMut<Overlay>,
    mut menu: ResMut<MenuState>,
    game: Option<ResMut<Game>>,
    mut field: Option<ResMut<Field>>,
    mut toasts: ResMut<Toasts>,
    mut next: ResMut<NextState<AppState>>,
    mut commands: Commands,
    state: Res<State<AppState>>,
) {
    let Some(mut game) = game else { return };
    let c = content();
    let mut keep = vec![];
    let in_town = *state.get() == AppState::Town;
    for a in queue.take() {
        match a {
            UiAction::OpenBag => {
                ov.0 = Some(Panel::Bag);
                menu.bag_sel = None;
                menu.slot_sel = None;
            }
            UiAction::OpenCharacter => {
                ov.0 = Some(Panel::Character);
                menu.skill_slot = None;
            }
            UiAction::OpenAtlas => {
                ov.0 = Some(Panel::Atlas);
                menu.line = None;
            }
            UiAction::Close => {
                if ov.0 == Some(Panel::Summary) {
                    // Le bilan se ferme en rentrant en ville.
                    if *state.get() == AppState::Field {
                        next.set(AppState::Town);
                    }
                }
                ov.0 = match &ov.0 {
                    Some(Panel::Npc(_)) => Some(Panel::Tavern),
                    Some(Panel::Chore(_)) => Some(Panel::Chores),
                    _ => None,
                };
                refresh_hero(&game, &mut field);
            }
            UiAction::Tab(t) => menu.tab = t,
            UiAction::SelectBag(i) => {
                menu.bag_sel = Some(i);
                menu.slot_sel = None;
            }
            UiAction::SelectSlot(s) => {
                menu.slot_sel = Some(s);
                menu.bag_sel = None;
            }
            UiAction::Equip(i) => {
                match game.0.equip_from_bag(c, i) {
                    Ok(()) => toasts.push("Équipé.".into()),
                    Err(e) => toasts.push(e),
                }
                menu.bag_sel = None;
                refresh_hero(&game, &mut field);
            }
            UiAction::Unequip(s) => {
                if !game.0.unequip_to_bag(c, s) {
                    toasts.push("Sac plein.".into());
                }
                menu.slot_sel = None;
                refresh_hero(&game, &mut field);
            }
            UiAction::Sell(i) => {
                if !in_town {
                    toasts.push("On ne vend qu'au marché.".into());
                } else if let Some(v) = game.0.sell(i) {
                    toasts.push(format!("Vendu pour {v} po."));
                }
                menu.bag_sel = None;
            }
            UiAction::Buy(i) => {
                if let Some(it) = menu.shop.get(i).cloned() {
                    let price = item_value(&it).max(2);
                    if game.0.gold < price {
                        toasts.push("Pas assez d'or.".into());
                    } else if game.0.add_to_bag(c, it.clone()) {
                        game.0.spend(price);
                        menu.shop.remove(i);
                        toasts.push(format!("Acheté : {}", item_name(c, &it)));
                    } else {
                        toasts.push("Sac plein.".into());
                    }
                }
            }
            UiAction::PickSkillSlot(s) => menu.skill_slot = Some(s),
            UiAction::ChooseSkill(s, id) => {
                if game.0.hero.choose_skill(c, s, &id) {
                    menu.skill_slot = None;
                    refresh_hero(&game, &mut field);
                }
            }
            UiAction::ToggleTalent(id) => {
                if !game.0.hero.toggle_talent(c, &id) {
                    toasts.push("Plus d'emplacement de talent libre.".into());
                }
                refresh_hero(&game, &mut field);
            }
            UiAction::SelectLine(l) => menu.line = Some(l),
            UiAction::Travel(zone) => {
                let party = game.0.world.party().iter().map(|n| n.id).collect();
                commands.insert_resource(crate::field::Expedition { zone, party, porter_for: None });
                ov.0 = None;
                next.set(AppState::Field);
            }
            UiAction::InspectNpc(id) => ov.0 = Some(Panel::Npc(id)),
            UiAction::Hire(id) => {
                let price = game.0.world.npc(id).map(|n| n.hire_price()).unwrap_or(0);
                if game.0.world.party().len() >= 3 {
                    toasts.push("Votre groupe est complet (3 compagnons).".into());
                } else if game.0.spend(price) {
                    if let Some(n) = game.0.world.npc_mut(id) {
                        n.hired_for = 1;
                        n.affinity += 3;
                        toasts.push(format!("{} vous suivra pour la prochaine expédition.", n.sheet.name));
                    }
                    ov.0 = Some(Panel::Tavern);
                } else {
                    toasts.push("Pas assez d'or.".into());
                }
            }
            UiAction::Recruit(id) => {
                let (ok, name) = match game.0.world.npc(id) {
                    Some(n) => (n.affinity >= 25, n.sheet.name.clone()),
                    None => (false, String::new()),
                };
                if game.0.world.party().len() >= 3 {
                    toasts.push("Votre groupe est complet (3 compagnons).".into());
                } else if ok {
                    if let Some(n) = game.0.world.npc_mut(id) {
                        n.companion = true;
                        n.hired_for = 0;
                    }
                    toasts.push(format!("{name} rejoint votre compagnie pour de bon !"));
                    ov.0 = Some(Panel::Tavern);
                } else {
                    toasts.push(format!("{name} ne vous connaît pas assez (affinité 25 requise)."));
                }
            }
            UiAction::Dismiss(id) => {
                if let Some(n) = game.0.world.npc_mut(id) {
                    n.companion = false;
                    n.hired_for = 0;
                    n.affinity -= 10;
                    toasts.push(format!("{} reprend sa route.", n.sheet.name));
                }
            }
            UiAction::Rest => {
                if game.0.spend(2) {
                    game.0.day += 1;
                    let mut r = game.0.rng.fork();
                    let g = &mut game.0;
                    g.world.advance_day(c, &mut r);
                    crate::save::save(&game.0);
                    toasts.push("Une nuit au chaud. Partie sauvegardée.".into());
                } else {
                    toasts.push("Une nuit coûte 2 po. Vous dormirez dehors...".into());
                }
            }
            UiAction::Save => {
                crate::save::save(&game.0);
                toasts.push("Partie sauvegardée.".into());
            }
            other => keep.push(other),
        }
        menu.dirty = true;
    }
    queue.0.extend(keep);
    // Le marché se renouvelle chaque jour.
    if in_town && menu.shop_day != game.0.day {
        menu.shop_day = game.0.day;
        menu.shop = market_stock(&mut game.0);
    }
}

fn market_stock(p: &mut errance_core::profile::Profile) -> Vec<Item> {
    let c = content();
    let mut v = vec![];
    // Toujours : de quoi commencer.
    v.push(p.new_item("gourdin_rouille", 1));
    v.push(p.new_item("potion_soin", 1));
    v.push(p.new_item("pain_dur", 1));
    let mut rng = p.rng.fork();
    for _ in 0..9 {
        let tier = if rng.chance(0.7) { 1 } else { 2 };
        p.next_uid += 1;
        if let Some(it) = random_item(c, &mut rng, p.next_uid, tier, None, 0.0) {
            v.push(it);
        }
    }
    v
}

fn rebuild_panel(
    mut commands: Commands,
    ov: Res<Overlay>,
    mut menu: ResMut<MenuState>,
    roots: Query<Entity, With<PanelRoot>>,
    fonts: Res<Fonts>,
    game: Option<Res<Game>>,
    state: Res<State<AppState>>,
) {
    if !ov.is_changed() && !menu.dirty {
        return;
    }
    menu.dirty = false;
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
    let (Some(panel), Some(game)) = (ov.0.clone(), game) else { return };
    let in_town = *state.get() == AppState::Town;
    let root = commands.spawn((ui::overlay(), PanelRoot)).id();
    let title = match &panel {
        Panel::Bag => "Sac & équipement".to_string(),
        Panel::Character => "Fiche du personnage".into(),
        Panel::Atlas => "Atlas des Voies".into(),
        Panel::Summary => "Bilan de l'expédition".into(),
        Panel::Chores => "Tableau des corvées".into(),
        Panel::Chore(_) => "Au travail !".into(),
        Panel::Market => "Marché de Gué-Rouge".into(),
        Panel::Tavern => "Taverne du Pichet Fêlé".into(),
        Panel::Npc(_) => "Aventurier".into(),
        Panel::Gate => "Porte de la ville".into(),
        Panel::Inn => "Auberge".into(),
        Panel::Company(_) => "Compagnie".into(),
    };
    let close_label = if panel == Panel::Summary { "Retour en ville" } else { "Fermer" };
    commands.entity(root).with_children(|p| {
        p.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, justify_content: JustifyContent::SpaceBetween, margin: UiRect::bottom(px(6)), ..default() })
            .with_children(|r| {
                r.spawn(ui::title(&fonts, title, 18.0, pal::GOLD));
                r.spawn(ui::row()).with_children(|rr| {
                    rr.spawn(ui::bold(&fonts, format!("{} po", game.0.gold), 13.0, pal::GOLD));
                    if panel != Panel::Chore(String::new()) {
                        rr.spawn(ui::button(&fonts, close_label, UiAction::Close, px(if panel == Panel::Summary { 150.0 } else { 90.0 }), 12.0));
                    }
                });
            });
    });
    let body = commands
        .spawn(Node { flex_direction: FlexDirection::Row, flex_grow: 1.0, column_gap: px(10), min_height: px(0), ..default() })
        .id();
    commands.entity(root).add_child(body);
    let p = &game.0;
    match panel {
        Panel::Bag => build_bag(&mut commands, body, &fonts, p, &menu, in_town),
        Panel::Character => build_character(&mut commands, body, &fonts, p, &menu),
        Panel::Atlas => build_atlas(&mut commands, body, &fonts, p, &menu),
        Panel::Summary => build_summary(&mut commands, body, &fonts, &menu),
        Panel::Market => build_market(&mut commands, body, &fonts, p, &menu),
        Panel::Tavern => build_tavern(&mut commands, body, &fonts, p),
        Panel::Npc(id) => build_npc(&mut commands, body, &fonts, p, id),
        Panel::Gate => build_gate(&mut commands, body, &fonts, p),
        Panel::Inn => build_inn(&mut commands, body, &fonts, p),
        Panel::Chores => crate::minigames::build_chore_list(&mut commands, body, &fonts, p),
        Panel::Chore(id) => crate::minigames::build_chore_game(&mut commands, body, &fonts, &id),
        Panel::Company(_) => {}
    }
}

/// Colonne défilante.
fn scroll_col(width: Val) -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(4),
            width,
            overflow: Overflow::scroll_y(),
            min_height: px(0),
            ..default()
        },
        ScrollPosition::default(),
    )
}

fn quality_color(it: &Item) -> Color {
    if it.unique.is_some() {
        return Color::srgb(1.0, 0.55, 0.15);
    }
    rgb(it.quality.color())
}

/// Ligne cliquable d'objet.
fn item_row(fonts: &Fonts, it: &Item, action: UiAction, selected: bool, extra: String) -> impl Bundle {
    let c = content();
    let name = item_name(c, it);
    let count = if it.count > 1 { format!(" ×{}", it.count) } else { String::new() };
    (
        Button,
        action,
        ButtonLook { normal: if selected { pal::BUTTON_ON } else { pal::PANEL_LIGHT }, pressed: pal::BUTTON_ON },
        Node {
            padding: UiRect::axes(px(6), px(3)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: px(6),
            ..default()
        },
        BorderColor::all(if selected { pal::GOLD } else { Color::srgba(0.4, 0.33, 0.22, 0.6) }),
        BackgroundColor(pal::PANEL_LIGHT),
        children![
            (Text::new(format!("T{} {name}{count}", it.tier)), font(&fonts.bold, 11.5), TextColor(quality_color(it))),
            (Text::new(extra), font(&fonts.regular, 10.5), TextColor(pal::DIM)),
        ],
    )
}

fn item_details(c: &mut ChildSpawnerCommands, fonts: &Fonts, p: &errance_core::profile::Profile, it: &Item) {
    let ct = content();
    let Some(b) = ct.base(&it.base) else { return };
    c.spawn(ui::bold(fonts, item_name(ct, it), 14.0, quality_color(it)));
    let mut info = format!("{} · Tier {} · {}", b.slot.label(), it.tier, it.quality.label());
    if b.two_handed {
        info.push_str(" · Deux mains");
    }
    c.spawn(ui::label(fonts, info, 11.0, pal::DIM));
    if !b.line.is_empty() {
        let lvl = p.hero.masteries.level(&b.line);
        let need = level_for_tier(it.tier);
        let line_name = ct.line(&b.line).map(|l| l.name.clone()).unwrap_or_default();
        let ok = lvl >= need;
        c.spawn(ui::label(
            fonts,
            format!("Voie : {line_name} (niv. {lvl}){}", if ok { String::new() } else { format!(" — requiert niv. {need}") }),
            11.0,
            if ok { pal::GREEN } else { pal::RED },
        ));
        let bonus = errance_core::mastery::item_power_bonus(lvl);
        for m in item_mods_with_bonus(ct, it, bonus) {
            c.spawn(ui::label(fonts, format!("  {}", m.describe()), 11.5, pal::INK));
        }
        if b.slot == EquipSlot::MainHand {
            let skills: Vec<String> = ct.skills.iter().filter(|s| s.line == b.line && s.slot != SkillSlot::Basic && s.mastery <= lvl).map(|s| s.name.clone()).collect();
            c.spawn(ui::label(fonts, format!("Skills : {}", skills.join(", ")), 10.5, Color::srgb(0.75, 0.85, 1.0)));
        }
    } else {
        for m in item_mods_with_bonus(ct, it, 0.0) {
            c.spawn(ui::label(fonts, format!("  {}", m.describe()), 11.5, pal::INK));
        }
    }
    if let Some(u) = it.unique.as_ref().and_then(|u| ct.unique(u)) {
        c.spawn(ui::label(fonts, format!("« {} »", u.lore), 10.5, Color::srgb(1.0, 0.75, 0.4)));
    }
    if !b.description.is_empty() {
        c.spawn(ui::label(fonts, b.description.clone(), 10.5, pal::DIM));
    }
    c.spawn(ui::label(fonts, format!("Valeur : {} po", item_value(it)), 10.5, pal::GOLD));
}

fn build_bag(commands: &mut Commands, body: Entity, fonts: &Fonts, p: &errance_core::profile::Profile, menu: &MenuState, in_town: bool) {
    let c = content();
    commands.entity(body).with_children(|b| {
        // Équipement porté.
        b.spawn(scroll_col(px(250))).with_children(|col| {
            col.spawn(ui::bold(fonts, "Porté", 13.0, pal::INK));
            for slot in EquipSlot::ALL {
                match p.hero.item(slot) {
                    Some(it) => {
                        col.spawn(item_row(fonts, it, UiAction::SelectSlot(slot), menu.slot_sel == Some(slot), slot.label().to_string()));
                    }
                    None => {
                        col.spawn(ui::label(fonts, format!("{} : —", slot.label()), 11.0, Color::srgba(0.6, 0.55, 0.5, 0.7)));
                    }
                }
            }
        });
        // Sac.
        b.spawn(scroll_col(px(280))).with_children(|col| {
            col.spawn(ui::bold(fonts, format!("Sac ({}/{})", p.bag.len(), errance_core::profile::BAG_SIZE), 13.0, pal::INK));
            if p.bag.is_empty() {
                col.spawn(ui::label(fonts, "Vide. Comme vos poches.", 11.0, pal::DIM));
            }
            for (i, it) in p.bag.iter().enumerate() {
                let slot = c.base(&it.base).map(|b| b.slot.label()).unwrap_or("");
                col.spawn(item_row(fonts, it, UiAction::SelectBag(i), menu.bag_sel == Some(i), slot.to_string()));
            }
        });
        // Détails.
        b.spawn((Node { flex_direction: FlexDirection::Column, row_gap: px(4), flex_grow: 1.0, padding: UiRect::all(px(8)), border_radius: BorderRadius::all(px(6)), ..default() }, BackgroundColor(pal::PANEL_LIGHT)))
            .with_children(|d| {
                if let Some(i) = menu.bag_sel {
                    if let Some(it) = p.bag.get(i) {
                        item_details(d, fonts, p, it);
                        // Comparaison avec l'objet porté.
                        if let Some(slot) = c.base(&it.base).map(|b| b.slot) {
                            if let Some(worn) = p.hero.item(slot) {
                                d.spawn(ui::label(fonts, format!("Porté actuellement : {}", item_name(c, worn)), 10.5, pal::DIM));
                            }
                        }
                        d.spawn(ui::row()).with_children(|r| {
                            r.spawn(ui::button(fonts, "Équiper", UiAction::Equip(i), px(100), 12.0));
                            if in_town {
                                r.spawn(ui::button(fonts, format!("Vendre ({} po)", (item_value(it) / 2).max(1)), UiAction::Sell(i), px(130), 12.0));
                            }
                        });
                    }
                } else if let Some(s) = menu.slot_sel {
                    if let Some(it) = p.hero.item(s) {
                        item_details(d, fonts, p, it);
                        d.spawn(ui::button(fonts, "Retirer", UiAction::Unequip(s), px(110), 12.0));
                    }
                } else {
                    d.spawn(ui::label(fonts, "Touchez un objet pour le voir.", 12.0, pal::DIM));
                    let st = p.hero.stats(c);
                    d.spawn(ui::label(fonts, format!("PV {:.0} · Armure {:.0} · Rés. {:.0}", st.get(Stat::MaxHp), st.get(Stat::Armor), st.get(Stat::MagicResist)), 12.0, pal::INK));
                    d.spawn(ui::label(fonts, format!("Puiss. physique {:.0} · magique {:.0} · soin {:.0}", st.get(Stat::PhysicalPower), st.get(Stat::MagicPower), st.get(Stat::HealPower)), 12.0, pal::INK));
                }
            });
    });
}

fn build_character(commands: &mut Commands, body: Entity, fonts: &Fonts, p: &errance_core::profile::Profile, menu: &MenuState) {
    let c = content();
    let h = &p.hero;
    let st = h.stats(c);
    let badge = h.role(c);
    commands.entity(body).with_children(|b| {
        // Stats & rôle.
        b.spawn(scroll_col(px(220))).with_children(|col| {
            col.spawn(ui::bold(fonts, format!("Rôle détecté : {}", badge.title), 13.0, pal::GOLD));
            let bars = [("Tank", badge.tank), ("Soin", badge.healer), ("Dégâts", badge.damage), ("Contrôle", badge.control), ("Soutien", badge.support)];
            for (n, v) in bars {
                col.spawn(ui::row()).with_children(|r| {
                    r.spawn((Text::new(n), font(&fonts.regular, 10.5), TextColor(pal::DIM), Node { width: px(56), ..default() }));
                    r.spawn((Node { width: px(120), height: px(6), ..default() }, BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)))).with_child((
                        Node { width: percent((v / 6.0 * 100.0).min(100.0)), height: percent(100), ..default() },
                        BackgroundColor(pal::GOLD),
                    ));
                });
            }
            col.spawn(Node { height: px(6), ..default() });
            for s in Stat::ALL {
                let v = st.get(s);
                if v.abs() < 0.001 || s == Stat::ItemPower {
                    continue;
                }
                let txt = if s.is_ratio() { format!("{} : {:.0} %", s.label(), v * 100.0) } else { format!("{} : {}", s.label(), errance_core::stats::fmt_num(v)) };
                col.spawn(ui::label(fonts, txt, 11.0, pal::INK));
            }
        });
        // Skills.
        b.spawn(scroll_col(px(250))).with_children(|col| {
            col.spawn(ui::bold(fonts, "Skills (touchez pour changer)", 13.0, pal::INK));
            for slot in SkillSlot::ACTIVE {
                let cur = h.skills.get(&slot).and_then(|id| c.skill(id));
                let name = cur.map(|s| s.name.clone()).unwrap_or_else(|| "— (rien de débloqué)".into());
                col.spawn((
                    Button,
                    UiAction::PickSkillSlot(slot),
                    ButtonLook { normal: if menu.skill_slot == Some(slot) { pal::BUTTON_ON } else { pal::PANEL_LIGHT }, pressed: pal::BUTTON_ON },
                    Node { padding: UiRect::axes(px(6), px(3)), border_radius: BorderRadius::all(px(4)), flex_direction: FlexDirection::Column, ..default() },
                    BackgroundColor(pal::PANEL_LIGHT),
                    children![
                        (Text::new(format!("{} : {}", slot.label(), name)), font(&fonts.bold, 11.5), TextColor(pal::INK)),
                    ],
                ));
            }
            col.spawn(Node { height: px(6), ..default() });
            col.spawn(ui::bold(fonts, format!("Talents ({}/{})", h.talents.len(), h.talent_slots()), 13.0, pal::INK));
            let lines = h.equipped_lines(c);
            for t in h.unlocked_talents(c) {
                let on = h.talents.contains(&t.id);
                let inactive = t.needs_equipped && !lines.contains(&t.line);
                let color = if inactive { pal::DIM } else if on { pal::GREEN } else { pal::INK };
                col.spawn((
                    Button,
                    UiAction::ToggleTalent(t.id.clone()),
                    ButtonLook { normal: if on { Color::srgba(0.2, 0.35, 0.18, 0.95) } else { pal::PANEL_LIGHT }, pressed: pal::BUTTON_ON },
                    Node { padding: UiRect::axes(px(6), px(3)), border_radius: BorderRadius::all(px(4)), flex_direction: FlexDirection::Column, ..default() },
                    BackgroundColor(pal::PANEL_LIGHT),
                    children![
                        (Text::new(format!("{}{}", if on { "[x] " } else { "" }, t.name)), font(&fonts.bold, 11.0), TextColor(color)),
                        (Text::new(t.description.clone()), font(&fonts.regular, 10.0), TextColor(pal::DIM)),
                    ],
                ));
            }
            if h.unlocked_talents(c).is_empty() {
                col.spawn(ui::label(fonts, "Aucun talent encore. Pratiquez une Voie (même le Labeur !).", 10.5, pal::DIM));
            }
        });
        // Choix d'un skill.
        b.spawn(scroll_col(Val::Auto)).insert(Node { flex_grow: 1.0, flex_direction: FlexDirection::Column, row_gap: px(4), overflow: Overflow::scroll_y(), ..default() }).with_children(|col| {
            match menu.skill_slot {
                Some(slot) => {
                    col.spawn(ui::bold(fonts, format!("Choisir : {}", slot.label()), 13.0, pal::GOLD));
                    let all = h.all_skills_for(c, slot);
                    if all.is_empty() {
                        col.spawn(ui::label(fonts, "Aucun objet ne donne de skill ici. Équipez-en un !", 11.0, pal::DIM));
                    }
                    for (s, unlocked) in all {
                        let line = c.line(&s.line).map(|l| l.name.clone()).unwrap_or_default();
                        let head = if unlocked { s.name.clone() } else { format!("(verrou) {} — {} niv. {}", s.name, line, s.mastery) };
                        let mut lines = vec![s.description.clone()];
                        lines.extend(s.effect_lines());
                        let meta = format!("Coût {} · Recharge {} s{}", s.cost, s.cooldown, if s.cast_time > 0.0 { format!(" · Incantation {} s", s.cast_time) } else { String::new() });
                        col.spawn((
                            Button,
                            if unlocked { UiAction::ChooseSkill(slot, s.id.clone()) } else { UiAction::None },
                            ButtonLook { normal: if h.skills.get(&slot) == Some(&s.id) { Color::srgba(0.2, 0.35, 0.18, 0.95) } else { pal::PANEL_LIGHT }, pressed: pal::BUTTON_ON },
                            Node { padding: UiRect::all(px(5)), border_radius: BorderRadius::all(px(4)), flex_direction: FlexDirection::Column, ..default() },
                            BackgroundColor(pal::PANEL_LIGHT),
                            children![
                                (Text::new(head), font(&fonts.bold, 12.0), TextColor(if unlocked { pal::INK } else { pal::DIM })),
                                (Text::new(lines.join("\n")), font(&fonts.regular, 10.5), TextColor(pal::DIM)),
                                (Text::new(meta), font(&fonts.regular, 10.0), TextColor(Color::srgb(0.7, 0.8, 1.0))),
                            ],
                        ));
                    }
                }
                None => {
                    col.spawn(ui::label(fonts, "Ici, pas de classe : ce que vous portez décide de vos skills.\nL'arme donne 3 skills, la tête, le torse et les pieds 1 chacun.\nLes talents s'ajoutent par-dessus. Mage en plaques ? Pourquoi pas.", 11.5, pal::DIM));
                }
            }
        });
    });
}

fn build_atlas(commands: &mut Commands, body: Entity, fonts: &Fonts, p: &errance_core::profile::Profile, menu: &MenuState) {
    let c = content();
    let m = &p.hero.masteries;
    commands.entity(body).with_children(|b| {
        b.spawn(scroll_col(px(260))).with_children(|col| {
            col.spawn(ui::label(fonts, format!("Maîtrise totale : {} · Talents : {}", m.total(), p.hero.talent_slots()), 11.0, pal::DIM));
            for cat in [LineCategory::Weapon, LineCategory::OffHand, LineCategory::Armor, LineCategory::Job] {
                col.spawn(ui::bold(fonts, cat.label(), 12.5, pal::GOLD));
                for l in c.lines_of(cat) {
                    let lvl = m.level(&l.id);
                    let fr = m.fraction(&l.id);
                    let sel = menu.line.as_deref() == Some(l.id.as_str());
                    col.spawn((
                        Button,
                        UiAction::SelectLine(l.id.clone()),
                        ButtonLook { normal: if sel { pal::BUTTON_ON } else { pal::PANEL_LIGHT }, pressed: pal::BUTTON_ON },
                        Node { padding: UiRect::axes(px(6), px(3)), border_radius: BorderRadius::all(px(4)), flex_direction: FlexDirection::Column, row_gap: px(2), ..default() },
                        BackgroundColor(pal::PANEL_LIGHT),
                    ))
                    .with_children(|r| {
                        r.spawn((Text::new(format!("{} — niv. {}", l.name, lvl)), font(&fonts.bold, 11.0), TextColor(if lvl > 0 { rgb(l.color) } else { pal::DIM })));
                        r.spawn((Node { width: px(220), height: px(4), ..default() }, BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5))))
                            .with_child((Node { width: percent(fr * 100.0), height: percent(100), ..default() }, BackgroundColor(rgb(l.color))));
                    });
                }
            }
        });
        b.spawn(scroll_col(Val::Auto)).insert(Node { flex_grow: 1.0, flex_direction: FlexDirection::Column, row_gap: px(3), overflow: Overflow::scroll_y(), ..default() }).with_children(|col| {
            let Some(id) = &menu.line else {
                col.spawn(ui::label(fonts, "Chaque arme, armure et métier a sa Voie.\nOn progresse en pratiquant : la gloire gagnée avec un objet fait monter sa Voie.\nTout l'Atlas est complétable. La spécialisation, c'est là où vous passez vos heures.", 12.0, pal::DIM));
                return;
            };
            let Some(l) = c.line(id) else { return };
            let lvl = m.level(id);
            col.spawn(ui::bold(fonts, format!("{} — niveau {} / 100", l.name, lvl), 14.0, rgb(l.color)));
            col.spawn(ui::label(fonts, l.description.clone(), 11.0, pal::DIM));
            let prog = m.progress(id);
            col.spawn(ui::label(fonts, format!("Gloire : {:.0} / {:.0} vers le niveau suivant", prog.fame, errance_core::mastery::fame_for_next(lvl)), 10.5, pal::DIM));
            // Nœuds : tiers, skills, talents, triés par niveau.
            let mut nodes: Vec<(u32, String, bool)> = vec![];
            if matches!(l.category, LineCategory::Weapon | LineCategory::Armor | LineCategory::OffHand) {
                for t in 2..=8u8 {
                    nodes.push((level_for_tier(t), format!("Porter le tier {t}"), true));
                }
                nodes.push((0, "+1 % de puissance d'objet par niveau".into(), true));
            }
            for s in c.skills.iter().filter(|s| s.line == *id && s.slot != SkillSlot::Basic) {
                nodes.push((s.mastery, format!("Skill {} : {}", s.slot.label(), s.name), true));
            }
            for t in c.passives.iter().filter(|t| t.line == *id) {
                nodes.push((t.mastery, format!("Talent : {} — {}", t.name, t.description), true));
            }
            nodes.sort_by_key(|n| n.0);
            for (need, txt, _) in nodes {
                let ok = lvl >= need;
                col.spawn(ui::label(fonts, format!("{} niv. {:>3}  {}", if ok { "*" } else { "-" }, need, txt), 11.0, if ok { pal::INK } else { pal::DIM }));
            }
        });
    });
}

fn build_summary(commands: &mut Commands, body: Entity, fonts: &Fonts, menu: &MenuState) {
    commands.entity(body).with_children(|b| {
        b.spawn(scroll_col(percent(100))).with_children(|col| {
            for l in &menu.summary {
                col.spawn(ui::label(fonts, l.clone(), 13.0, pal::INK));
            }
        });
    });
}

fn build_market(commands: &mut Commands, body: Entity, fonts: &Fonts, p: &errance_core::profile::Profile, menu: &MenuState) {
    let c = content();
    commands.entity(body).with_children(|b| {
        b.spawn(scroll_col(px(330))).with_children(|col| {
            col.spawn(ui::bold(fonts, "À vendre aujourd'hui", 13.0, pal::INK));
            for (i, it) in menu.shop.iter().enumerate() {
                let price = item_value(it).max(2);
                col.spawn(item_row(fonts, it, UiAction::Buy(i), false, format!("{price} po")));
            }
        });
        b.spawn(scroll_col(px(300))).with_children(|col| {
            col.spawn(ui::bold(fonts, "Votre sac (touchez pour vendre)", 13.0, pal::INK));
            for (i, it) in p.bag.iter().enumerate() {
                let v = (item_value(it) / 2).max(1);
                col.spawn(item_row(fonts, it, UiAction::Sell(i), false, format!("+{v} po")));
            }
            if p.bag.is_empty() {
                col.spawn(ui::label(fonts, "Rien à vendre.", 11.0, pal::DIM));
            }
            let _ = c;
        });
    });
}

fn build_tavern(commands: &mut Commands, body: Entity, fonts: &Fonts, p: &errance_core::profile::Profile) {
    let c = content();
    commands.entity(body).with_children(|b| {
        b.spawn(scroll_col(px(380))).with_children(|col| {
            col.spawn(ui::bold(fonts, "Aventuriers libres", 13.0, pal::INK));
            for n in p.world.tavern() {
                let role = n.sheet.role(c).title;
                let w = n.sheet.item(EquipSlot::MainHand).map(|i| item_name(c, i)).unwrap_or_default();
                col.spawn((
                    Button,
                    UiAction::InspectNpc(n.id),
                    ButtonLook { normal: pal::PANEL_LIGHT, pressed: pal::BUTTON_ON },
                    Node { padding: UiRect::axes(px(6), px(3)), border_radius: BorderRadius::all(px(4)), flex_direction: FlexDirection::Column, ..default() },
                    BackgroundColor(pal::PANEL_LIGHT),
                    children![
                        (Text::new(format!("{} — {} (T{})", n.sheet.name, role, n.sheet.power_tier())), font(&fonts.bold, 11.5), TextColor(pal::INK)),
                        (Text::new(format!("{w} · {} po l'expédition", n.hire_price())), font(&fonts.regular, 10.5), TextColor(pal::DIM)),
                    ],
                ));
            }
        });
        b.spawn(scroll_col(Val::Auto)).insert(Node { flex_grow: 1.0, flex_direction: FlexDirection::Column, row_gap: px(4), ..default() }).with_children(|col| {
            col.spawn(ui::bold(fonts, "Votre groupe", 13.0, pal::INK));
            let party = p.world.party();
            if party.is_empty() {
                col.spawn(ui::label(fonts, "Personne. Un mercenaire se loue pour une expédition ;\nun compagnon de longue date se gagne (affinité 25+).", 11.0, pal::DIM));
            }
            for n in party {
                let kind = if n.companion { "Compagnon".to_string() } else { format!("Mercenaire ({} exp.)", n.hired_for) };
                col.spawn(ui::row()).with_children(|r| {
                    r.spawn(ui::label(fonts, format!("{} — {} · affinité {}", n.sheet.name, kind, n.affinity), 11.5, pal::INK));
                    r.spawn(ui::button(fonts, "Voir", UiAction::InspectNpc(n.id), px(50), 11.0));
                    r.spawn(ui::button(fonts, "Renvoyer", UiAction::Dismiss(n.id), px(80), 11.0));
                });
            }
        });
    });
}

fn build_npc(commands: &mut Commands, body: Entity, fonts: &Fonts, p: &errance_core::profile::Profile, id: u32) {
    let c = content();
    let Some(n) = p.world.npc(id) else { return };
    let st = n.sheet.stats(c);
    commands.entity(body).with_children(|b| {
        b.spawn(scroll_col(px(330))).with_children(|col| {
            col.spawn(ui::bold(fonts, n.sheet.name.clone(), 15.0, pal::GOLD));
            col.spawn(ui::label(fonts, format!("« {} »", n.line), 11.5, pal::DIM));
            col.spawn(ui::label(fonts, format!("Rôle : {} · Tier {}", n.sheet.role(c).title, n.sheet.power_tier()), 12.0, pal::INK));
            let traits: Vec<String> = n.traits.iter().map(|t| format!("{} ({})", t.label(), t.description())).collect();
            col.spawn(ui::label(fonts, format!("Traits : {}", traits.join(" · ")), 11.0, pal::INK));
            col.spawn(ui::label(fonts, format!("Affinité avec vous : {}", n.affinity), 11.0, if n.affinity >= 25 { pal::GREEN } else { pal::DIM }));
            if let Some(co) = n.company.and_then(|cid| p.world.company(cid)) {
                col.spawn(ui::label(fonts, format!("Membre de : {}", co.name), 11.0, pal::INK));
            }
            col.spawn(ui::label(fonts, format!("PV {:.0} · Armure {:.0} · Puiss. {:.0}/{:.0} · Soin {:.0}", st.get(Stat::MaxHp), st.get(Stat::Armor), st.get(Stat::PhysicalPower), st.get(Stat::MagicPower), st.get(Stat::HealPower)), 11.0, pal::INK));
            for slot in [EquipSlot::MainHand, EquipSlot::OffHand, EquipSlot::Head, EquipSlot::Chest, EquipSlot::Feet] {
                if let Some(it) = n.sheet.item(slot) {
                    col.spawn(ui::label(fonts, format!("{} : {}", slot.label(), item_name(c, it)), 10.5, quality_color(it)));
                }
            }
            let skills: Vec<String> = n.sheet.skills.values().filter_map(|id| c.skill(id)).map(|s| s.name.clone()).collect();
            col.spawn(ui::label(fonts, format!("Skills : {}", skills.join(", ")), 10.5, Color::srgb(0.75, 0.85, 1.0)));
            let rules: Vec<String> = n.sheet.rules.iter().map(|r| format!("Si {} → {}", r.when.label(), r.then.label())).collect();
            col.spawn(ui::label(fonts, format!("Règles :\n{}", rules.join("\n")), 10.0, pal::DIM));
        });
        b.spawn(Node { flex_direction: FlexDirection::Column, row_gap: px(6), ..default() }).with_children(|col| {
            let in_party = n.companion || n.hired_for > 0;
            if !in_party {
                col.spawn(ui::button(fonts, format!("Engager ({} po, 1 expédition)", n.hire_price()), UiAction::Hire(n.id), px(240), 12.0));
                col.spawn(ui::button(fonts, "Proposer de rejoindre ma compagnie", UiAction::Recruit(n.id), px(240), 12.0));
            } else if !n.companion {
                col.spawn(ui::button(fonts, "Proposer de rester (compagnon)", UiAction::Recruit(n.id), px(240), 12.0));
            }
            col.spawn(ui::label(fonts, "Mercenaire : payé pour une expédition, équipement et règles fixes.\nCompagnon : reste avec vous, gagne en maîtrise, devient vraiment à vous.", 10.5, pal::DIM));
        });
    });
}

fn build_gate(commands: &mut Commands, body: Entity, fonts: &Fonts, p: &errance_core::profile::Profile) {
    let c = content();
    commands.entity(body).with_children(|b| {
        b.spawn(scroll_col(px(460))).with_children(|col| {
            for z in &c.zones {
                let danger = (z.tier as i32 - p.hero.power_tier() as i32).max(0);
                let warn = match danger {
                    0 => "",
                    1 => " — dangereux",
                    _ => " — mortel pour vous",
                };
                col.spawn((
                    Button,
                    UiAction::Travel(z.id.clone()),
                    ButtonLook { normal: pal::PANEL_LIGHT, pressed: pal::BUTTON_ON },
                    Node { padding: UiRect::all(px(6)), border_radius: BorderRadius::all(px(5)), flex_direction: FlexDirection::Column, ..default() },
                    BackgroundColor(pal::PANEL_LIGHT),
                    children![
                        (Text::new(format!("{} — Tier {}{}", z.name, z.tier, warn)), font(&fonts.bold, 13.0), TextColor(if danger >= 2 { pal::RED } else if danger == 1 { pal::GOLD } else { pal::INK })),
                        (Text::new(z.description.clone()), font(&fonts.regular, 11.0), TextColor(pal::DIM)),
                    ],
                ));
            }
        });
        b.spawn(Node { flex_direction: FlexDirection::Column, row_gap: px(4), flex_grow: 1.0, ..default() }).with_children(|col| {
            col.spawn(ui::bold(fonts, "Qui part avec vous", 13.0, pal::INK));
            let party = p.world.party();
            if party.is_empty() {
                col.spawn(ui::label(fonts, "Vous partez seul.", 11.0, pal::DIM));
            }
            for n in party {
                col.spawn(ui::label(fonts, format!("• {} ({})", n.sheet.name, n.sheet.role(c).title), 11.0, pal::INK));
            }
            col.spawn(ui::label(fonts, "Le butin ramassé n'est à vous qu'une fois rentré.\nSi vous tombez, il est perdu.", 10.5, pal::DIM));
        });
    });
}

fn build_inn(commands: &mut Commands, body: Entity, fonts: &Fonts, p: &errance_core::profile::Profile) {
    commands.entity(body).with_children(|b| {
        b.spawn(Node { flex_direction: FlexDirection::Column, row_gap: px(8), ..default() }).with_children(|col| {
            col.spawn(ui::label(fonts, "La patronne essuie un verre en vous regardant de haut en bas.\n« Une paillasse, c'est 2 pièces. D'avance. »", 12.0, pal::INK));
            col.spawn(ui::button(fonts, "Dormir (2 po) — la journée passe, sauvegarde", UiAction::Rest, px(360), 12.0));
            col.spawn(ui::button(fonts, "Sauvegarder", UiAction::Save, px(200), 12.0));
            let c = &p.counters;
            col.spawn(ui::label(fonts, format!("Jour {} · Ennemis vaincus : {} · Chutes : {} · Corvées : {} · Expéditions : {}", p.day, c.kills, c.deaths, c.chores, c.expeditions), 11.0, pal::DIM));
        });
    });
}
