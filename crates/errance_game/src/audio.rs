//! Sons et musique fabriqués par le code (aucun fichier audio).
//!
//! Bruitages : bruit filtré + enveloppes. Musique de ville : un luth joué
//! par synthèse Karplus-Strong (une corde qu'on pince, simulée).

use bevy::audio::Volume;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use std::sync::Arc;

const RATE: u32 = 22050;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Sfx {
    Hit,
    Crit,
    Swing,
    Spell,
    Heal,
    Shield,
    Coin,
    LevelUp,
    Death,
    Click,
    Boom,
}

#[derive(Message, Clone, Copy)]
pub struct PlaySfx(pub Sfx);

#[derive(Resource)]
pub struct Sounds {
    map: HashMap<Sfx, Handle<AudioSource>>,
    town_music: Handle<AudioSource>,
}

#[derive(Resource)]
pub struct AudioPrefs {
    pub music: bool,
    pub sfx: bool,
}

impl Default for AudioPrefs {
    fn default() -> Self {
        Self { music: true, sfx: true }
    }
}

#[derive(Component)]
struct Music;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PlaySfx>()
            .init_resource::<AudioPrefs>()
            .add_systems(Startup, build_sounds)
            .add_systems(Update, (play_sfx, audio_toggles))
            .add_systems(OnEnter(crate::AppState::Town), start_town_music)
            .add_systems(OnExit(crate::AppState::Town), stop_music);
    }
}

fn wav(samples: &[f32]) -> AudioSource {
    let n = samples.len() as u32;
    let mut b = Vec::with_capacity(44 + samples.len() * 2);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + n * 2).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&RATE.to_le_bytes());
    b.extend_from_slice(&(RATE * 2).to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&(n * 2).to_le_bytes());
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32000.0) as i16;
        b.extend_from_slice(&v.to_le_bytes());
    }
    AudioSource { bytes: Arc::from(b.into_boxed_slice()) }
}

struct Noise(u32);
impl Noise {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

fn len(secs: f32) -> usize {
    (secs * RATE as f32) as usize
}

fn tone(freq: f32, secs: f32, decay: f32, vol: f32) -> Vec<f32> {
    (0..len(secs))
        .map(|i| {
            let t = i as f32 / RATE as f32;
            (t * freq * std::f32::consts::TAU).sin() * (-t * decay).exp() * vol * (t * 400.0).min(1.0)
        })
        .collect()
}

fn mix(into: &mut Vec<f32>, src: &[f32], at: f32) {
    let off = len(at);
    if into.len() < off + src.len() {
        into.resize(off + src.len(), 0.0);
    }
    for (i, s) in src.iter().enumerate() {
        into[off + i] += s;
    }
}

/// Bruit filtré (passe-bas simple) avec enveloppe.
fn noise_burst(secs: f32, decay: f32, smooth: f32, vol: f32, seed: u32) -> Vec<f32> {
    let mut n = Noise(seed);
    let mut lp = 0.0;
    (0..len(secs))
        .map(|i| {
            let t = i as f32 / RATE as f32;
            lp += (n.next() - lp) * smooth;
            lp * (-t * decay).exp() * vol
        })
        .collect()
}

/// Corde pincée (Karplus-Strong).
fn pluck(freq: f32, secs: f32, vol: f32, seed: u32) -> Vec<f32> {
    let period = (RATE as f32 / freq).round().max(2.0) as usize;
    let mut n = Noise(seed);
    let mut buf: Vec<f32> = (0..period).map(|_| n.next()).collect();
    let mut out = Vec::with_capacity(len(secs));
    let mut idx = 0;
    for _ in 0..len(secs) {
        let a = buf[idx];
        let b = buf[(idx + 1) % period];
        let v = (a + b) * 0.4985;
        buf[idx] = v;
        out.push(a * vol);
        idx = (idx + 1) % period;
    }
    out
}

fn sweep(f0: f32, f1: f32, secs: f32, vol: f32) -> Vec<f32> {
    let n = len(secs);
    let mut phase = 0.0f32;
    (0..n)
        .map(|i| {
            let k = i as f32 / n as f32;
            let f = f0 + (f1 - f0) * k;
            phase += f / RATE as f32 * std::f32::consts::TAU;
            phase.sin() * (1.0 - k) * vol * (k * 30.0).min(1.0)
        })
        .collect()
}

fn note(semi: i32) -> f32 {
    // La2 = 220 Hz.
    220.0 * 2f32.powf(semi as f32 / 12.0)
}

fn build_sounds(mut commands: Commands, mut audio: ResMut<Assets<AudioSource>>) {
    let mut map = HashMap::default();
    let mut add = |k: Sfx, s: Vec<f32>| {
        map.insert(k, audio.add(wav(&s)));
    };
    // Coup : choc sourd + claquement.
    let mut hit = noise_burst(0.12, 40.0, 0.35, 0.6, 7);
    mix(&mut hit, &tone(90.0, 0.12, 30.0, 0.5), 0.0);
    add(Sfx::Hit, hit);
    let mut crit = noise_burst(0.2, 25.0, 0.5, 0.7, 11);
    mix(&mut crit, &tone(70.0, 0.2, 18.0, 0.6), 0.0);
    mix(&mut crit, &tone(880.0, 0.12, 30.0, 0.15), 0.02);
    add(Sfx::Crit, crit);
    // Fendre l'air.
    add(Sfx::Swing, noise_burst(0.18, 14.0, 0.08, 0.5, 3));
    // Sort : montée scintillante.
    let mut spell = sweep(300.0, 900.0, 0.3, 0.25);
    mix(&mut spell, &noise_burst(0.3, 10.0, 0.6, 0.08, 5), 0.0);
    add(Sfx::Spell, spell);
    // Soin : petit carillon.
    let mut heal = vec![];
    for (k, s) in [12, 16, 19].iter().enumerate() {
        mix(&mut heal, &tone(note(*s) * 2.0, 0.5, 6.0, 0.18), k as f32 * 0.06);
    }
    add(Sfx::Heal, heal);
    let mut shield = tone(note(7) * 2.0, 0.35, 8.0, 0.18);
    mix(&mut shield, &tone(note(14) * 2.0, 0.35, 8.0, 0.12), 0.02);
    add(Sfx::Shield, shield);
    // Pièce : deux tintements.
    let mut coin = tone(1760.0, 0.15, 25.0, 0.2);
    mix(&mut coin, &tone(2349.0, 0.25, 18.0, 0.2), 0.07);
    add(Sfx::Coin, coin);
    // Niveau gagné : arpège majeur au luth.
    let mut lvl = vec![];
    for (k, s) in [0, 4, 7, 12, 16].iter().enumerate() {
        mix(&mut lvl, &pluck(note(*s + 12), 1.2, 0.35, 100 + k as u32), k as f32 * 0.09);
    }
    add(Sfx::LevelUp, lvl);
    // Mort : grave qui descend.
    let mut death = sweep(160.0, 50.0, 0.5, 0.4);
    mix(&mut death, &noise_burst(0.4, 8.0, 0.1, 0.25, 9), 0.0);
    add(Sfx::Death, death);
    add(Sfx::Click, tone(660.0, 0.05, 60.0, 0.15));
    let mut boom = noise_burst(0.5, 7.0, 0.12, 0.7, 13);
    mix(&mut boom, &tone(55.0, 0.5, 6.0, 0.6), 0.0);
    add(Sfx::Boom, boom);

    // Musique de ville : un luth en ré dorien, 16 temps, en boucle.
    let beat = 0.42;
    let melody: [(i32, f32); 32] = [
        (5, 1.0), (8, 0.7), (12, 0.8), (8, 0.6),
        (10, 1.0), (8, 0.6), (7, 0.8), (5, 0.6),
        (3, 1.0), (7, 0.7), (10, 0.8), (7, 0.6),
        (5, 1.0), (12, 0.6), (10, 0.7), (8, 0.5),
        (3, 1.0), (7, 0.7), (10, 0.8), (7, 0.6),
        (8, 1.0), (7, 0.6), (5, 0.8), (3, 0.6),
        (2, 1.0), (5, 0.7), (8, 0.8), (5, 0.6),
        (3, 1.0), (10, 0.6), (8, 0.7), (7, 0.5),
    ];
    let bass = [5, 5, 3, 0, 3, 3, 2, 5];
    let mut music = vec![0.0; len(beat * 32.0 + 3.0)];
    for (k, (s, v)) in melody.iter().enumerate() {
        mix(&mut music, &pluck(note(*s), 1.6, 0.22 * v, 500 + k as u32), k as f32 * beat);
        if k % 4 == 3 {
            mix(&mut music, &pluck(note(*s + 7), 1.0, 0.08, 900 + k as u32), k as f32 * beat + beat * 0.5);
        }
    }
    for (k, b) in bass.iter().enumerate() {
        mix(&mut music, &pluck(note(*b - 12), 3.0, 0.25, 700 + k as u32), k as f32 * beat * 4.0);
    }
    music.truncate(len(beat * 32.0));
    // Fondu aux raccords de boucle.
    let fade = len(0.02);
    let n = music.len();
    for i in 0..fade {
        let k = i as f32 / fade as f32;
        music[i] *= k;
        music[n - 1 - i] *= k;
    }
    if let Ok(dir) = std::env::var("ERRANCE_DUMP_AUDIO") {
        let _ = std::fs::write(format!("{dir}/musique_ville.wav"), &*wav(&music).bytes);
        let _ = std::fs::write(format!("{dir}/niveau.wav"), &*map.get(&Sfx::LevelUp).map(|_| wav(&{
            let mut lvl = vec![];
            for (k, s) in [0, 4, 7, 12, 16].iter().enumerate() {
                mix(&mut lvl, &pluck(note(*s + 12), 1.2, 0.35, 100 + k as u32), k as f32 * 0.09);
            }
            lvl
        })).unwrap().bytes);
    }
    let town_music = audio.add(wav(&music));
    commands.insert_resource(Sounds { map, town_music });
}

fn play_sfx(mut commands: Commands, mut reader: MessageReader<PlaySfx>, sounds: Option<Res<Sounds>>, prefs: Res<AudioPrefs>, time: Res<Time>, mut last: Local<HashMap<Sfx, f32>>) {
    let Some(sounds) = sounds else { return };
    let now = time.elapsed_secs();
    for PlaySfx(k) in reader.read() {
        if !prefs.sfx {
            continue;
        }
        // Pas plus d'un même son toutes les 70 ms.
        if last.get(k).map(|t| now - t < 0.07).unwrap_or(false) {
            continue;
        }
        last.insert(*k, now);
        if let Some(h) = sounds.map.get(k) {
            let vol = match k {
                Sfx::Hit | Sfx::Swing => 0.55,
                Sfx::Click => 0.4,
                _ => 0.8,
            };
            commands.spawn((AudioPlayer::new(h.clone()), PlaybackSettings::DESPAWN.with_volume(Volume::Linear(vol))));
        }
    }
}

fn start_town_music(mut commands: Commands, sounds: Option<Res<Sounds>>, prefs: Res<AudioPrefs>) {
    let Some(sounds) = sounds else { return };
    if !prefs.music {
        return;
    }
    commands.spawn((AudioPlayer::new(sounds.town_music.clone()), PlaybackSettings::LOOP.with_volume(Volume::Linear(0.45)), Music));
}

fn stop_music(mut commands: Commands, q: Query<Entity, With<Music>>) {
    for e in q.iter() {
        commands.entity(e).despawn();
    }
}

fn audio_toggles(
    mut commands: Commands,
    mut queue: ResMut<crate::ui::UiQueue>,
    mut prefs: ResMut<AudioPrefs>,
    music: Query<Entity, With<Music>>,
    sounds: Option<Res<Sounds>>,
    state: Res<State<crate::AppState>>,
    mut toasts: ResMut<crate::Toasts>,
) {
    let mut keep = vec![];
    for a in queue.take() {
        match a {
            crate::ui::UiAction::ToggleMusic => {
                prefs.music = !prefs.music;
                for e in music.iter() {
                    commands.entity(e).despawn();
                }
                if prefs.music && *state.get() == crate::AppState::Town {
                    if let Some(s) = &sounds {
                        commands.spawn((AudioPlayer::new(s.town_music.clone()), PlaybackSettings::LOOP.with_volume(Volume::Linear(0.45)), Music));
                    }
                }
                toasts.push(if prefs.music { "Musique activée".into() } else { "Musique coupée".into() });
            }
            crate::ui::UiAction::ToggleSfx => {
                prefs.sfx = !prefs.sfx;
                toasts.push(if prefs.sfx { "Bruitages activés".into() } else { "Bruitages coupés".into() });
            }
            other => keep.push(other),
        }
    }
    queue.0.extend(keep);
}
