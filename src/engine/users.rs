use std::collections::{HashMap, HashSet};

use lichess_api::model::Speed;

use crate::engine::GameModes;
use crate::engine::Settings;

use crate::lichess::game::Game;
use crate::stream::model::Model;
use crate::twitch::command::GameMode;
use crate::twitch::command::Setting;

use super::moves::GameVotes;
use super::moves::Vote;

pub type Username = String;

pub struct UserData;

pub struct ActiveUsers {
    game_votes: GameVotes,
    settings_votes: SettingsVotes,
    users: HashMap<Username, UserData>,
}

impl ActiveUsers {
    pub fn new() -> Self {
        Self {
            game_votes: GameVotes::new(&Speed::Bullet),
            settings_votes: Default::default(),
            users: Default::default(),
        }
    }

    pub fn handle_new_game(&mut self, game: &Game) {
        self.game_votes = GameVotes::new(&game.speed);
    }

    pub fn reset_vote_timer(&mut self) {
        self.game_votes.reset_vote_timer()
    }

    pub fn update_model(&self, model: &mut Model) {
        model.game_votes = self.game_votes.get_model_game_votes();
        model.settings = self.get_settings();
    }

    pub fn add_delay(&mut self) {
        self.game_votes.add_delay();
    }

    pub fn can_delay(&self) -> bool {
        self.game_votes.get_delays().can_delay()
    }

    pub fn votes_ready(&self) -> bool {
        self.game_votes.votes_ready()
    }

    pub fn get_top_vote(&mut self) -> Option<Vote> {
        self.game_votes.get_top_vote()
    }

    pub fn get_settings(&self) -> Settings {
        self.settings_votes.get_settings()
    }

    pub fn add_move_vote(&mut self, user: Username, vote: super::moves::Vote) {
        self.add_user(user.to_string());
        self.game_votes.add_vote(user, vote);
    }

    pub fn add_settings_vote(&mut self, user: Username, setting: Setting, on: bool) {
        self.add_user(user.to_string());

        match setting {
            Setting::GameMode(game_mode) => self.add_game_mode_vote(user, game_mode, on),
        }
    }

    pub fn add_game_mode_vote(&mut self, user: Username, game_mode: GameMode, on: bool) {
        self.settings_votes.add_game_mode_vote(user, game_mode, on)
    }

    pub fn add_user(&mut self, user: Username) {
        if !self.users.contains_key(&user) {
            self.users.insert(user, UserData {});
        }
    }

    pub fn remove_user(&mut self, user: &Username) {
        self.users.remove(user);
        self.settings_votes.remove_user(&user);
    }

    pub fn reset_game_votes(&mut self) {
        self.game_votes.reset();
    }
}

#[derive(Default)]
pub struct SettingsVotes {
    pub blitz: HashSet<Username>,
    pub rapid: HashSet<Username>,
    pub classical: HashSet<Username>,
}

#[derive(Default)]
pub struct SettingsMetrics {
    pub blitz: usize,
    pub rapid: usize,
    pub classical: usize,
    pub total: usize,
}

impl SettingsVotes {
    pub fn get_settings(&self) -> Settings {
        fn is_majority(count: usize, total: usize) -> bool {
            let ratio = count as f64 / total as f64;
            ratio >= 0.5
        }

        let blitz = self.blitz.len();
        let rapid = self.rapid.len();
        let classical = self.classical.len();

        let total = blitz + rapid + classical;

        let game_modes = GameModes {
            blitz: is_majority(blitz, total),
            rapid: is_majority(rapid, total),
            classical: is_majority(classical, total),
        };

        Settings { game_modes, blitz, rapid, classical, total }
    }

    pub fn add_game_mode_vote(&mut self, user: Username, game_mode: GameMode, on: bool) {
        let set = match game_mode {
            GameMode::Blitz => &mut self.blitz,
            GameMode::Rapid => &mut self.rapid,
            GameMode::Classical => &mut self.classical,
            _ => panic!(""),
        };

        if on {
            set.insert(user);
        } else {
            set.remove(&user);
        }
    }

    pub fn remove_user(&mut self, user: &Username) {
        self.blitz.remove(user);
        self.rapid.remove(user);
        self.classical.remove(user);
    }
}
