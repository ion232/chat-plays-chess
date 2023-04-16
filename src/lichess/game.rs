use std::collections::HashMap;
use std::str::FromStr;
use std::time::Instant;

use chess::BoardStatus;
use chess::ChessMove;

use lichess_api::model::board::stream::events::GameEventInfo;
use lichess_api::model::Color;
use lichess_api::model::Speed;

use lichess_api::model::board::stream::game::GameFull;
use lichess_api::model::board::stream::game::GameState;
use lichess_api::model::board::stream::game::OpponentGone;

use crate::engine::events::internal::EventSender;
use crate::engine::events::internal::GameNotification;
use crate::engine::events::internal::Notification;
use crate::stream::model::ClockSettings;
use crate::stream::model::Player;
use crate::stream::model::Timer;

pub type GameId = String;

pub struct GameManager {
    our_id: String,
    games: HashMap<GameId, Game>,
    last_finished_game: Option<Game>,
    current_game_id: Option<GameId>,
    event_sender: EventSender,
}

#[derive(Clone)]
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

impl GameManager {
    pub fn new(our_id: String, event_sender: EventSender) -> Self {
        Self {
            our_id,
            games: Default::default(),
            last_finished_game: Default::default(),
            current_game_id: Default::default(),
            event_sender,
        }
    }

    pub fn game(&self, game_id: &str) -> Option<&Game> {
        self.games.get(game_id)
    }

    pub fn convert_move(&self, chess_move: String) -> Option<chess::ChessMove> {
        let Some(game) = self.current_game() else {
            return None;
        };

        let Some(uci_move) = ChessMove::from_str(&chess_move).ok() else {
            return None;
        };

        if game.board.legal(uci_move) {
            uci_move.into()
        } else {
            None
        }
    }

    pub fn last_game(&self) -> Option<&Game> {
        self.last_finished_game.as_ref()
    }

    pub fn current_game(&self) -> Option<&Game> {
        let Some(game_id) = &self.current_game_id else {
            return None;
        };

        if let Some(game) = self.games.get(game_id) {
            game.into()
        } else {
            None
        }
    }

    pub fn oldest_game_id(&self) -> Option<String> {
        if self.games.is_empty() {
            return None;
        }

        self.games
            .iter()
            .min_by(|l, r| l.1.timestamp.cmp(&r.1.timestamp))
            .map(|(id, _)| id.to_string())
    }

    pub fn advance_game_time(&mut self) {
        // self.current_game().unwrap().elapse_time(milliseconds)
    }

    pub fn switch_game(&mut self, game_id: &str) {
        let game_id = game_id.to_string();
        if self.games.contains_key(&game_id) {
            self.current_game_id = game_id.to_string().into();
            self.event_sender
                .send_notification(Notification::Game(GameNotification::NewCurrentGame));
        } else {
            log::warn!("[GameManager] Failed to switch to game {}", &game_id);
        }
    }

    pub fn process_game_start(&mut self, game_info: &GameEventInfo) {
        let game_id = game_info.game_id.clone();
        let game = Game::from_game_start(game_info);

        let None = self.games.insert(game_id.clone(), game) else {
            log::warn!("[GameManager] Evicted game {} during process game start", &game_id);
            return;
        };

        self.event_sender
            .send_notification(Notification::Game(GameNotification::GameStarted { game_id }));
    }

    pub fn process_game_finish(&mut self, game_info: &GameEventInfo) {
        let game_id = game_info.game_id.clone();
        if let Some(game) = self.games.get_mut(&game_id) {
            game.process_game_info(game_info);
        } else {
            log::warn!("[GameManager] Failed to find game {} during process game finish", &game_id);
        }

        self.event_sender.send_notification(Notification::Game(GameNotification::GameFinished));

        let Some(finished_game) = self.games.remove(&game_id) else {
            return;
        };

        let Some(current_game_id) = &self.current_game_id else {
            return;
        };

        if finished_game.game_id == *current_game_id {
            self.current_game_id = None;
            self.last_finished_game = finished_game.into();
        }
    }

    pub fn process_game_full(&mut self, game_full: &GameFull) {
        let game_id = &game_full.id;
        let Some(game) = self.games.get_mut(game_id) else {
            log::warn!("[GameManager] Failed to find game {} during process game full", game_id);
            return;
        };

        // I'm assuming here that GameFull has all necessary data - therefore we can override.
        *game = Game::from_game_full(&self.our_id, game_full);

        if game.finished {
            return;
        }

        let game_id = game.game_id.to_string();
        let notification = if game.is_our_turn {
            GameNotification::OurTurn { game_id }
        } else {
            GameNotification::TheirTurn { game_id }
        };
        let notification = Notification::Game(notification);

        self.event_sender.send_notification(notification);
    }

    pub fn process_game_update(&mut self, game_id: &str, game_state: &GameState) {
        let Some(game) = self.games.get_mut(game_id) else {
            log::warn!("[GameManager] Failed to find game {} during process game update", &game_id);
            return;
        };

        let previous_board = game.board.clone();
        game.process_game_state(&game_state);

        if game.finished {
            return;
        }

        let game_id = game.game_id.to_string();
        let notification = if game.is_our_turn {
            GameNotification::OurTurn { game_id }
        } else {
            GameNotification::TheirTurn { game_id }
        };
        let notification = Notification::Game(notification);

        // To avoid requiring a mut ref.
        self.event_sender.clone().send_notification(notification);

        if game.board != previous_board {
            let notification = GameNotification::PlayerMoved {
                game_id: game.game_id.to_string(),
                was_us: !game.is_our_turn,
            };
            self.event_sender.send_notification(Notification::Game(notification))
        }
    }

    pub fn process_opponent_gone(&mut self, opponent_gone: &OpponentGone) {
        // Might use this to switch games so the timer doesn't have to run out.
        // Will see - it introduces a lot of complexity.
        _ = opponent_gone;
    }
}

impl Game {
    pub fn from_game_start(game: &GameEventInfo) -> Self {
        let clock_settings = game
            .seconds_left
            .map(|seconds| ClockSettings { limit: (seconds / 60) as u32, increment: 0 });

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

        let last_move = if !game.last_move.is_empty() {
            chess::ChessMove::from_str(&game.last_move).ok()
        } else {
            None
        };

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

    pub fn from_game_full(our_id: &str, game: &GameFull) -> Self {
        let board = board_from_api_fen(game.initial_fen.clone());

        let our_name = "Twitch".to_string();
        let our_color = color_from_game(game, &our_id).unwrap();

        let is_our_turn = our_color == board.side_to_move();
        let move_history = Default::default();

        let timer_millis = game.clock.clone().map(|c| c.initial).unwrap_or_default() as u64;
        let timer = Timer::new(timer_millis);

        let (mut us, mut opponent) = if our_color == chess::Color::Black {
            let us = Player {
                name: our_name,
                color: our_color,
                rating: game.black.rating,
                timer: timer.clone(),
            };
            let opponent = Player {
                name: game.white.name.to_string(),
                color: !us.color,
                rating: game.white.rating,
                timer,
            };
            (us, opponent)
        } else {
            let us = Player {
                name: our_name,
                color: our_color,
                rating: game.white.rating,
                timer: timer.clone(),
            };
            let opponent = Player {
                name: game.black.name.to_string(),
                color: !us.color,
                rating: game.black.rating,
                timer,
            };
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
            last_move =
                game_state.moves.split(" ").last().and_then(|m| chess::ChessMove::from_str(m).ok());
        }

        let clock_settings = game
            .clock
            .clone()
            .map(|c| ClockSettings { limit: c.initial / 60000, increment: c.increment / 1000 });

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

    pub fn elapse_time(&mut self, milliseconds: u64) {
        if self.finished {
            return;
        }

        if self.is_our_turn {
            self.us.timer.elapse(milliseconds);
        } else {
            self.opponent.timer.elapse(milliseconds);
        }
    }

    pub fn process_game_info(&mut self, game: &GameEventInfo) {
        if self.clock_settings.is_none() {
            self.clock_settings = game
                .seconds_left
                .map(|seconds| ClockSettings { limit: (seconds / 60) as u32, increment: 0 });
        }

        self.opponent.name =
            game.opponent.id.as_ref().unwrap_or(&game.opponent.username).to_string();
        self.opponent.rating = game.opponent.rating;
        self.is_our_turn = game.is_my_turn;
    }

    pub fn process_game_state(&mut self, game: &GameState) {
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

fn board_from_api_fen(fen: Option<String>) -> chess::Board {
    if let Some(fen) = fen {
        if fen == "startpos" {
            chess::Board::default()
        } else {
            chess::Board::from_str(&fen).unwrap_or_default()
        }
    } else {
        chess::Board::default()
    }
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

fn color_from_game(game: &GameFull, our_id: &str) -> Option<chess::Color> {
    if game.white.id == our_id {
        chess::Color::White.into()
    } else if game.black.id == our_id {
        chess::Color::Black.into()
    } else {
        None
    }
}
