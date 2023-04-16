use std::collections::HashMap;

use lichess_api::model::Speed;

use crate::{
    engine::votes::settings::{GameModes, Settings},
    lichess::game::Game,
};

pub struct Model {
    pub title: Title,
    pub notice: Notice,
    pub chat_commands: Vec<Command>,
    pub move_history: Vec<String>,
    pub us: Player,
    pub opponent: Player,
    pub board: chess::Board,
    pub settings: Settings,
    pub game_votes: GameVotes,
    pub state: State,
}

pub struct Title {
    pub url: &'static str,
    pub speed: Option<Speed>,
    pub clock_settings: Option<ClockSettings>,
}

#[derive(Clone)]
pub struct Notice {
    pub lines: Vec<String>,
}

#[derive(Clone)]
pub struct ClockSettings {
    pub limit: u32,
    pub increment: u32,
}

#[derive(Debug)]
pub struct Command {
    pub username: String,
    pub command: String,
}

#[derive(Clone)]
pub struct Player {
    pub name: String,
    pub color: chess::Color,
    pub rating: Option<u32>,
    pub timer: Timer,
}

#[derive(Copy, Clone)]
pub struct Timer {
    pub minutes: u64,
    pub seconds: u64,
}

#[derive(Clone, Default)]
pub struct GameVotes {
    pub votes: HashMap<String, VoteStats>,
    pub delays: Delays,
}

#[derive(Clone, Copy)]
pub struct VoteStats {
    pub vote_changes: i32,
    pub total_votes: u32,
}

#[derive(Clone, Default)]
pub struct Delays {
    pub current: u8,
    pub max: u8,
}

pub enum State {
    FindingNewOpponent,
    OurTurn,
    OpponentsTurn,
    GameFinished,
    WaitingForChallengeReply { remaining: u64 },
    Unknown,
}

#[derive(Clone)]
pub enum Side {
    Ours,
    Theirs,
}

impl Model {
    pub fn update_from_game(&mut self, game: Game) {
        self.title.speed = game.speed.into();
        self.title.clock_settings = game.clock_settings;

        self.board = game.board;
        self.move_history = game.move_history.clone();
        self.opponent = game.opponent.clone();
        self.us = game.us.clone();
    }
}

impl Default for Model {
    fn default() -> Self {
        let title = Title::new();
        let notice = Default::default();
        let chat_commands = Default::default();
        let move_history = Default::default();
        let user = Player {
            name: "Twitch".to_string(),
            color: chess::Color::White,
            rating: None,
            timer: Timer { minutes: 0, seconds: 0 },
        };
        let opponent = Player {
            name: "Unknown".to_string(),
            color: chess::Color::Black,
            rating: None,
            timer: Timer { minutes: 0, seconds: 0 },
        };
        let board = chess::Board::default();
        let settings = Settings {
            game_modes: GameModes::default(),
            bullet: 0,
            rapid: 0,
            classical: 0,
            total: 0,
        };
        let game_votes =
            GameVotes { votes: Default::default(), delays: Delays { current: 0, max: 6 } };
        let state = State::Unknown;

        Self {
            title,
            notice,
            chat_commands,
            move_history,
            us: user,
            opponent,
            board,
            settings,
            game_votes,
            state,
        }
    }
}

impl Title {
    pub fn new() -> Self {
        Self { url: "lichess.org/@/TTVPlaysChess", speed: None, clock_settings: None }
    }
}

impl Default for Notice {
    fn default() -> Self {
        let lines = vec![
            "Welcome to TTVPlaysChess!".to_string(),
            "".to_string(),
            "Read the channel description".to_string(),
            "for details about this".to_string(),
            "stream and how to participate.".to_string(),
        ];
        Self { lines }
    }
}

impl Command {
    pub fn new(username: String, command: String) -> Self {
        Command { username, command }
    }
}

impl Timer {
    pub fn new(milliseconds: u64) -> Self {
        Self { minutes: milliseconds / (60 * 1000), seconds: (milliseconds % (60 * 1000)) / 1000 }
    }

    pub fn elapse(&mut self, milliseconds: u64) {
        let total = self.as_millis();
        let timer =
            if milliseconds < total { Self::new(total - milliseconds) } else { Self::new(0) };

        *self = timer;
    }

    fn as_millis(&self) -> u64 {
        (self.minutes * 60 * 1000) + (self.seconds * 1000)
    }
}

impl GameVotes {
    pub fn lines(&self) -> Vec<String> {
        // Not the most efficient, but the max legal chess moves appears to be 218.
        let mut lines: Vec<(String, VoteStats)> = self.votes.clone().into_iter().collect();
        lines.sort_by(|l, r| r.1.total_votes.cmp(&l.1.total_votes));
        let mut lines: Vec<String> = lines
            .into_iter()
            .map(|(chess_move, vote_stats)| format!("{}: {}", chess_move, vote_stats.to_string()))
            .collect();
        lines.insert(0, self.delays.to_string());
        lines.insert(1, "".to_string());

        lines
    }
}

impl VoteStats {
    pub fn update_changes(old: &VoteStats, new: &mut VoteStats) {
        new.vote_changes = new.total_votes as i32 - old.total_votes as i32;
    }
}

impl Delays {
    pub fn new(max: u8) -> Self {
        Self { current: 0, max }
    }

    pub fn add_delay(&mut self) {
        self.current += 1;
        self.current = std::cmp::min(self.current, self.max);
    }

    pub fn can_delay(&self) -> bool {
        self.current >= self.max
    }
}

impl ToString for Title {
    fn to_string(&self) -> String {
        let Some(speed) = &self.speed else {
            return "???".to_string();
        };

        // TODO: Move this into lichess api.
        let speed = match speed {
            Speed::UltraBullet => "Ultrabullet",
            Speed::Bullet => "Bullet",
            Speed::Blitz => "Blitz",
            Speed::Rapid => "Rapid",
            Speed::Classical => "Classical",
            Speed::Correspondence => "Correspondence",
        };

        if let Some(clock) = &self.clock_settings {
            format!("{} ({} + {})", speed, clock.limit, clock.increment)
        } else {
            speed.to_string()
        }
    }
}

impl ToString for Command {
    fn to_string(&self) -> String {
        format!("{}: {}", self.username, self.command)
    }
}

impl ToString for Player {
    fn to_string(&self) -> String {
        let rating = self.rating.map(|r| r.to_string()).unwrap_or("????".to_string());
        format!("{} {} {}", self.name, rating, self.timer.to_string())
    }
}

impl ToString for Timer {
    fn to_string(&self) -> String {
        let extra_minutes = self.seconds / 60;
        let minutes = self.minutes + extra_minutes;
        let seconds = self.seconds % 60;
        let seconds = if seconds <= 9 { format!("0{}", seconds) } else { seconds.to_string() };

        format!("{}:{}", minutes, seconds)
    }
}

impl ToString for VoteStats {
    fn to_string(&self) -> String {
        let op = if self.vote_changes.is_positive() { "+" } else { "" };
        format!("{} ({}{})", self.total_votes, op, self.vote_changes)
    }
}

impl ToString for Delays {
    fn to_string(&self) -> String {
        let full_count = self.current as usize;
        let empty_count = (self.max - self.current) as usize;

        let full = "[x]".repeat(full_count);
        let empty = "[ ]".repeat(empty_count);

        format!("Delays:{}{}", full, empty)
    }
}

impl ToString for State {
    fn to_string(&self) -> String {
        match self {
            State::FindingNewOpponent => "Finding new opponent...".to_string(),
            State::OurTurn => "In game: Our turn".to_string(),
            State::OpponentsTurn => "In game: Opponents turn".to_string(),
            State::GameFinished => "Game finished".to_string(),
            State::WaitingForChallengeReply { remaining } => {
                format!("Sent challenge - waiting {}s for response...", remaining).to_string()
            }
            State::Unknown => "Unknown".to_string(),
        }
    }
}
