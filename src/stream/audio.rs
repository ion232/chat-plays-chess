use std::fs::File;
use std::io::BufReader;

use rodio::{Decoder, OutputStream, Sink};

use crate::error::Result;

#[derive(Default)]
pub struct AudioManager {
    playback: Option<Playback>,
}

pub struct Playback {
    pub sink: Sink,
    output_stream: OutputStream,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Clip {
    Capture,
    Draw,
    Lobby,
    Loss,
    Move,
    Start,
    Win,
}

impl AudioManager {
    pub fn setup(&mut self) -> Result<()> {
        let (output_stream, handle) = OutputStream::try_default()?;
        let sink = rodio::Sink::try_new(&handle)?;

        self.playback = Playback::new(sink, output_stream).into();

        Ok(())
    }

    // Not ideal to read from the file every time, but should be fine.
    pub fn play_clip(&mut self, clip: Clip) {
        let file_path = clip.file_path();

        let Ok(file) = File::open(file_path.clone()) else {
            log::error!("Failed to open clip file at {}", file_path);
            return;
        };

        let Ok(decoder) = Decoder::new(BufReader::new(file)) else {
            log::error!("Failed to open decode clip {}", clip.to_string());
            return;
        };

        let Some(playback) = &mut self.playback else {
            return;
        };

        let volume = match clip {
            Clip::Capture => 0.8,
            Clip::Draw => 1.0,
            Clip::Lobby => 0.8,
            Clip::Loss => 0.5,
            Clip::Move => 0.6,
            Clip::Start => 0.7,
            Clip::Win => 1.0,
        };

        playback.sink.set_volume(volume);
        playback.sink.append(decoder);
    }
}

impl Playback {
    pub fn new(sink: Sink, output_stream: OutputStream) -> Self {
        Self { sink, output_stream }
    }
}

impl Clip {
    fn file_path(&self) -> String {
        let name = self.to_string();
        format!("assets/audio/{}.mp3", name)
    }
}

impl ToString for Clip {
    fn to_string(&self) -> String {
        match self {
            Clip::Capture => "capture",
            Clip::Draw => "draw",
            Clip::Lobby => "lobby",
            Clip::Loss => "loss",
            Clip::Move => "move",
            Clip::Start => "start",
            Clip::Win => "win",
        }
        .to_string()
    }
}
