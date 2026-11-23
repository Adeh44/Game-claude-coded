# Prochaine session

## État réel (23/11/2026, fin du chantier Givrecol)
- Version v0.10, branche `claude/mobile-rpg-game-design-o55gg9`, poussée. Release CI : `apk-latest`. CI de c1f4fe9 (v0.9) verte.
- Nouveau : passages d'escalade à Givrecol (T5). Une barre de rochers coupe la zone, la Bête est au-dessus. Deux voies (Couloir des Chamois T4 13 m, Directe du Givre T5 10 m). Panneau de paroi : prises à toucher, Bras, Chaleur, pitons, chutes retenues ou au sol ; rappel pour redescendre. Voie Escalade (métier) + 3 talents. Module `climb` (cœur) et `climb.rs` (jeu). Détails : REFERENCE.md, section Escalade.
- 56 tests verts, clippy propre. Le simulateur affiche aussi l'escalade (robot grimpeur par tenue et niveau).
- `demo_driver` tourne maintenant en PreUpdate (sinon ses actions étaient perdues, voir METHODE).
- Le script d'environnement n'avait encore PAS tourné : libs installées à la main (liste ci-dessous), build Bevy ~21 min.
- Libs : `libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev libxkbcommon-x11-0 xvfb xdotool imagemagick pkg-config mesa-vulkan-drivers libgl1-mesa-dri` (miroir Ubuntu parfois en 504 : boucle de réessais).
- 23/11 : premier test d'Adeh sur téléphone (Redmi, Android 16) : plantage quelques secondes après « Nouvelle partie ». La boîte noire a donné la cause : bug wgpu-hal + pilote Vulkan (`ERROR_FRAGMENTED_POOL`). Corrigé par un wgpu-hal patché dans `vendor/` (voir METHODE). À confirmer par Adeh sur le téléphone.
- À tester ensuite : le panneau d'escalade au doigt (taille des prises, lisibilité).

## Consignes d'Adeh (valables jusqu'à nouvel ordre)
- Travailler jusqu'à ce qu'Adeh dise qu'il est revenu.
- Un fil par chantier (~45-60 min). En fin de chantier : docs → commit → push → `create_session` du fil suivant (même environnement `env_012MCygdvnM7WLQmJSPkCmzt`, même dépôt, même branche) avec le prompt de relais mis à jour, puis s'arrêter.
- Ne jamais attendre une durée annoncée : lancer en arrière-plan et vérifier toutes les 2 minutes.
- Quand Adeh revient : finir l'étape en cours, récap (négatif d'abord, bonnes surprises détaillées, quoi tester, mini-leçon Rust), commit, push.

## Programme (chantiers dans l'ordre)
1. ~~CI verte + APK~~ (fait).
2. ~~Récolte & artisanat~~ (fait, v0.8).
3. ~~Quête écrite « Gérald le Borgne »~~ (fait, v0.9).
4. ~~Givrecol : passages d'escalade~~ (fait, v0.10).
5. **Collines Sourdes & réputation** : un nouveau boss pour les Collines (zone sans boss depuis la quête), et la réputation de Gué-Rouge qui sert enfin (remises au marché selon la réputation, une ligne dans la Fiche, peut-être un garde qui salue).
6. Puis `IDEES_EN_ATTENTE.md`.

## Réflexes
- Contenu : modifier le `.ron` puis `cargo test -p errance_core contenu`.
- Équilibrage : `cargo run -p errance_sim --release -- 200`.
- Tests du cœur pendant que Bevy compile : `CARGO_TARGET_DIR=<scratchpad>/tcore cargo test -p errance_core` (pas de verrou partagé avec le build du jeu).
- Démos escalade : `ERRANCE_DEMO=field:givrecol:climb:0` (au pied, panneau ouvert), `field:givrecol:autoclimb:1` + `ERRANCE_FRAMES=110` (le robot grimpe la Directe), `autoclimb:1:done` + `ERRANCE_FRAMES=200` (au sommet, bouton Rappel).
- Démos quête : `ERRANCE_DEMO=tavern:rumor` (rumeur), `questpanel:rumor` (dialogue), `gate:quest:1` (porte), `quest:0:ambush` + `ERRANCE_FRAMES=24` (embuscade), `quest:1:stuff:fort` (fortin).
- Démo atelier : `ERRANCE_DEMO=workshop:res[:forge]`. Récolte : `ERRANCE_DEMO=field:marches:gather` + `ERRANCE_FRAMES=14` ; `ERRANCE_DEBUG_GATHER=1` trace la jauge.
- Captures : il faut aussi `mesa-vulkan-drivers libgl1-mesa-dri` (sinon panique « no adapter »).
- Captures : `ERRANCE_DEMO=… ERRANCE_SCREENSHOT=x.png ERRANCE_FRAMES=200 xvfb-run -a -s "-screen 0 1280x640x24" ./target/debug/errance` (mots-clés dans `demo_driver`, `crates/errance_game/src/lib.rs`).

## Prompt de relais (à coller ou envoyé par create_session)
```
On reprend Errance (RPG mobile Rust/Bevy, dépôt Adeh44/Game-claude-coded, branche claude/mobile-rpg-game-design-o55gg9). Tu as l'autorisation de commit et push sur cette branche.
Lis dans l'ordre : CLAUDE.md, docs/claude/METHODE.md, docs/claude/REFERENCE.md, docs/claude/IDEES_EN_ATTENTE.md, docs/claude/PROCHAINE_SESSION.md, et applique les « Consignes d'Adeh » (un fil par chantier, vérifier toutes les 2 minutes plutôt qu'attendre, travailler jusqu'à ce qu'Adeh dise qu'il est revenu).
Vérifie d'abord que le script d'environnement a tourné (ls target/debug/errance) ; sinon installe les libs listées dans PROCHAINE_SESSION.md (boucle de réessais en arrière-plan, le miroir Ubuntu renvoie parfois 504) puis lance cargo build -p errance_game en arrière-plan et vérifie toutes les 2 min. Vérifie aussi que la CI GitHub du dernier commit est verte (sinon corrige d'abord).
Fais le chantier « Collines Sourdes & réputation ». Adeh est absent : ne lui pose pas de questions, avance seul, et en fin de chantier passe le relais au fil suivant (create_session sur l'environnement env_012MCygdvnM7WLQmJSPkCmzt, même dépôt et branche) avec le prompt de relais mis à jour pour le chantier suivant.
```
