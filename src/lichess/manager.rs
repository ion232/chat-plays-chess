use std::borrow::Borrow;
use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;
use std::time::Instant;

use chess::ChessMove;

use lichess_api::model::Title;
use lichess_api::model::VariantKey;

use lichess_api::model::board::stream::events;
use lichess_api::model::board::stream::events::GameEventInfo;
use lichess_api::model::board::stream::game;
use lichess_api::model::board::stream::game::ChatLine;
use lichess_api::model::board::stream::game::GameFull;
use lichess_api::model::board::stream::game::GameState;
use lichess_api::model::board::stream::game::OpponentGone;

use lichess_api::model::challenges::decline::Reason;
use lichess_api::model::challenges::ChallengeCreated;
use lichess_api::model::challenges::ChallengeJson;
use lichess_api::model::challenges::Status;

use crate::engine::Settings;

use crate::engine::action::ActionSender;
use crate::lichess::action::{Action, Actor};
use crate::lichess::events::Event;
use crate::lichess::game::Game;

use crate::lichess::Context;
use crate::stream::model::Model;
use crate::stream::model::State;

use super::challenge::ChallengeManager;
use super::game::GameId;

const CHALLENGE_WAIT_TIME: u64 = 20;

pub struct LichessManager {
    pub actor: Actor,
    action_sender: ActionSender,
    settings: Settings,
    other_games: HashMap<GameId, Game>,
    last_game: Option<Game>,
    game: Option<Game>,
    challenge_manager: ChallengeManager,
    last_updated: Instant,
}

impl LichessManager {
    pub fn new(context: Context, action_sender: ActionSender) -> Self {
        Self {
            actor: Actor::new(context),
            action_sender,
            settings: Default::default(),
            other_games: Default::default(),
            last_game: Default::default(),
            game: Default::default(),
            challenge_manager: Default::default(),
            last_updated: Instant::now(),
        }
    }

    pub fn manage_state(&mut self) {
        let elapsed = self.last_updated.elapsed().as_millis();
        self.last_updated = Instant::now();

        let Some(game) = &mut self.game else {
            return;
        };

        game.elapse_time(elapsed as u64);

        if game.is_our_turn {
            let action = Action::make_move(game.game_id.to_string()).into();
            self.action_sender.send(action);
        }
    }

    pub fn update_model(&mut self, model: &mut Model) {
        if let Some(challenge) = self.challenge_manager.get_outbound() {
            let remaining = CHALLENGE_WAIT_TIME - challenge.timestamp.elapsed().as_secs();
            model.state = State::WaitingForChallengeReply { remaining };
        }

        if let Some(game) = &self.game {
            game.update_model(
                &mut model.board,
                &mut model.move_history,
                &mut model.opponent,
                &mut model.us,
                &mut model.state,
            );
        } else if let Some(game) = &self.last_game {
            game.update_model(
                &mut model.board,
                &mut model.move_history,
                &mut model.opponent,
                &mut model.us,
                &mut model.state,
            );
        }

        model.settings = self.settings.clone();
    }

    pub fn get_game(&mut self, game_id: &str) -> Option<&Game> {
        if let Some(game) = &self.game {
            if game.game_id == game_id {
                return game.into();
            }
        }
        None
    }

    pub fn set_game(&mut self, game: Game) {
        if let Some(current_game) = &self.game {
            log::warn!("Setting game {} when game {} already exists", game.game_id, current_game.game_id);
        }
        self.game = game.into();
    }

    pub fn add_outbound(&mut self, challenge: ChallengeCreated) {
        let challenge = challenge.challenge;
        if challenge.decline_reason.is_none() {
            self.challenge_manager.set_outbound(challenge);
        }
    }

    pub fn find_new_opponent(&mut self) {
        if self.game.is_some() {
            return;
        }

        if let Some(challenge) = self.challenge_manager.get_outbound() {
            let id = challenge.challenge.base.id.to_string();
            if challenge.timestamp.elapsed() > Duration::from_secs(CHALLENGE_WAIT_TIME) {
                log::info!("Outbound challenge has expired - canceling: {}", &id);
                self.challenge_manager.clear_outbound();
                self.action_sender.send(Action::cancel_challenge(id.to_string()).into());
            }
        } else if let Some(challenge) = self.challenge_manager.remove_latest_inbound() {
            self.action_sender.send(Action::accept_challenge(challenge.base.id).into());
        } else if !self.other_games.is_empty() {
            let game_id =
                self.other_games.iter().min_by(|l, r| l.1.timestamp.cmp(&r.1.timestamp)).map(|(id, _)| id.to_string());
            let Some(game_id) = game_id else {
                return;
            };
            if let Some(game) = self.other_games.remove(&game_id) {
                self.action_sender.send(crate::engine::action::Action::SwitchGame(game));
            }
        } else {
            self.action_sender.send(Action::challenge_random_bot().into());
        }
    }

    pub fn convert_move(&self, chess_move: String) -> Option<chess::ChessMove> {
        if let Some(game) = &self.game {
            if let Some(uci_move) = ChessMove::from_str(&chess_move).ok() {
                if game.board.legal(uci_move) {
                    return uci_move.into();
                }
            }
        }
        None
    }

    pub fn in_game(&self) -> bool {
        self.game.is_some()
    }

    pub fn process_lichess_event(&mut self, event: Event) {
        match event {
            Event::AccountEvent { event } => self.process_account_event(event),
            Event::GameEvent { game_id, event } => self.process_game_event(game_id, event),
        }
    }

    pub fn process_account_event(&mut self, event: events::Event) {
        match event {
            events::Event::Challenge { challenge } => self.process_challenge(challenge),
            events::Event::ChallengeCanceled { challenge } => self.process_challenge_canceled(challenge),
            events::Event::ChallengeDeclined { challenge } => self.process_challenge_declined(challenge),
            events::Event::GameStart { game } => self.process_game_start(&game),
            events::Event::GameFinish { game } => self.process_game_finish(&game),
        }
    }

    pub fn process_game_event(&mut self, game_id: String, event: game::Event) {
        match event {
            game::Event::GameFull { game_full } => self.process_game_full(game_id, game_full),
            game::Event::GameState { game_state } => self.process_game_state(game_id, game_state),
            game::Event::ChatLine { chat_line } => self.process_chat_line(game_id, chat_line),
            game::Event::OpponentGone { opponent_gone } => self.process_opponent_gone(game_id, opponent_gone),
        }
    }

    fn process_challenge(&mut self, challenge: ChallengeJson) {
        log::info!("Challenge event received: id: {}", challenge.base.id);

        match challenge.base.status {
            Status::Created => self.process_challenge_created(challenge),
            Status::Offline => {
                self.process_challenge_offline(challenge);
            }
            Status::Canceled => {
                self.process_challenge_canceled(challenge);
            }
            Status::Declined => {
                self.process_challenge_declined(challenge);
            }
            Status::Accepted => {
                log::info!("Challenge was accepted by opponent.");
            }
        }
    }

    fn process_challenge_created(&mut self, challenge: ChallengeJson) {
        log::info!("Challenge {} created", &challenge.base.id);

        let challenge_id = challenge.base.id.to_string();
        let challenger = challenge.base.challenger.user.id.to_string();
        let is_external_challenge = challenger != self.actor.context.bot_id;

        if !is_external_challenge {
            return;
        }

        log::info!("Challenge is from {}", challenger);

        let Some(Title::Bot) = challenge.base.challenger.user.title else {
            let action = Action::decline_challenge(challenge_id, Reason::OnlyBot).into();
            self.action_sender.send(action);
            return;
        };

        let VariantKey::Standard = challenge.base.variant.key else {
            let action = Action::decline_challenge(challenge_id, Reason::Variant).into();
            self.action_sender.send(action);
            return;
        };

        if self.game.is_none() && self.challenge_manager.get_outbound().is_none() {
            let action = Action::accept_challenge(challenge_id).into();
            self.action_sender.send(action);
        } else {
            self.challenge_manager.add_inbound(challenge);
        }
    }

    fn process_challenge_offline(&mut self, challenge: ChallengeJson) {
        log::info!("Bot challenged is offline.");
        self.challenge_manager.nullify_challenge(challenge);
    }

    fn process_challenge_canceled(&mut self, challenge: ChallengeJson) {
        log::info!("Challenge {} canceled by opponent", challenge.base.id);
        self.challenge_manager.nullify_challenge(challenge);
    }

    fn process_challenge_declined(&mut self, challenge: ChallengeJson) {
        log::info!("Challenge {} declined", challenge.base.id);
        self.challenge_manager.nullify_challenge(challenge);
    }

    fn process_game_start(&mut self, game_info: &GameEventInfo) {
        log::info!("Game started: id: {}", game_info.game_id);

        if let Some(challenge) = self.challenge_manager.get_outbound() {
            if game_info.game_id == challenge.challenge.base.id {
                self.challenge_manager.clear_outbound();
            }
        }

        let game = Game::from_game_start(game_info);

        if self.game.is_none() {
            self.game = game.into();
        } else {
            _ = self.other_games.insert(game.game_id.to_string(), game);
        }
    }

    fn process_game_finish(&mut self, game_info: &GameEventInfo) {
        log::info!("Game finished: id: {}", game_info.game_id);

        if let Some(current_game) = &mut self.game {
            if current_game.game_id == game_info.game_id {
                current_game.process_game_info(game_info);
                self.last_game = None;
                std::mem::swap(&mut self.game, &mut self.last_game);
            }
        }

        _ = self.other_games.remove(&game_info.game_id);
    }

    fn process_game_full(&mut self, game_id: String, game_full: GameFull) {
        log::info!("GameFull: id: {}", game_id);

        let bot_id = self.actor.context.bot_id;
        let game = Game::from_game_full(bot_id.clone(), &game_full);

        if self.game.is_some() {
            _ = self.other_games.insert(game_id.to_string(), game);
        } else {
            log::info!("Game updated: id: {}", game.game_id);
            self.game = game.into();
        }
    }

    fn process_game_state(&mut self, game_id: String, game_state: GameState) {
        log::info!("GameState: id: {}", game_id);

        if let Some(current_game) = &mut self.game {
            if current_game.game_id == game_id {
                current_game.process_game_state(game_state);
            }
        } else if let Some(game) = self.other_games.get_mut(&game_id) {
            game.process_game_state(game_state);
        }
    }

    fn process_chat_line(&mut self, game_id: String, chat_line: ChatLine) {
        // I don't currently intend to use these chat lines.
        _ = self;

        log::info!("Game {}: Chat line: {}: {}", &game_id, &chat_line.username, &chat_line.text)
    }

    fn process_opponent_gone(&mut self, game_id: String, opponent_gone: OpponentGone) {
        // I don't think bots can claim victory.
        _ = self;

        log::info!("Opponent left game {} (gone={})", &game_id, &opponent_gone.gone)
    }
}
