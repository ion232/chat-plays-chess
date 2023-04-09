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
            Clip::Capture => todo!(),
            Clip::Draw => todo!(),
            Clip::Lobby => todo!(),
            Clip::Loss => todo!(),
            Clip::Move => todo!(),
            Clip::Resign => todo!(),
            Clip::Start => todo!(),
            Clip::Win => todo!(),
        }
    }
}
