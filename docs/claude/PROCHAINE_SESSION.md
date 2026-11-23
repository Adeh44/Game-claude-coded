# Prochaine session

## État réel (23/11/2026, fin de la partie a)
- Version v0.7, branche `claude/mobile-rpg-game-design-o55gg9`, **poussée**. La CI est verte (tests + APK). Release : `apk-latest`.
- Environnement cloud « Game-claude-coded » : son script installe les libs (alsa, udev, xkbcommon, xvfb, xdotool, imagemagick, clang, libc6-dev-arm64-cross), met Rust à jour et précompile le jeu. Vérifier qu'il a bien tourné : `ls target/debug/errance`.
- 38 tests verts (`cargo test -p errance_core`).
- Adeh n'a pas encore testé l'APK sur son téléphone.

## Consignes d'Adeh (valables jusqu'à nouvel ordre)
- Travailler jusqu'à ce qu'Adeh dise qu'il est revenu.
- Un fil par chantier (~45-60 min). En fin de chantier : docs → commit → push → `create_session` du fil suivant (même environnement `env_012MCygdvnM7WLQmJSPkCmzt`, même dépôt, même branche) avec le prompt de relais mis à jour, puis s'arrêter.
- Ne jamais attendre une durée annoncée : lancer en arrière-plan et vérifier toutes les 2 minutes.
- Quand Adeh revient : finir l'étape en cours, récap (négatif d'abord, bonnes surprises détaillées, quoi tester, mini-leçon Rust), commit, push.

## Programme (chantiers dans l'ordre)
1. ~~CI verte + APK~~ (fait).
2. **Récolte & artisanat** (boucle à la Albion) :
   - nœuds dans les zones (arbres → bois, filons → minerai, carcasses de bêtes → peaux), récoltés en restant près d'eux (jauge) ; quantité et tier selon la zone et la maîtrise ;
   - nouvelles Voies `recolte` et `forge` (catégorie Job) avec talents ;
   - ressources empilables dans le sac ;
   - atelier en ville : recette = base + ressources du tier, qualité tirée avec bonus de maîtrise Forge ;
   - tests core + test de contenu, captures.
3. Quête écrite « Gérald le Borgne » (rumeur taverne → embuscade → repaire → boss, récompense unique).
4. Givrecol : passages d'escalade (mini-jeu de prises, endurance, froid).
5. Puis `IDEES_EN_ATTENTE.md`.

## Réflexes
- Contenu : modifier le `.ron` puis `cargo test -p errance_core contenu`.
- Équilibrage : `cargo run -p errance_sim --release -- 200`.
- Captures : `ERRANCE_DEMO=… ERRANCE_SCREENSHOT=x.png ERRANCE_FRAMES=200 xvfb-run -a -s "-screen 0 1280x640x24" ./target/debug/errance` (mots-clés dans `demo_driver`, `crates/errance_game/src/lib.rs`).

## Prompt de relais (à coller ou envoyé par create_session)
```
On reprend Errance (RPG mobile Rust/Bevy, dépôt Adeh44/Game-claude-coded, branche claude/mobile-rpg-game-design-o55gg9). Tu as l'autorisation de commit et push sur cette branche.
Lis dans l'ordre : CLAUDE.md, docs/claude/METHODE.md, docs/claude/REFERENCE.md, docs/claude/IDEES_EN_ATTENTE.md, docs/claude/PROCHAINE_SESSION.md, et applique les « Consignes d'Adeh ».
Fais le chantier « Récolte & artisanat ». Adeh dort/est absent : ne lui pose pas de questions, avance seul, et en fin de chantier passe le relais au fil suivant comme décrit.
```
