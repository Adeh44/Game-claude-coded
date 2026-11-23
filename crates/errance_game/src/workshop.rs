//! L'atelier de Gué-Rouge : forger de l'équipement avec ses ressources,
//! ou les revendre.

use crate::menus::{MenuState, item_details, scroll_col};
use crate::ui::{self, ButtonLook, Fonts, UiAction, font, pal};
use bevy::prelude::*;
use bevy::ui_widgets::Button;
use errance_core::content::content;
use errance_core::craft::{LINE_FORGE, forge_luck, missing, recipes_at, resource_value, stack_count};
use errance_core::item::Item;
use errance_core::profile::Profile;

/// Tier affiché (1 par défaut).
pub fn tier_of(menu: &MenuState) -> u8 {
    menu.craft_tier.clamp(1, 8)
}

fn recipe_row(fonts: &Fonts, name: String, extra: String, action: UiAction, selected: bool, ready: bool) -> impl Bundle {
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
            (Text::new(name), font(&fonts.bold, 11.5), TextColor(if ready { pal::GREEN } else { pal::INK })),
            (Text::new(extra), font(&fonts.regular, 10.5), TextColor(pal::DIM)),
        ],
    )
}

pub fn build_workshop(commands: &mut Commands, body: Entity, fonts: &Fonts, p: &Profile, menu: &MenuState) {
    let c = content();
    let tier = tier_of(menu);
    let forge = p.hero.masteries.level(LINE_FORGE);
    commands.entity(body).with_children(|b| {
        // Colonne 1 : réserve de ressources.
        b.spawn(scroll_col(px(200))).with_children(|col| {
            col.spawn(ui::bold(fonts, "Réserve", 13.0, pal::INK));
            if p.resources.is_empty() {
                col.spawn(ui::label(fonts, "Rien. Les arbres marqués, les filons et les bêtes abattues vous attendent dehors.", 11.0, pal::DIM));
            }
            for (i, r) in p.resources.iter().enumerate() {
                let Some(d) = c.resource(&r.id) else { continue };
                let v = resource_value(r.tier, r.count);
                col.spawn(recipe_row(fonts, format!("T{} {} ×{}", r.tier, d.tier_name(r.tier), r.count), format!("vendre {v} po"), UiAction::SellResource(i), false, false));
            }
            col.spawn(ui::label(fonts, format!("Forge niv. {forge} · Récolte niv. {}", p.hero.masteries.level(errance_core::craft::LINE_GATHER)), 10.5, pal::DIM));
        });
        // Colonne 2 : recettes du tier choisi.
        b.spawn(scroll_col(px(250))).with_children(|col| {
            col.spawn(ui::row()).with_children(|r| {
                r.spawn(ui::button(fonts, "<", UiAction::CraftTier(tier.saturating_sub(1).max(1)), px(34), 12.0));
                r.spawn(ui::bold(fonts, format!("Tier {tier}"), 13.0, pal::GOLD));
                r.spawn(ui::button(fonts, ">", UiAction::CraftTier((tier + 1).min(8)), px(34), 12.0));
            });
            for rec in recipes_at(c, tier) {
                let Some(base) = c.base(&rec.base) else { continue };
                let ready = missing(c, p, &rec).is_empty();
                let need: Vec<String> = rec.needs.iter().map(|(id, n)| format!("{n} {}", c.resource(id).map(|d| d.name.to_lowercase()).unwrap_or_default())).collect();
                col.spawn(recipe_row(
                    fonts,
                    base.name.clone(),
                    need.join(" + "),
                    UiAction::CraftSelect(rec.base.clone()),
                    menu.craft_sel.as_deref() == Some(rec.base.as_str()),
                    ready,
                ));
            }
        });
        // Colonne 3 : détail et bouton Forger.
        b.spawn((Node { flex_direction: FlexDirection::Column, row_gap: px(4), flex_grow: 1.0, padding: UiRect::all(px(8)), border_radius: BorderRadius::all(px(6)), ..default() }, BackgroundColor(pal::PANEL_LIGHT)))
            .with_children(|d| {
                let sel = menu.craft_sel.as_ref().and_then(|b| errance_core::craft::recipe(c, b, tier));
                let Some(rec) = sel else {
                    d.spawn(ui::label(fonts, "Choisissez une recette.", 12.0, pal::DIM));
                    d.spawn(ui::label(fonts, "Chaque objet demande des ressources de son tier. Plus votre Forge monte, plus les objets sortent beaux.", 11.0, pal::DIM));
                    d.spawn(ui::label(fonts, format!("Chance de qualité : bonus +{:.0} %", forge_luck(forge) * 100.0), 11.0, pal::INK));
                    return;
                };
                let preview = Item::new(0, &rec.base, tier);
                item_details(d, fonts, p, &preview);
                d.spawn(ui::bold(fonts, "Il faut :", 12.0, pal::INK));
                for (id, n) in &rec.needs {
                    let have = stack_count(&p.resources, id, tier);
                    let name = c.resource(id).map(|r| r.tier_name(tier).to_string()).unwrap_or_default();
                    d.spawn(ui::label(fonts, format!("  {name} : {have}/{n}"), 11.5, if have >= *n { pal::GREEN } else { pal::RED }));
                }
                d.spawn(ui::label(fonts, format!("  Forge niv. {} (vous : {forge})", rec.forge_level), 11.5, if forge >= rec.forge_level { pal::GREEN } else { pal::RED }));
                d.spawn(ui::label(fonts, format!("  {} po de charbon et d'outils", rec.gold), 11.5, if p.gold >= rec.gold { pal::GREEN } else { pal::RED }));
                let ok = missing(c, p, &rec).is_empty();
                d.spawn(ui::button_colored(
                    fonts,
                    "Forger",
                    if ok { UiAction::Craft(rec.base.clone(), tier) } else { UiAction::None },
                    px(120),
                    13.0,
                    if ok { pal::BUTTON_ON } else { Color::srgba(0.2, 0.17, 0.14, 0.9) },
                ));
            });
    });
}
