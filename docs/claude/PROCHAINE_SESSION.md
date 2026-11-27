# Prochaine session

## État réel (27/11/2026, passation vers un nouveau projet claude.ai)
- Branche `claude/mobile-rpg-game-design-o55gg9`, tout est poussé. Release CI : `apk-latest`.
- Version de contenu v0.10 (escalade de Givrecol). Depuis : réparation de l'affichage Android + retours de jeu d'Adeh + monde plus vivant (voir ci-dessous).
- 56 tests verts, clippy propre.
- **Le jeu tourne et s'affiche sur le téléphone d'Adeh** (Redmi, Android 16, GPU Mali-G57 MC2, Vulkan) depuis le 25/11.

### Ce qui a été fait du 23 au 27/11 (fil précédent)
1. Plantage au lancement sur téléphone : bug wgpu-hal + pilote Vulkan (`ERROR_FRAGMENTED_POOL`) → copie patchée dans `vendor/wgpu-hal`.
2. 3D invisible sur téléphone : vraie cause = MSAA différent entre caméra 3D et caméra d'interface → `Msaa::Off` sur toutes les caméras (règle dans METHODE). Bindless aussi désactivé. OpenGL plante au démarrage sur ce téléphone (impasse, le garde-fou rebascule sur Vulkan).
3. Outils de diagnostic : boîte noire (rapport de plantage sur l'écran titre), journal du moteur en bas à gauche + bouton « Copier le journal pour Claude » (presse-papiers Android par JNI). Adeh colle le texte dans la conversation : pas besoin de captures.
4. Retours de jeu d'Adeh (25/11), tous faits : récolte au toucher (bouton), cercle lumineux au sol sous les ressources, section « Dans vos poches » du sac en expédition, panneau « Vos sorts » dans la Fiche.
5. Monde plus vivant (27/11, commit a1e18fb) : `scenery.rs` (herbe dense en maillages fusionnés, fleurs, buissons, cailloux, chemin, neige/braises/champignons selon la zone), +2 à 3 groupes de monstres par zone. À faire confirmer par Adeh sur téléphone (fluidité !).

### Chantier EN COURS : vrais modèles 3D (KayKit Adventurers, CC0)
- Fichiers déjà dans le dépôt : `crates/errance_game/assets/kaykit/` (Knight, Mage, Rogue, Rogue_Hooded, Barbarian en .glb, armes .gltf/.bin, textures, `LICENSE_KayKit.txt`). 18 Mo. Aucun code ne les utilise encore.
- Source : https://github.com/KayKit-Game-Assets/KayKit-Character-Pack-Adventures-1.0 (le réseau du conteneur y accède).
- Plan validé (à coder dans un nouveau module `kaykit.rs`) :
  1. Intégrer les fichiers au binaire : `EmbeddedAssetRegistry::insert_asset(full_path, Path::new("kaykit/X"), include_bytes!(...))` pour chaque fichier (glb, gltf, bin, png), puis charger `embedded://kaykit/Knight.glb`. Pas de chargement de fichier externe sur Android.
  2. Bevy 0.20 : faire apparaître un modèle avec `WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(...)))` et l'observer `On<WorldInstanceReady>` (exemple : `bevy-0.20.1/examples/animation/animated_mesh.rs`).
  3. Corps selon le torse : plastron/cotte → Knight ; veste_cuir/gambison → Rogue (Rogue_Hooded si capuche) ; robe → Mage ; haillons/rien → Barbarian. Échelle ≈ 0,8 (modèles hauts de 2,2). Face +Z comme nos modèles.
  4. Une fois prêt : cacher toutes les pièces sauf `*_ArmLeft/ArmRight/Body/Head/LegLeft/LegRight` ; montrer casque (`Knight_Helmet` si casque*), chapeau (`Mage_Hat`), cape (`*_Cape` si cape) ; accrocher l'arme à l'os `handslot.r` (bouclier/tome à `handslot.l`) : epee/epee_longue → sword_1handed, claymore → sword_2handed, hache → axe_1handed, grande_hache → axe_2handed, dague(s) → dagger, arbalete → crossbow_1handed, arbalete_lourde → crossbow_2handed, bâtons/sceptre → staff, baguette → wand, tome → spellbook_open, bouclier_rond → shield_round, ecu → shield_badge. Les autres (masse, marteau, lance, arc, torche, orbe…) : garder `models::build_weapon` / `build_offhand` accrochés à l'os.
  5. Animations : les 76 clips sont identiques dans les 5 fichiers (même squelette) → un seul `AnimationGraph` depuis Knight.glb, indices : Idle 36, Unarmed_Idle 66, 2H_Melee_Idle 13, Running_A 48, Walking_A 72, Punch_A 68, Kick 67, 1H_Chop 0, 1H_Slice_Horizontal 2, 2H_Slice 9, Dualwield_Chop 31, 1H_Ranged_Shoot 6, 2H_Ranged_Shoot 16, Spellcast_Shoot 62, Spellcasting 63, Dodge_Backward 27, Hit_A 34, Death_A 23, Interact 37, Block 18, Cheer 22.
  6. Déclencheurs : `sync_views` (field.rs) calcule déjà l'état (bouge, incantation, mort, étourdi) → le copier dans un composant `KayRig` sur la racine du combattant. Les événements du combat (`SkillUsed` avec nom et tags, `Damage`, `Dodge`, `Death`) lancent les animations ponctuelles : « coup_de_pied » → Kick, sorts (tags Spell/Heal) → Spellcast_Shoot, esquive (Movement) → Dodge_Backward, attaque de base selon l'arme. Ne plus faire basculer la racine à la mort pour ces modèles (Death_A s'en charge). Récolte → Interact.
  7. Squelettes ennemis et créatures : restent procéduraux (KayKit a aussi un pack Skeletons CC0, plus tard).
- Puis : interface moins « placeholder » (chantier suivant choisi), et ville marchable (Adeh l'attendait).

### À tester par Adeh
- Fluidité avec la nouvelle herbe (si ça rame : baisser `density` dans `scenery.rs`).
- Le panneau d'escalade au doigt (taille des prises, lisibilité).
- Le journal du moteur reste affiché en bas à gauche : à masquer une fois tout validé.

### Environnement
- Le script d'environnement ne tourne pas : installer à la main `libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev libxkbcommon-x11-0 xvfb xdotool imagemagick pkg-config mesa-vulkan-drivers libgl1-mesa-dri` (+ `libegl1 libegl-mesa0 libgles2` pour tester OpenGL), boucle de réessais (miroir 504). Build Bevy ~20 min en arrière-plan.

## Consignes d'Adeh (valables jusqu'à nouvel ordre)
- Coder jusqu'à ce qu'Adeh dise d'arrêter. S'il est absent : avancer seul, sans questions.
- Un fil par chantier quand il est absent (relais par `create_session`, environnement `env_012MCygdvnM7WLQmJSPkCmzt`). Quand il est là : il teste sur téléphone et colle le journal.
- Ne jamais attendre une durée annoncée : lancer en arrière-plan et vérifier toutes les minutes.
- Durée de la CI (APK publié) : 5 à 7 min si les dépendances n'ont pas changé, 8 à 11 min si Cargo.toml / Cargo.lock ont bougé. Réflexe : attendre 5 min, puis vérifier chaque minute. Erreur 429 de Maven = passagère, relancer le job.
- Préférer le texte copié (bouton du journal) aux captures (coût en tokens).
- Récap quand il revient : négatif d'abord, bonnes surprises détaillées, quoi tester, mini-leçon Rust.

## Programme (chantiers dans l'ordre)
1. ~~CI + APK~~, ~~Récolte & artisanat~~, ~~Quête Gérald~~, ~~Escalade Givrecol~~, ~~Affichage Android~~, ~~Retours de jeu du 25/11~~, ~~Monde plus vivant~~.
2. **Vrais modèles 3D KayKit** (en cours, plan ci-dessus).
3. Interface plus belle (cadres, icônes de sorts, boutons).
4. Ville marchable.
5. Collines Sourdes & réputation (boss des Collines, remises au marché).
6. Puis `IDEES_EN_ATTENTE.md`.

## Réflexes
- Contenu : modifier le `.ron` puis `cargo test -p errance_core contenu`.
- Équilibrage : `cargo run -p errance_sim --release -- 200`.
- Tests du cœur pendant que Bevy compile : `CARGO_TARGET_DIR=<scratchpad>/tcore cargo test -p errance_core`.
- Démos : `ERRANCE_DEMO=town`, `field:<zone>`, `field:marches:gather` (+ `notap`), `field:givrecol:climb:0`, `autoclimb:1[:done]`, `character`, `market`, `town:copy`, `chore:<id>:paid`, quêtes (`tavern:rumor`, `quest:0:ambush`…).
- Captures : `ERRANCE_SAVE_DIR=<dossier> ERRANCE_DEMO=… ERRANCE_SCREENSHOT=x.png ERRANCE_FRAMES=30 xvfb-run -a -s "-screen 0 1280x640x24" ./target/debug/errance`.
- Clics réels : script `play.sh` (Xvfb + xdotool, `mousedown` / pause / `mouseup`), voir METHODE.

## Prompt de relais (à coller dans le nouveau projet)
```
On reprend Errance (RPG mobile Rust/Bevy, dépôt Adeh44/Game-claude-coded, branche claude/mobile-rpg-game-design-o55gg9). Tu as l'autorisation de commit et push sur cette branche.
Lis dans l'ordre : CLAUDE.md, docs/claude/METHODE.md, docs/claude/REFERENCE.md, docs/claude/IDEES_EN_ATTENTE.md, docs/claude/PROCHAINE_SESSION.md, et applique les « Consignes d'Adeh ».
Vérifie d'abord que le script d'environnement a tourné (ls target/debug/errance) ; sinon installe les libs listées dans PROCHAINE_SESSION.md (boucle de réessais en arrière-plan) puis lance cargo build -p errance_game en arrière-plan et vérifie toutes les minutes. Vérifie aussi que la CI GitHub du dernier commit est verte.
Reprends le chantier en cours « Vrais modèles 3D KayKit » (plan détaillé dans PROCHAINE_SESSION.md, fichiers déjà dans crates/errance_game/assets/kaykit/). Code jusqu'à ce que je te dise d'arrêter.
```
