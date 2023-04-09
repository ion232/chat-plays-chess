use std::str::FromStr;
use std::time::Instant;

use chess::BoardStatus;
use lichess_api::model::board::stream::events::GameEventInfo;
use lichess_api::model::Color;
use lichess_api::model::Speed;

use lichess_api::model::board::stream::game::GameFull;
use lichess_api::model::board::stream::game::GameState;

use crate::stream::model::ClockSettings;
use crate::stream::model::Player;
use crate::stream::model::State;
use crate::stream::model::Timer;

pub type GameId = String;

pub enum GameMode {
    Bullet,
    Blitz,
    Rapid,
    Classical,
}

impl ToString for GameMode {
    fn to_string(&self) -> String {
        match self {
            Self::Bullet => "bullet",
            Self::Blitz => "blitz",
            Self::Rapid => "rapid",
            Self::Classical => "classical",
        }
        .to_string()
    }
}

pub struct Game {
    pub game_id: GameId,
    pub speed: Speed,
    pub timestamp: Instant,
    pub clock_settings: Option<ClockSettings>,
    pub board: chess::Board,
    pub move_history: Vec<String>,
    pub last_move: Option<chess::ChessMove>,
    pub is_our_turn: bool,
    pub us: Player,
    pub opponent: Player,
    pub finished: bool,
}

impl Game {
    pub fn from_game_start(game: &GameEventInfo) -> Self {
        let clock_settings =
            game.seconds_left.map(|seconds| ClockSettings { limit: (seconds / 60) as u32, increment: 0 });

        let timer = Timer::new(game.seconds_left.unwrap_or_default() * 1000);
        let us = Player {
            name: "Twitch".to_string(),
            color: color_from_api_color(&game.color).unwrap(),
            rating: None,
            timer,
        };
        let opponent_name = game.opponent.id.as_ref().unwrap_or(&game.opponent.username);
        let opponent = Player {
            name: opponent_name.to_string(),
            color: !us.color,
            rating: game.opponent.rating,
            timer: us.timer.clone(),
        };

        let last_move =
            if !game.last_move.is_empty() { chess::ChessMove::from_str(&game.last_move).ok() } else { None };

        Self {
            game_id: game.game_id.clone(),
            speed: game.speed.clone(),
            timestamp: Instant::now(),
            clock_settings,
            board: chess::Board::from_str(&game.fen).unwrap(),
            move_history: Default::default(),
            last_move,
            is_our_turn: game.is_my_turn,
            us,
            opponent,
            finished: false,
        }
    }

    pub fn from_game_full(bot_id: &str, game: &GameFull) -> Self {
        let board = if let Some(fen) = &game.initial_fen {
            if fen == "startpos" {
                chess::Board::default()
            } else {
                chess::Board::from_str(fen).unwrap()
            }
        } else {
            chess::Board::default()
        };

        let our_name = "Twitch".to_string();
        let our_color = color_from_game(game, &bot_id).unwrap();

        let is_our_turn = our_color == board.side_to_move();
        let move_history = Default::default();

        let timer_millis = game.clock.clone().map(|c| c.initial).unwrap_or_default() as u64;
        let timer = Timer::new(timer_millis);

        let (mut us, mut opponent) = if our_color == chess::Color::Black {
            let us = Player { name: our_name, color: our_color, rating: game.black.rating, timer: timer.clone() };
            let opponent =
                Player { name: game.white.name.to_string(), color: !us.color, rating: game.white.rating, timer };
            (us, opponent)
        } else {
            let us = Player { name: our_name, color: our_color, rating: game.white.rating, timer: timer.clone() };
            let opponent =
                Player { name: game.black.name.to_string(), color: !us.color, rating: game.black.rating, timer };
            (us, opponent)
        };
        let mut last_move = None;

        if let Some(game_state) = &game.state {
            if our_color == chess::Color::Black {
                us.timer = Timer::new(game_state.btime);
                opponent.timer = Timer::new(game_state.wtime);
            } else {
                us.timer = Timer::new(game_state.wtime);
                opponent.timer = Timer::new(game_state.btime);
            }
            last_move = game_state.moves.split(" ").last().and_then(|m| chess::ChessMove::from_str(m).ok());
        }

        let clock_settings =
            game.clock.clone().map(|c| ClockSettings { limit: c.initial / 60000, increment: c.increment / 1000 });

        Self {
            game_id: game.id.to_string(),
            speed: game.speed.clone(),
            timestamp: Instant::now(),
            clock_settings,
            board,
            move_history,
            last_move,
            is_our_turn,
            us,
            opponent,
            finished: false,
        }
    }

    pub fn update_model(
        &self,
        board: &mut chess::Board,
        move_history: &mut Vec<String>,
        opponent: &mut Player,
        us: &mut Player,
        state: &mut State,
    ) {
        *board = self.board.clone();
        *move_history = self.move_history.clone();
        *opponent = self.opponent.clone();
        *us = self.us.clone();
        *state = if self.finished {
            State::GameFinished
        } else if self.is_our_turn {
            State::OurTurn
        } else {
            State::OpponentsTurn
        };
    }

    pub fn elapse_time(&mut self, milliseconds: u64) {
        if self.is_our_turn {
            self.us.timer.elapse(milliseconds);
        } else {
            self.opponent.timer.elapse(milliseconds);
        }
    }

    pub fn process_game_info(&mut self, game: &GameEventInfo) {
        if self.clock_settings.is_none() {
            self.clock_settings =
                game.seconds_left.map(|seconds| ClockSettings { limit: (seconds / 60) as u32, increment: 0 });
        }

        self.opponent.name = game.opponent.id.as_ref().unwrap_or(&game.opponent.username).to_string();
        self.opponent.rating = game.opponent.rating;
        self.is_our_turn = game.is_my_turn;
    }

    pub fn process_game_state(&mut self, game: GameState) {
        let moves: Vec<&str> = game.moves.split(" ").collect();
        self.move_history = moves.iter().map(|m| m.to_string()).collect();
        self.last_move = moves.last().map(|m| chess::ChessMove::from_str(m).ok()).flatten();

        let Some(board) = board_from_moves(moves) else {
            log::warn!("Board status for game {} is no longer ongoing", self.game_id);
            return;
        };

        self.board = board;
        self.is_our_turn = self.us.color == board.side_to_move();

        if self.us.color == chess::Color::Black {
            self.us.timer = Timer::new(game.btime);
            self.opponent.timer = Timer::new(game.wtime);
        } else {
            self.us.timer = Timer::new(game.wtime);
            self.opponent.timer = Timer::new(game.btime);
        }

        if let Some(clock_settings) = &mut self.clock_settings {
            clock_settings.increment = (game.binc / 10000) as u32;
        }

        // TODO: Refactor this as an enum in the lichess api crate.
        if game.status != "started" || game.winner.is_some() {
            log::info!("Game {} finished", self.game_id);
            self.finished = true;
        }
    }
}

fn board_from_moves(moves: Vec<&str>) -> Option<chess::Board> {
    let mut board = chess::Board::default();
    let mut result = chess::Board::default();

    for game_move in moves {
        if game_move.is_empty() {
            continue;
        }
        if let BoardStatus::Ongoing = board.status() {
            if let Some(chess_move) = chess::ChessMove::from_str(game_move).ok() {
                board.make_move(chess_move, &mut result);
                board = result;
            }
        } else {
            return None;
        }
    }

    board.into()
}

fn color_from_api_color(color: &Color) -> Option<chess::Color> {
    if let Color::Black = color {
        chess::Color::Black.into()
    } else if let Color::White = color {
        chess::Color::White.into()
    } else {
        None
    }
}

fn color_from_game(game: &GameFull, bot_id: &str) -> Option<chess::Color> {
    if game.white.id == bot_id {
        chess::Color::White.into()
    } else if game.black.id == bot_id {
        chess::Color::Black.into()
    } else {
        None
    }
}
