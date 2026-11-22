# Errance — RPG mobile solo (Rust + Bevy, Android)

Mémoire du projet : `docs/claude/`. Ordre de lecture :
1. `docs/claude/METHODE.md` — façon de travailler, pièges déjà payés
2. `docs/claude/REFERENCE.md` — design et décisions
3. `docs/claude/IDEES_EN_ATTENTE.md`
4. `docs/claude/PROCHAINE_SESSION.md` — toujours en dernier

Commandes :
- Tests des règles : `cargo test -p errance_core -p errance_sim`
- Jeu sur PC : `cargo run -p errance_game` (ZQSD/WASD + souris, 1-6 skills)
- Équilibrage : `cargo run -p errance_sim --release`
- APK : construit par GitHub Actions (`.github/workflows/ci.yml`), release `apk-latest`.
