use std::collections::{HashSet, HashMap};

use crate::{
    engine::events::internal::{EventSender, Notification},
    twitch::command::GameMode,
    twitch::command::Setting,
};

use super::Username;

pub struct VoteTracker {
    pub bullet: HashSet<Username>,
    pub rapid: HashSet<Username>,
    pub classical: HashSet<Username>,
    pub event_sender: EventSender,
}

#[derive(Default, Clone, Eq, PartialEq)]
pub struct Settings {
    pub game_modes: GameModes,
    pub bullet: usize,
    pub rapid: usize,
    pub classical: usize,
    pub total: usize,
}

#[derive(Clone, Eq, PartialEq)]
pub struct GameModes {
    pub bullet: bool,
    pub rapid: bool,
    pub classical: bool,
}

impl VoteTracker {
    pub fn new(event_sender: EventSender) -> Self {
        Self {
            bullet: Default::default(),
            rapid: Default::default(),
            classical: Default::default(),
            event_sender,
        }
    }

    pub fn add_vote(&mut self, user: Username, setting: Setting, on: bool) {
        match setting {
            Setting::GameMode(game_mode) => self.add_game_mode_vote(user, game_mode, on),
        }

        self.event_sender.send_notification(Notification::SettingsChanged);
    }

    pub fn remove_user(&mut self, user: &Username) {
        self.bullet.remove(user);
        self.rapid.remove(user);
        self.classical.remove(user);
    }

    pub fn settings(&self) -> Settings {
        fn is_enabled(count: usize, total: usize) -> bool {
            if total == 0 {
                return false;
            }

            let ratio = count as f64 / total as f64;
            ratio >= 0.5
        }

        let bullet = self.bullet.len();
        let rapid = self.rapid.len();
        let classical = self.classical.len();

        let mut all = HashSet::<String>::default();

        for user in &self.bullet {
            all.insert(user.to_string());
        }
        for user in &self.rapid {
            all.insert(user.to_string());
        }
        for user in &self.classical {
            all.insert(user.to_string());
        }

        let total = all.len();

        let game_modes = GameModes {
            bullet: is_enabled(bullet, total),
            rapid: is_enabled(rapid, total),
            classical: is_enabled(classical, total),
        };

        Settings { game_modes, bullet, rapid, classical, total }
    }

    fn add_game_mode_vote(&mut self, user: Username, game_mode: GameMode, on: bool) {
        let set = match game_mode {
            GameMode::Bullet => &mut self.bullet,
            GameMode::Rapid => &mut self.rapid,
            GameMode::Classical => &mut self.classical,
        };

        if on {
            set.insert(user.to_string());
        } else {
            set.remove(&user);
        }
    }
}

impl Settings {
    pub fn lines(&self) -> Vec<String> {
        fn description(on: bool, count: usize, total: usize) -> String {
            if total == 0 || count == 0 {
                "off: no votes".to_string()
            } else {
                let state = if on {
                    "on"
                } else {
                    "off"
                };

                let percentage = 100.0 * count as f32 / total as f32;
                let percentage = format!("{:.width$}", percentage, width = 3);

                format!("{} ({}%)", state, percentage)
            }
        }

        let bullet = description(self.game_modes.bullet, self.bullet, self.total);
        let rapid = description(self.game_modes.rapid, self.rapid, self.total);
        let classical = description(self.game_modes.classical, self.classical, self.total);

        vec![
            format!("Bullet: {}", bullet),
            "Blitz: always on".to_owned(),
            format!("Rapid: {}", rapid),
            format!("Classical: {}", classical),
        ]
    }
}

impl Default for GameModes {
    fn default() -> Self {
        Self { bullet: true, rapid: true, classical: true }
    }
}
