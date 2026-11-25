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

## Boîte noire (plantages sur le téléphone)
- Une panique Rust est écrite dans `plantage.txt` (message, fichier:ligne, fil, étape) ; l'étape en cours est notée dans `etape.txt` (dossier de sauvegarde). Au lancement suivant, l'écran titre affiche le rapport en rouge : Adeh n'a qu'à faire une capture.
- Test sur PC : `ERRANCE_SAVE_DIR=dossier ERRANCE_TEST_PANIC=1` (panique en ville après 4 s), puis relancer.
- Journal du moteur : `journal.rs` capte erreurs / alertes Bevy-wgpu + la fiche du GPU, les écrit dans `journal.txt` et les affiche en petit en bas à gauche de l'écran (toujours visible pour l'instant : à masquer une fois le téléphone d'Adeh réglé). Bouton « Copier le journal pour Claude » : rapport texte (version, étape, dernier plantage, moteur, GPU, erreurs) dans le presse-papiers Android via JNI (`clipboard.rs`) ; sur PC, `journal_copie.txt`. Démo : `ERRANCE_DEMO=town:copy`. Adeh préfère coller le texte plutôt qu'envoyer des captures.
- Clics réels : `play.sh` doit faire `mousedown`, pause 0,6 s, `mouseup` (un `click` est trop rapide pour le rendu logiciel). Bouton « Nouvelle partie » à (640, 380) en 1280×640.

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
| `quetes.ron: Expected option` | Champ `Option<f32>` écrit `size: 24.0` | En RON une option s'écrit `Some(24.0)` |
| Simulateur : 100 % → 0 % en ajoutant un ennemi T1 | L'IA du héros se fait encercler à 3 | Normal : régler les groupes de quête au simulateur (`quête :` en bas de chaque build) |
| Une action poussée par la démo (`queue.0.push`) n'arrive jamais | `UiQueue` est vidée en PostUpdate ; si la démo tourne après le système qui lit la file, l'action est perdue | `demo_driver` tourne en PreUpdate (corrigé le 23/11) |
| Téléphone (Redmi, Android 16) : plantage quelques secondes après « Nouvelle partie », `entered unreachable code` dans `wgpu-hal/src/vulkan/descriptor.rs:208` | Le pilote Vulkan renvoie `ERROR_FRAGMENTED_POOL`, que wgpu-hal 30.0.2 croit impossible | Copie corrigée dans `vendor/wgpu-hal` (`[patch.crates-io]` du Cargo.toml racine) : réserve marquée pleine, nouvelle réserve, nouvel essai. À retirer quand wgpu corrigera (vérifier la version de wgpu-hal à chaque montée de Bevy) |
| Téléphone : plus de plantage mais écran jamais effacé (traînées, menu de ville par-dessus les panneaux, 3D absente) | Avec le patch, les allocations Vulkan échouent en silence : la caméra 3D ne dessine plus | OpenGL plante au démarrage sur ce Redmi : impasse. Piste suivie : matériaux « bindless » (tableaux de textures) désactivés partout via `disabled_features` dans `render_plugin()`. Moteur auto (Vulkan), bouton sur le titre + garde-fou conservés |
| OpenGL impossible à tester sur PC (Xvfb : « failed to choose present mode ») | Pas d'EGL fenêtré dans Xvfb | Tester OpenGL seulement sur le téléphone ; sur PC c'est un bon test du garde-fou |
| Un talent sans effet | Stat `ItemPower` jamais lue par le moteur | Utiliser des stats réellement appliquées (Armor, PhysicalPower…) |
