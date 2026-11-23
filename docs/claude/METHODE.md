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

## Outils de test sans téléphone
- Captures : variables `ERRANCE_SCREENSHOT`, `ERRANCE_FRAMES`, `ERRANCE_DEMO` (voir `demo_driver`).
- Clics réels : script `play.sh` (Xvfb + xdotool) dans le scratchpad de la session (à recréer : lance le jeu, clique aux coordonnées, `import` pour capturer).
- Sons : `ERRANCE_DUMP_AUDIO=dossier` écrit la musique en .wav.

## Routine git
1. `git status` + `git diff --stat` lus.
2. `git add -A && git commit -m "vX.Y : …" && git push -u origin claude/mobile-rpg-game-design-o55gg9`, lire la ligne de retour.

## Pièges déjà payés
| Symptôme | Cause | Réflexe |
|---|---|---|
| `bevy@0.20.1 requires rustc 1.97.1` | Rust du conteneur trop vieux | `rustup update stable` avant le premier build |
| `dl.google.com` refusé (403 proxy) | Réseau du conteneur : pas de SDK Android ici | L'APK se construit uniquement sur GitHub Actions |
| Build Bevy > 10 min | Première compilation complète | Lancer en arrière-plan, coder le core pendant ce temps |
| `git push` → 403 « Claude doesn't have GitHub access » | App Claude GitHub pas installée sur le dépôt | Adeh : claude.ai/connect-github ; en attendant, commits locaux |
| Panique B0002 au lancement | Un système avec `Res<X>` ET `ResMut<X>` | Un seul `ResMut<X>` par système |
| Panique « duplicate components » | Un bundle contenant deux `Node` (bouton + position) | Envelopper dans un nœud parent |
| Symboles (★ ✓ ◇ 🔒 ⟳) invisibles | Absents de la police Alegreya Sans | Rester en texte simple (*, ok, -, (verrou)) |
| Le jeu panique sous Xvfb : `libxkbcommon-x11.so` | Lib manquante dans le conteneur | `apt-get install libxkbcommon-x11-0` |
| `cargo check --target aarch64-linux-android` échoue (blake3, android-activity) | Pas de NDK ici (dl.google.com bloqué) | Vérif possible avec le backend `android-native-activity` + `CC_aarch64_linux_android=clang` + `CFLAGS_aarch64_linux_android="--target=aarch64-linux-gnu -isystem /usr/aarch64-linux-gnu/include"` (paquet `libc6-dev-arm64-cross`). Ne pas commiter le changement de backend. |
| Bouton qui n'agit pas sur le skill | Les boutons de skill doivent réagir à l'appui | `ActivateOnPress` (`ui::instant()`) |
| `wayland-sys` : build script échoue, pas de `target/` au démarrage | Le script d'environnement n'a pas tourné | `apt-get install` des libs (liste dans PROCHAINE_SESSION) ; miroir en 504 → boucle de 5 essais en arrière-plan |
| Un talent sans effet | Stat `ItemPower` jamais lue par le moteur | Utiliser des stats réellement appliquées (Armor, PhysicalPower…) |
