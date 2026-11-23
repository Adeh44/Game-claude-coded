# Prochaine session

## État réel (23/11/2026, fin du chantier Récolte & artisanat)
- Version v0.8, branche `claude/mobile-rpg-game-design-o55gg9`, poussée. Release CI : `apk-latest`.
- Nouveau : récolte en expédition (arbres entaillés, filons, plantes, carcasses de bêtes, jauge), Voies Récolte et Forge (+6 talents), réserve de ressources, Atelier du forgeron en ville (recettes T1 → T8, qualité selon la Forge, vente des ressources).
- 46 tests verts (`cargo test -p errance_core`).
- Le script d'environnement n'avait PAS tourné ce matin-là (pas de `target/`, libs absentes) : installer à la main `libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev libxkbcommon-x11-0 xvfb xdotool imagemagick pkg-config` (miroir Ubuntu parfois en 504 : réessayer).
- Adeh n'a pas encore testé l'APK sur son téléphone.

## Consignes d'Adeh (valables jusqu'à nouvel ordre)
- Travailler jusqu'à ce qu'Adeh dise qu'il est revenu.
- Un fil par chantier (~45-60 min). En fin de chantier : docs → commit → push → `create_session` du fil suivant (même environnement `env_012MCygdvnM7WLQmJSPkCmzt`, même dépôt, même branche) avec le prompt de relais mis à jour, puis s'arrêter.
- Ne jamais attendre une durée annoncée : lancer en arrière-plan et vérifier toutes les 2 minutes.
- Quand Adeh revient : finir l'étape en cours, récap (négatif d'abord, bonnes surprises détaillées, quoi tester, mini-leçon Rust), commit, push.

## Programme (chantiers dans l'ordre)
1. ~~CI verte + APK~~ (fait).
2. ~~Récolte & artisanat~~ (fait, v0.8).
3. **Quête écrite** « Gérald le Borgne » (rumeur taverne → embuscade → repaire → boss, récompense unique).
4. Givrecol : passages d'escalade (mini-jeu de prises, endurance, froid).
5. Puis `IDEES_EN_ATTENTE.md`.

## Réflexes
- Contenu : modifier le `.ron` puis `cargo test -p errance_core contenu`.
- Équilibrage : `cargo run -p errance_sim --release -- 200`.
- Démo atelier : `ERRANCE_DEMO=workshop:res` (ressources de test). Récolte : `ERRANCE_DEMO=field:marches`.
- Captures : `ERRANCE_DEMO=… ERRANCE_SCREENSHOT=x.png ERRANCE_FRAMES=200 xvfb-run -a -s "-screen 0 1280x640x24" ./target/debug/errance` (mots-clés dans `demo_driver`, `crates/errance_game/src/lib.rs`).

## Prompt de relais (à coller ou envoyé par create_session)
```
On reprend Errance (RPG mobile Rust/Bevy, dépôt Adeh44/Game-claude-coded, branche claude/mobile-rpg-game-design-o55gg9). Tu as l'autorisation de commit et push sur cette branche.
Lis dans l'ordre : CLAUDE.md, docs/claude/METHODE.md, docs/claude/REFERENCE.md, docs/claude/IDEES_EN_ATTENTE.md, docs/claude/PROCHAINE_SESSION.md, et applique les « Consignes d'Adeh ».
Fais le chantier « Quête écrite : Gérald le Borgne ». Adeh dort/est absent : ne lui pose pas de questions, avance seul, et en fin de chantier passe le relais au fil suivant comme décrit.
```
