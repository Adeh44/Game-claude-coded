# Prochaine session

## État réel (23/11/2026, fin du chantier Quête écrite)
- Version v0.9, branche `claude/mobile-rpg-game-design-o55gg9`, poussée. Release CI : `apk-latest`. CI de cd27f3c (v0.8) verte.
- Nouveau : quête « Gérald le Borgne » (rumeur à la taverne → embuscade sur la route du sud → vieux fortin → Gérald, récompense L'Œil de Gérald + 150 po + réputation Gué-Rouge). Module `quest` + `quetes.ron`, nouvel ennemi « la Fouine ». Détails : REFERENCE.md, section Quêtes écrites.
- 53 tests verts. Le simulateur affiche aussi les rencontres de quête.
- Ce matin encore, le script d'environnement n'avait PAS tourné : libs installées à la main (liste ci-dessous), build Bevy ~14 min.
- Libs : `libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev libxkbcommon-x11-0 xvfb xdotool imagemagick pkg-config mesa-vulkan-drivers libgl1-mesa-dri` (miroir Ubuntu parfois en 504 : boucle de réessais).
- Adeh n'a pas encore testé l'APK sur son téléphone.

## Consignes d'Adeh (valables jusqu'à nouvel ordre)
- Travailler jusqu'à ce qu'Adeh dise qu'il est revenu.
- Un fil par chantier (~45-60 min). En fin de chantier : docs → commit → push → `create_session` du fil suivant (même environnement `env_012MCygdvnM7WLQmJSPkCmzt`, même dépôt, même branche) avec le prompt de relais mis à jour, puis s'arrêter.
- Ne jamais attendre une durée annoncée : lancer en arrière-plan et vérifier toutes les 2 minutes.
- Quand Adeh revient : finir l'étape en cours, récap (négatif d'abord, bonnes surprises détaillées, quoi tester, mini-leçon Rust), commit, push.

## Programme (chantiers dans l'ordre)
1. ~~CI verte + APK~~ (fait).
2. ~~Récolte & artisanat~~ (fait, v0.8).
3. ~~Quête écrite « Gérald le Borgne »~~ (fait, v0.9).
4. **Givrecol** : passages d'escalade (mini-jeu de prises, endurance, froid).
5. Puis `IDEES_EN_ATTENTE.md`.

## Réflexes
- Contenu : modifier le `.ron` puis `cargo test -p errance_core contenu`.
- Équilibrage : `cargo run -p errance_sim --release -- 200`.
- Démos quête : `ERRANCE_DEMO=tavern:rumor` (rumeur), `questpanel:rumor` (dialogue), `gate:quest:1` (porte), `quest:0:ambush` + `ERRANCE_FRAMES=24` (embuscade), `quest:1:stuff:fort` (fortin).
- Démo atelier : `ERRANCE_DEMO=workshop:res[:forge]` (ressources de test, forge une épée). Récolte : `ERRANCE_DEMO=field:marches:gather` + `ERRANCE_FRAMES=14` (le rendu logiciel est lent, l arbre est vidé vers 17 images) ; `ERRANCE_DEBUG_GATHER=1` trace la jauge.
- Captures : il faut aussi `mesa-vulkan-drivers libgl1-mesa-dri` (sinon panique « no adapter »).
- Captures : `ERRANCE_DEMO=… ERRANCE_SCREENSHOT=x.png ERRANCE_FRAMES=200 xvfb-run -a -s "-screen 0 1280x640x24" ./target/debug/errance` (mots-clés dans `demo_driver`, `crates/errance_game/src/lib.rs`).

## Prompt de relais (à coller ou envoyé par create_session)
```
On reprend Errance (RPG mobile Rust/Bevy, dépôt Adeh44/Game-claude-coded, branche claude/mobile-rpg-game-design-o55gg9). Tu as l'autorisation de commit et push sur cette branche.
Lis dans l'ordre : CLAUDE.md, docs/claude/METHODE.md, docs/claude/REFERENCE.md, docs/claude/IDEES_EN_ATTENTE.md, docs/claude/PROCHAINE_SESSION.md, et applique les « Consignes d'Adeh ».
Vérifie d'abord que le script d'environnement a tourné (ls target/debug/errance) ; sinon installe les libs listées dans PROCHAINE_SESSION.md (boucle de réessais en arrière-plan) puis lance cargo build -p errance_game en arrière-plan et vérifie toutes les 2 min. Vérifie aussi que la CI GitHub du dernier commit est verte (sinon corrige d'abord).
Fais le chantier « Givrecol : passages d'escalade ». Adeh dort/est absent : ne lui pose pas de questions, avance seul, et en fin de chantier passe le relais au fil suivant comme décrit.
```
