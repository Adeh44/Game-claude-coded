# Prochaine session

## État réel (fin de nuit 22/11/2026)
- Version : v0.7, branche `claude/mobile-rpg-game-design-o55gg9`.
- **Tout est commité en local, rien n'est poussé** : push refusé (403, app Claude GitHub absente du dépôt). Si la machine cloud a été recyclée, le travail est perdu sauf si le push a réussi entre-temps. Vérifier en premier.
- APK : jamais construit (la CI n'a jamais tourné). Projet Android « prédit », pas « mesuré ».
- Tests : `cargo test -p errance_core` → 38 tests verts. Clippy propre sur core + sim.
- Jeu testé sur PC uniquement (rendu logiciel, souris simulée).

## Programme (dans l'ordre)
1. Adeh installe l'app Claude sur le dépôt (claude.ai/connect-github) → `git push` → regarder la CI (job « APK Android »).
2. Si l'APK se construit : release `apk-latest` → installer sur le téléphone → playtest (noter en vrac, on trie avec le skill epinoia-playtest adapté).
3. Corriger ce que le téléphone révèle : taille/emplacement des boutons, perf (ombres coupées sur Android, décor réduit), toucher multi-doigts.
4. Si l'APK échoue : lire le log CI ; pistes connues → version de `games-activity` vs crate `android-activity`, NDK, nom de la lib (`errance_game`).

## Réflexes
- Avant d'ajouter du contenu : modifier le `.ron`, puis `cargo test -p errance_core contenu`.
- Avant de toucher à l'équilibrage : `cargo run -p errance_sim --release -- 200`.
- Captures sans téléphone : `ERRANCE_DEMO=field:marches:auto:stuff:party:brawl ERRANCE_SCREENSHOT=x.png ERRANCE_FRAMES=200 xvfb-run -a -s "-screen 0 1280x640x24" ./target/debug/errance` (voir `demo_driver` dans `crates/errance_game/src/lib.rs` pour les mots-clés).
- Après `cargo build`, première compilation Bevy ≈ 13 min : la lancer en arrière-plan.

## Documents périmés
Aucun connu.

## Prompt de relais (à coller)
```
On reprend Errance (RPG mobile Rust/Bevy). Lis dans l'ordre : CLAUDE.md, docs/claude/METHODE.md, docs/claude/REFERENCE.md, docs/claude/IDEES_EN_ATTENTE.md, docs/claude/PROCHAINE_SESSION.md.
Je t'autorise à rattacher et cloner Adeh44/Game-claude-coded, push compris, sur la branche claude/mobile-rpg-game-design-o55gg9.
Questions d'ouverture en quiz : sur quelle machine je suis, est-ce que le push/la CI ont marché, est-ce que j'ai pu installer l'APK.
Programme : faire passer la CI, puis corriger ce que le premier test sur téléphone révèle.
```
