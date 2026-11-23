# Référence — Errance

Mis à jour : 23/11/2026 (chantier Quête écrite : Gérald le Borgne).

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
| `crates/errance_core` | Toutes les règles, zéro Bevy. Testé (53 tests). |
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
- `expedition` : plan de zone (packs, décor, boss, nœuds de récolte) et mise en place du combat.
- `quest` : quêtes écrites (offre, étapes = missions aux monstres fixés, embuscade, récompense), `MissionRun` suit l'objectif sur le terrain.
- `craft` : récolte (nœuds, jauge `Gathering::tick`, carcasses de bêtes) et atelier (recettes calculées, qualité selon la Forge).

### Jeu (Bevy)
- `field.rs` : expédition 3D (sol, décor, combattants, VFX, butin, fin, mode porteur).
- `hud.rs` : joystick flottant (moitié gauche), 7 boutons de skills (déclenchés à l'appui), barres, cible, ordres aux compagnons, zoom pincé.
- `models.rs` : modèles low-poly assemblés (humains par pièces d'équipement, rat, loup, sanglier, ours, araignée, squelette, troll, crapaud, arbres, rochers) + animation procédurale.
- `gather.rs` : nœuds de récolte en 3D (arbre entaillé, filon brillant, touffe de plantes, carcasse) + repère flottant + jauge au-dessus des skills.
- `workshop.rs` : panneau Atelier du forgeron (réserve, recettes par tier, détail + Forger, vente des ressources).
- `town.rs` : diorama de Gué-Rouge + menu ville. `menus.rs` : tous les panneaux. `minigames.rs` : 4 corvées. `save.rs` : sauvegarde auto (20 s + mise en arrière-plan).
- Interface dessinée pour 420 px de haut puis mise à l'échelle (`UiScale`).

## Contenu actuel (compté par le test)
25 voies · 71 talents · 8 zones (T1 → T8) · 4 ressources (bois, minerai, peau, fibre) · 1 quête écrite. Le reste : voir la sortie de `cargo test -p errance_core contenu -- --nocapture`.

## Récolte & artisanat (v0.8)
- Nœuds par zone (`nodes:` dans `zones.ron`) : Tree → bois, Vein → minerai, Herb → fibre ; les bêtes (famille `bete`) laissent une carcasse → peau. Tier = celui de la zone (25 % un cran dessous, 7 % un cran dessus), 3 à 5 unités.
- Rester à moins de 1,9 m : la jauge monte (1,3 s × (1 + 0,2 × (tier-1)) / (1 + 2 % × niveau)). Un coup reçu la remet à zéro. Récolte niv. requis = même échelle que l'équipement (T2:4, T3:12…).
- Gloire de Récolte : 5 × multiplicateur de tier par unité. Rendement : +1 % de chances d'une unité bonus par niveau.
- Ressources : réserve à part (`Profile.resources`), pas de place au sac. Perdues si on tombe, gardées en mode porteur.
- Atelier : recette calculée, pas écrite : ressource principale du matériau (Métal → minerai, Bois → bois, Cuir → peau, Tissu → fibre) × 12 (arme 2 mains) / 8 (arme, torse) / 6 (main secondaire) / 4 (tête, pieds, cape), + un quart en ressource d'appoint, + 1/3 de la valeur en po. Forge niv. requis = échelle des tiers. Qualité : chance +4 %/niveau de Forge. Gloire de Forge : 4 × unités × multiplicateur de tier.
- Talents : Récolte (Bras de bûcheron, Pas du cueilleur, Coureur des bois), Forge (Bras de forgeron, Habitué des braises, Entretien).

## Quêtes écrites (v0.9)
- Contenu : `content/quetes.ron` (vérifié par `content_valide`). Avancement : `Profile.quests` (id → étape ; = nb d'étapes quand finie). Flag `quete:<id>` à la fin.
- Une étape = une mission : la zone sert de décor (couleurs, arbres), les groupes de monstres sont écrits et tous posés (`layout_mission`), boss optionnel, embuscade optionnelle (ennemis qui surgissent en arc devant le héros quand il a parcouru `at` du chemin). Avec un boss : murs effondrés (rochers) autour de lui, brèche au sud.
- Réussite : boss abattu, ou (sans boss) zone vidée APRÈS l'embuscade. Puis retour au feu de camp → le bilan raconte la suite. Tomber = rien de perdu côté quête, on recommence l'étape.
- Interface : bloc « Rumeurs » en haut de la colonne droite de la taverne → panneau Quête (dialogue + Accepter) ; l'étape en cours apparaît en tête de la Porte (bord doré, tier, danger).
- « Gérald le Borgne » (donneuse : Mère Ortie, patronne du Pichet Fêlé), dispo après 1 expédition : 1) La route du sud (Vergues, T2, taille 24) : loups, un cogneur, puis embuscade de la Fouine (lieutenant, 260 PV) + archer + va-nu-pieds ; 2) Le vieux fortin (Collines, T3, taille 28) : 4 groupes + Gérald. Récompense : 150 po, L'Œil de Gérald (unique `quest_only`, Excellente, ne tombe plus au hasard), +50 réputation Gué-Rouge.
- Gérald n'est plus le boss des Collines Sourdes (zone sans boss désormais).
- Simulation (IA, solo) : Recrue T2 contre l'embuscade 38 %, Mage-tank T3 100 % ; Gérald + archer : T2 0 %, T3 83 %.

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
- Aucun son.
- La stat `ItemPower` existe mais n'est lue nulle part (ne pas l'utiliser dans un talent tant que ce n'est pas branché).
- Bijoux, potions, repas : pas encore fabricables (alchimie, cuisine, joaillerie à venir).
- Les panneaux laissent voir un peu la 3D (transparence plus forte que prévu, origine non trouvée).
- Pas de ville marchable (diorama + menu).
- Les Collines Sourdes n'ont plus de boss depuis que Gérald est réservé à la quête.
- La réputation `gue_rouge` est stockée mais ne sert encore à rien (prix, portes, PNJ ?).
