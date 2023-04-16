use std::collections::HashSet;

use crate::{twitch::command::GameMode, twitch::command::Setting};

use super::Username;

#[derive(Default)]
pub struct VoteTracker {
    pub bullet: HashSet<Username>,
    pub rapid: HashSet<Username>,
    pub classical: HashSet<Username>,
}

#[derive(Default, Clone)]
pub struct Settings {
    pub game_modes: GameModes,
    pub bullet: usize,
    pub rapid: usize,
    pub classical: usize,
    pub total: usize,
}

#[derive(Clone)]
pub struct GameModes {
    pub bullet: bool,
    pub rapid: bool,
    pub classical: bool,
}

impl VoteTracker {
    pub fn add_vote(&mut self, user: Username, setting: Setting, on: bool) {
        match setting {
            Setting::GameMode(game_mode) => self.add_game_mode_vote(user, game_mode, on),
        }
    }

    pub fn remove_user(&mut self, user: &Username) {
        self.bullet.remove(user);
        self.rapid.remove(user);
        self.classical.remove(user);
    }

    pub fn settings(&self) -> Settings {
        fn is_majority(count: usize, total: usize) -> bool {
            let ratio = count as f64 / total as f64;
            ratio >= 0.5
        }

        let bullet = self.bullet.len();
        let rapid = self.rapid.len();
        let classical = self.classical.len();

        let total = bullet + rapid + classical;

        let game_modes = GameModes {
            bullet: is_majority(bullet, total),
            rapid: is_majority(rapid, total),
            classical: is_majority(classical, total),
        };

        Settings { game_modes, bullet, rapid, classical, total }
    }

    fn add_game_mode_vote(&mut self, user: Username, game_mode: GameMode, on: bool) {
        let set = match game_mode {
            // Cleanest way I can find to just ignore the vote.
            GameMode::Blitz => &mut self.bullet,
            GameMode::Rapid => &mut self.rapid,
            GameMode::Classical => &mut self.classical,
        };

        if on {
            set.insert(user);
        } else {
            set.remove(&user);
        }
    }
}

impl Settings {
    pub fn lines(&self) -> Vec<String> {
        fn to_str(on: bool) -> &'static str {
            if on {
                "on"
            } else {
                "off"
            }
        }
        let total = std::cmp::max(self.total, 1);
        vec![
            format!("Bullet: {} ({}%)", to_str(self.game_modes.bullet), self.bullet / total),
            "Blitz: always on".to_owned(),
            format!("Rapid: {} ({}%)", to_str(self.game_modes.rapid), self.rapid / total),
            format!(
                "Classical: {} ({}%)",
                to_str(self.game_modes.classical),
                self.classical / total
            ),
        ]
    }
}

impl Default for GameModes {
    fn default() -> Self {
        Self { bullet: true, rapid: true, classical: true }
    }
}
