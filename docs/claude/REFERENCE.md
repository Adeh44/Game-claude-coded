# Référence — Errance

Mis à jour : 22/11/2026 (nuit 1).

## Le jeu en une phrase
RPG solo mobile (Android, paysage), monde de fantasy médiévale « sandbox » : on arrive en haillons à Gué-Rouge, on fait des corvées, on survit aux rats, on porte la torche pour des compagnies… puis on devient très fort dans la Voie qu'on a pratiquée.

## Piliers
1. **Tu es ce que tu portes.** Pas de classe. L'arme donne 3 skills (Arme I, II, III + attaque de base), la tête/le torse/les pieds 1 skill chacun selon leur poids (tissu, cuir, plaque). Les talents (passifs) s'ajoutent dans des emplacements limités (1 → 6).
2. **Progression par la pratique.** Chaque arme, main secondaire, armure et métier = une Voie (0 → 100). La gloire gagnée avec un objet fait monter sa Voie. La maîtrise débloque : tiers portables, skills avancés, talents, +1 %/niveau de puissance d'objet. Tout l'Atlas est complétable.
3. **Départ à la dure.** 60 PV, mains nues, 0 po. Corvées (mini-jeux) → Labeur (talents de PV/énergie/armure). Vérifié par simulation : à mains nues, un loup affamé = 47 % de victoire.
4. **Monde vivant.** ~32 aventuriers PNJ avec de vraies fiches (équipement, maîtrises, règles, traits), des compagnies. Ils progressent hors écran.
5. **Compagnons.** Mercenaires (1 expédition, payés) ou compagnons de longue date (affinité ≥ 25) dont on gère équipement, skills, talents et règles « si… alors… ».
6. **Être recruté.** Les compagnies emmènent le joueur comme Porteur (rang 0) : on suit, on ramasse le butin, on touche une petite part. Rangs : Porteur → Recrue (30) → Membre (100) → Vétéran (250) → Officier (500).

## Architecture (workspace Cargo)
| Crate | Rôle |
|---|---|
| `crates/errance_core` | Toutes les règles, zéro Bevy. Testé (38 tests). |
| `crates/errance_core/content/*.ron` | Contenu (voies, skills, talents, objets, affixes, uniques, ennemis, zones, corvées, noms). Embarqué via `include_str!`. |
| `crates/errance_game` | Bevy 0.20 : rendu 3D low-poly procédural, HUD tactile, menus, mini-jeux. Lib `cdylib` pour Android + bin `errance` pour PC. |
| `crates/errance_sim` | Simulateur d'équilibrage (milliers de combats). |
| `android/` | Projet Gradle (GameActivity), paysage verrouillé, icône vectorielle, clé debug fixe. |
| `.github/workflows/ci.yml` | Tests + clippy + APK + release `apk-latest`. |

### Modules du cœur
- `stats` : 22 stats, `StatBlock` (base + fixes) × (1 + %), plafonds, mitigation r/(r+100).
- `skill` : `SkillDef` = ciblage (Enemy, Ally, AreaAtTarget, Cone, Line, Dash, Leap…) + livraison (Instant, Projectile, Delayed = zone esquivable) + effets (Damage, Heal, Shield, Status, Taunt, Knockback, Pull, Detonate…). 19 états. Mots-clés (Tag) pour les synergies.
- `passive` : talents (Stat, StatWhile, TagPower, TagCooldown, OnHit, OnHeal, OnDamaged, OnKill, Convert, StatusDuration).
- `combat` : moteur temps réel 2D. Menace (aggro), provocation, IA par règles (compagnons et monstres), laisse des monstres, meute, projectiles, zones retardées, gloire partagée, événements pour l'affichage. Reproductible (graine).
- `character` : fiche (équipement → stats → `FighterSpec`), choix de skills/talents, badge de rôle (« Mage-Rempart »…).
- `mastery` : courbe 40 × 1,075^niveau, tiers par niveau (T2:4, T3:12, T4:22, T5:35, T6:50, T7:68, T8:85).
- `item`/`loot` : bases × tier (T8 ≈ 7× T1) × qualité (5) × affixes ; uniques ; prix.
- `world` : aventuriers, compagnies, rangs, traits, jours qui passent.
- `profile` : sauvegarde (RON), sac 40 places.
- `expedition` : plan de zone (packs, décor, boss) et mise en place du combat.

### Jeu (Bevy)
- `field.rs` : expédition 3D (sol, décor, combattants, VFX, butin, fin, mode porteur).
- `hud.rs` : joystick flottant (moitié gauche), 7 boutons de skills (déclenchés à l'appui), barres, cible, ordres aux compagnons, zoom pincé.
- `models.rs` : modèles low-poly assemblés (humains par pièces d'équipement, rat, loup, sanglier, ours, araignée, squelette, troll, crapaud, arbres, rochers) + animation procédurale.
- `town.rs` : diorama de Gué-Rouge + menu ville. `menus.rs` : tous les panneaux. `minigames.rs` : 4 corvées. `save.rs` : sauvegarde auto (20 s + mise en arrière-plan).
- Interface dessinée pour 420 px de haut puis mise à l'échelle (`UiScale`).

## Contenu actuel (compté par le test)
23 voies · 153 skills · 65 talents · 63 objets de base · 22 affixes · 6 uniques · 14 ennemis (T1 → T4) · 4 zones · 6 corvées.

## Décisions
- Bevy (pur Rust, tout en code, Android officiel). Rejetés : Macroquad (trop bas niveau), Fyrox (mobile peu éprouvé), Godot+Rust (éditeur à clics).
- 3D low-poly procédurale plutôt que 2D : rend bien sans artiste, l'équipement se voit.
- Contenu embarqué (pas de chargement d'assets sur Android).
- Emplacements Mains/Jambes retirés (écran mobile chargé) — voir idées.
- L'IA des compagnons ET des monstres utilise le même système de règles.

## Décisions rejetées
- Niveau de personnage global (contraire à « on progresse par la pratique »).
- Points de talent à répartir (contraire à « tout l'Atlas est complétable »).

## Dettes ouvertes
- APK jamais construit : le push GitHub est bloqué (403, app Claude non installée sur le dépôt). La CI est prête mais n'a jamais tourné.
- Aucun son.
- Zones et ennemis seulement jusqu'au tier 4 (objets jusqu'au T8).
- Les panneaux laissent voir un peu la 3D (transparence plus forte que prévu, origine non trouvée).
- Pas de ville marchable (diorama + menu).
