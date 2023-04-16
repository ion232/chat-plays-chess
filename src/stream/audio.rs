use std::collections::HashMap;
use std::fs::File;

use rodio::Source;

#[derive(Default)]
pub struct AudioManager {}

pub enum Clip {
    Capture,
    Draw,
    Lobby,
    Loss,
    Move,
    Resign,
    Start,
    Win,
}

impl AudioManager {
    pub fn setup(&mut self) {}

    pub fn play_clip(&self, clip: Clip) {
        match clip {
            _ => {
                log::info!("[test] Playing {} audio clip.", clip.to_string());
            }
        }
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
            Clip::Resign => "resign",
            Clip::Start => "start",
            Clip::Win => "win",
        }
        .to_string()
    }
}
