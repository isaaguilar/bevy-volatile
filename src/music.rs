use std::{io::Cursor, sync::Arc, time::Duration};

use bevy::{
    asset::LoadState,
    audio::{AddAudioSource, ChannelCount, Decodable, Sample, SampleRate, Source, Volume},
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task, futures::check_ready, futures_lite::future::yield_now},
};

const INTRO_PATH: &str = "sfx/TlalocRock_intro.ogg";
const LOOP_PATH: &str = "sfx/TlalocRock_loop.ogg";

/// Register after Bevy's default plugins. Playback is independent of game states.
pub fn plugin(app: &mut App) {
    app.add_audio_source::<Music>()
        .add_systems(Startup, load_music)
        .add_systems(Update, prepare_music);
}

#[derive(Resource)]
struct MusicLoading {
    intro: Handle<AudioSource>,
    loop_track: Handle<AudioSource>,
    intro_failed: bool,
    loop_failed: bool,
    started: bool,
    task: Option<Task<Result<Music, String>>>,
}

fn load_music(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(MusicLoading {
        intro: assets.load(INTRO_PATH),
        loop_track: assets.load(LOOP_PATH),
        intro_failed: false,
        loop_failed: false,
        started: false,
        task: None,
    });
}

fn prepare_music(
    mut commands: Commands,
    server: Res<AssetServer>,
    tracks: Res<Assets<AudioSource>>,
    mut music_assets: ResMut<Assets<Music>>,
    mut loading: ResMut<MusicLoading>,
) {
    if let Some(task) = &mut loading.task {
        if let Some(result) = check_ready(task) {
            loading.task = None;
            match result {
                Ok(music) => {
                    commands.spawn((
                        AudioPlayer(music_assets.add(music)),
                        PlaybackSettings::ONCE.with_volume(Volume::Linear(0.3)),
                    ));
                }
                Err(error) => error!("Failed to prepare music: {error}"),
            }
        }
        return;
    }

    if loading.started {
        return;
    }
    if let Some(LoadState::Failed(error)) = server.get_load_state(loading.intro.id())
        && !loading.intro_failed
    {
        error!("Failed to load {INTRO_PATH}: {error}");
        loading.intro_failed = true;
    }
    if let Some(LoadState::Failed(error)) = server.get_load_state(loading.loop_track.id())
        && !loading.loop_failed
    {
        error!("Failed to load {LOOP_PATH}: {error}");
        loading.loop_failed = true;
    }
    if loading.intro_failed || loading.loop_failed {
        return;
    }

    if let (Some(intro), Some(loop_track)) =
        (tracks.get(&loading.intro), tracks.get(&loading.loop_track))
    {
        let intro = intro.clone();
        let loop_track = loop_track.clone();
        loading.started = true;
        loading.task = Some(AsyncComputeTaskPool::get().spawn(async move {
            let intro = decode(intro, INTRO_PATH).await?;
            let loop_track = decode(loop_track, LOOP_PATH).await?;
            Music::new(intro, loop_track)
        }));
    }
}

struct Track {
    samples: Arc<[Sample]>,
    channels: ChannelCount,
    sample_rate: SampleRate,
}

async fn decode(source: AudioSource, path: &str) -> Result<Track, String> {
    let decoder = rodio::Decoder::builder()
        .with_byte_len(source.bytes.len() as u64)
        .with_data(Cursor::new(source))
        .build()
        .map_err(|error| format!("{path}: {error}"))?;
    let channels = decoder.channels();
    let sample_rate = decoder.sample_rate();
    let mut samples = Vec::new();
    for sample in decoder {
        samples.push(sample);
        // On single-threaded web builds, let the executor run other work during decoding.
        if samples.len() % 16384 == 0 {
            yield_now().await;
        }
    }
    if samples.is_empty() || samples.len() % usize::from(channels.get()) != 0 {
        return Err(format!("{path}: empty or incomplete audio frames"));
    }
    Ok(Track {
        samples: samples.into(),
        channels,
        sample_rate,
    })
}

#[derive(Asset, TypePath)]
struct Music {
    intro: Arc<[Sample]>,
    loop_track: Arc<[Sample]>,
    channels: ChannelCount,
    sample_rate: SampleRate,
}

impl Music {
    fn new(intro: Track, loop_track: Track) -> Result<Self, String> {
        if intro.channels != loop_track.channels || intro.sample_rate != loop_track.sample_rate {
            return Err("intro and loop must have matching channel counts and sample rates".into());
        }
        if intro.samples.is_empty() || loop_track.samples.is_empty() {
            return Err("intro and loop must both contain samples".into());
        }
        Ok(Self {
            intro: intro.samples,
            loop_track: loop_track.samples,
            channels: intro.channels,
            sample_rate: intro.sample_rate,
        })
    }
}

impl Decodable for Music {
    type Decoder = MusicSamples;

    fn decoder(&self) -> Self::Decoder {
        MusicSamples {
            intro: self.intro.clone(),
            loop_track: self.loop_track.clone(),
            channels: self.channels,
            sample_rate: self.sample_rate,
            position: 0,
            playing_intro: true,
        }
    }
}

struct MusicSamples {
    intro: Arc<[Sample]>,
    loop_track: Arc<[Sample]>,
    channels: ChannelCount,
    sample_rate: SampleRate,
    position: usize,
    playing_intro: bool,
}

impl Iterator for MusicSamples {
    type Item = Sample;

    fn next(&mut self) -> Option<Sample> {
        if self.playing_intro && self.position == self.intro.len() {
            self.playing_intro = false;
            self.position = 0;
        }
        let samples = if self.playing_intro {
            &self.intro
        } else {
            &self.loop_track
        };
        let sample = samples[self.position];
        self.position += 1;
        if !self.playing_intro && self.position == samples.len() {
            self.position = 0;
        }
        Some(sample)
    }
}

impl Source for MusicSamples {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> ChannelCount {
        self.channels
    }

    fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(samples: &[f32], channels: u16, rate: u32) -> Track {
        Track {
            samples: samples.into(),
            channels: channels.try_into().unwrap(),
            sample_rate: rate.try_into().unwrap(),
        }
    }

    #[test]
    fn intro_is_followed_by_repeating_loop_without_extra_samples() {
        let music = Music::new(track(&[1., 2.], 1, 44100), track(&[3., 4., 5.], 1, 44100)).unwrap();
        assert_eq!(
            music.decoder().take(11).collect::<Vec<_>>(),
            [1., 2., 3., 4., 5., 3., 4., 5., 3., 4., 5.]
        );
    }

    #[test]
    fn rejects_empty_or_mismatched_tracks() {
        assert!(Music::new(track(&[], 1, 44100), track(&[1.], 1, 44100)).is_err());
        assert!(Music::new(track(&[1.], 1, 44100), track(&[], 1, 44100)).is_err());
        assert!(Music::new(track(&[1.], 1, 44100), track(&[1.], 2, 44100)).is_err());
        assert!(Music::new(track(&[1.], 1, 44100), track(&[1.], 1, 48000)).is_err());
    }

    #[test]
    fn invalid_audio_is_reported() {
        let audio = AudioSource {
            bytes: Arc::from(&b"not an audio file"[..]),
        };
        assert!(bevy::tasks::block_on(decode(audio, "bad.ogg")).is_err());
    }

    #[test]
    fn supplied_tracks_decode_and_sequence_across_both_boundaries() {
        let intro = AudioSource {
            bytes: Arc::from(&include_bytes!("../assets/sfx/TlalocRock_intro.ogg")[..]),
        };
        let loop_track = AudioSource {
            bytes: Arc::from(&include_bytes!("../assets/sfx/TlalocRock_loop.ogg")[..]),
        };
        let intro = bevy::tasks::block_on(decode(intro, INTRO_PATH)).unwrap();
        let loop_track = bevy::tasks::block_on(decode(loop_track, LOOP_PATH)).unwrap();
        let intro_len = intro.samples.len();
        let loop_len = loop_track.samples.len();
        let expected = [
            intro.samples[intro_len - 1],
            loop_track.samples[0],
            loop_track.samples[loop_len - 1],
            loop_track.samples[0],
        ];
        let mut samples = Music::new(intro, loop_track).unwrap().decoder();
        let actual = [
            samples.nth(intro_len - 1).unwrap(),
            samples.next().unwrap(),
            samples.nth(loop_len - 2).unwrap(),
            samples.next().unwrap(),
        ];
        assert_eq!(actual, expected);
    }
}
