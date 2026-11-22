# Méthode — Errance

## Questionnaire de départ (22/11/2026)
- Machines : session Claude Code cloud + téléphone Android pour jouer. Documents dans le dépôt (`docs/claude/`).
- Versionnement : git, branche `claude/mobile-rpg-game-design-o55gg9`. Adeh a autorisé commit + push à chaque jalon en validant le plan.
- Mode : vitesse (Claude code seul, souvent la nuit) + recap ludique le matin avec une mini-leçon Rust.
- Outils : Rust (stable), Bevy 0.20.1, Android via cargo-ndk + Gradle (GameActivity) sur GitHub Actions. Pas d'éditeur graphique.
- Profil : profil habituel d'Adeh (quiz, messages courts, direct, chaleureux, pas de gras pour souligner).

## Règles propres au projet
- Toute règle du jeu vit dans `crates/errance_core` (zéro Bevy) et se teste. Le jeu Bevy n'est qu'un affichage + les gestes.
- Le contenu (skills, objets, talents, ennemis) est en RON dans `crates/errance_core/content/`, embarqué à la compilation (`include_str!`) : aucun fichier à charger sur Android.
- Un test (`content_valide`) vérifie toutes les références du contenu : on le lance après chaque ajout.
- Aléatoire uniquement via `errance_core::rng::Rng` (à graine) : tout combat se rejoue.
- Polices embarquées (accents français). Pas d'asset externe obligatoire.
- Versions : v0.1, v0.2… C'est Adeh qui déclare la v1.0.

## Routine git
1. `git status` + `git diff --stat` lus.
2. `git add -A && git commit -m "vX.Y : …" && git push -u origin claude/mobile-rpg-game-design-o55gg9`, lire la ligne de retour.

## Pièges déjà payés
| Symptôme | Cause | Réflexe |
|---|---|---|
| `bevy@0.20.1 requires rustc 1.97.1` | Rust du conteneur trop vieux | `rustup update stable` avant le premier build |
| `dl.google.com` refusé (403 proxy) | Réseau du conteneur : pas de SDK Android ici | L'APK se construit uniquement sur GitHub Actions |
| Build Bevy > 10 min | Première compilation complète | Lancer en arrière-plan, coder le core pendant ce temps |
