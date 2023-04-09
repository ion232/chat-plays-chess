pub mod action;
pub mod events;
pub mod moves;
pub mod users;

use std::time::Duration;

use lichess_api::model::users::User;

use rand::rngs::ThreadRng;
use rand::seq::IteratorRandom;
use rand::seq::SliceRandom;

use crate::error::Result;

use crate::engine::users::ActiveUsers;

use crate::engine::events::Event;
use crate::engine::events::EventSubscriber;

use crate::lichess::action::AccountAction;
use crate::lichess::action::Action as LichessAction;
use crate::lichess::action::GameAction;
use crate::lichess::events::Event as LichessEvent;
use crate::lichess::game::Game;
use crate::lichess::manager::LichessManager;
use crate::lichess::Context as LichessContext;

use crate::stream::model::Command;
use crate::stream::model::Model;

use crate::twitch::action::Action as TwitchAction;
use crate::twitch::command::Command as TwitchCommand;
use crate::twitch::command::Setting;
use crate::twitch::events::ChatCommand;
use crate::twitch::events::Event as TwitchEvent;
use crate::twitch::Context as TwitchContext;

use self::action::Action;
use self::action::ActionReceiver;
use self::action::ActionSender;
use self::moves::Vote;

pub struct Engine {
    pub(crate) active_users: ActiveUsers,
    pub model: Model,
    pub(crate) event_subscriber: EventSubscriber,
    pub(crate) action_receiver: ActionReceiver,
    pub(crate) lichess_manager: LichessManager,
    pub(crate) rng: ThreadRng,
}

impl Engine {
    pub fn new(lichess_context: LichessContext, twitch_context: TwitchContext) -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        let action_receiver = ActionReceiver::new(receiver);
        let action_sender = ActionSender::new(sender);

        Engine {
            active_users: ActiveUsers::new(),
            model: Default::default(),
            event_subscriber: EventSubscriber::new(lichess_context.clone(), twitch_context),
            action_receiver,
            lichess_manager: LichessManager::new(lichess_context, action_sender),
            rng: rand::thread_rng(),
        }
    }

    pub async fn setup(&mut self) -> Result<()> {
        self.event_subscriber.subscribe_to_all().await?;

        tokio::time::sleep(Duration::from_secs(5)).await;

        // while self.event_subscriber.has_event() {
        //     type LichessAccountEvent = lichess_api::model::board::stream::events::Event;

        //     match self.event_subscriber.next_event().await? {
        //         Event::LichessEvent(event) => match event {
        //             LichessEvent::AccountEvent { event } => match event {
        //                 LichessAccountEvent::Challenge { challenge } => {
        //                     let challenge_id = challenge.base.id.to_string();
        //                     let reason = Reason::Generic;
        //                     if challenge_id == self.lichess_manager.actor.context.bot_id {
        //                         if let Err(error) = self.lichess_manager.actor.cancel_challenge(challenge_id).await {
        //                             log::error!("Cancel challenge error for {}: {}", challenge.base.id, error)
        //                         } else {
        //                             log::info!("Successfully cancelled challenge {}", challenge.base.id);
        //                         }
        //                     } else {
        //                         if let Err(error) = self.lichess_manager.actor.decline_challenge(challenge_id, reason).await {
        //                             log::error!("Decline challenge error for {}: {}", challenge.base.id, error)
        //                         } else {
        //                             log::info!("Successfully declined challenge {}", challenge.base.id);
        //                         }
        //                     }
        //                 }
        //                 LichessAccountEvent::GameStart { game } => {
        //                     let game_id = game.game_id;
        //                     if let Err(error) = self.lichess_manager.actor.abort(&game_id).await {
        //                         log::warn!("Abort error for {}: {}", game_id, error);
        //                         log::info!("Trying resign instead...");
        //                         if let Err(error) = self.lichess_manager.actor.resign(&game_id).await {
        //                             log::error!("Resign error for {}: {}", game_id, error);
        //                         } else {
        //                             log::info!("Successfully resigned.");
        //                         }
        //                     }
        //                 }
        //                 _ => {}
        //             },
        //             _ => {}
        //         },
        //         Event::TwitchEvent(_) => {}
        //     }
        // }

        Ok(())
    }

    pub async fn process(&mut self) -> Result<()> {
        tokio::time::sleep(Duration::from_millis(1)).await;

        if self.event_subscriber.has_event() {
            let event = self.event_subscriber.next_event().await?;
            self.process_event(event).await;
        }
        self.process_action_queue().await;

        self.manage_state();
        self.process_action_queue().await;

        Ok(())
    }

    async fn process_action_queue(&mut self) {
        while let Some(action) = self.action_receiver.next() {
            match action {
                Action::Lichess(action) => self.process_lichess_action(action).await,
                Action::Twitch(action) => self.process_twitch_action(action).await,
                Action::ResetVoteTimer => self.reset_vote_timer(),
                Action::SwitchGame(game) => self.switch_game(game),
                Action::Shutdown => {}
            }
        }
    }

    async fn process_lichess_action(&mut self, action: LichessAction) {
        match action {
            LichessAction::Account(action) => match action {
                AccountAction::AcceptChallenge { challenge_id } => {
                    log::info!("Accepting challenge: id {}", &challenge_id);
                    if let Err(error) = self.lichess_manager.actor.accept_challenge(challenge_id).await {
                        log::error!("Accept challenge error: {}", error)
                    }
                }
                AccountAction::CancelChallenge { challenge_id } => {
                    log::info!("Canceling challenge: id {}", &challenge_id);
                    let result = self.lichess_manager.actor.cancel_challenge(challenge_id).await;
                    if let Err(error) = result {
                        log::error!("Cancel challenge error: {}", error);
                    }
                }
                AccountAction::DeclineChallenge { challenge_id, reason } => {
                    log::info!("Declining challenge: id {}", &challenge_id);
                    let result = self.lichess_manager.actor.decline_challenge(challenge_id, reason).await;
                    if let Err(error) = result {
                        log::error!("Decline challenge error: {}", error);
                    }
                }
                AccountAction::ChallengeRandomBot => self.challenge_random_bot().await,
            },
            LichessAction::Game { game_id, action } => match action {
                GameAction::Abort => {
                    log::info!("Aborting game {}", &game_id);
                    let result = self.lichess_manager.actor.abort(&game_id).await;
                    if let Err(error) = result {
                        log::error!("Game abort error: {}", error);
                    }
                }
                GameAction::Move => {
                    log::info!("Making move {}", &game_id);
                    self.make_move(game_id).await;
                }
                GameAction::OfferDraw => {
                    log::info!("Offering to draw game {}", &game_id);
                    let result = self.lichess_manager.actor.offer_draw(&game_id).await;
                    if let Err(error) = result {
                        log::error!("Game draw error: {}", error);
                    }
                }
                GameAction::Resign => {
                    log::info!("Resigning game {}", &game_id);
                    let result = self.lichess_manager.actor.resign(&game_id).await;
                    if let Err(error) = result {
                        log::error!("Game resign error: {}", error);
                    }
                }
            },
        }
    }

    fn reset_vote_timer(&mut self) {
        self.active_users.reset_vote_timer();
    }

    fn switch_game(&mut self, game: Game) {
        log::info!("Switching to game {}", &game.game_id);

        self.active_users.handle_new_game(&game);

        self.lichess_manager.set_game(game);
        self.lichess_manager.update_model(&mut self.model);
    }

    async fn challenge_random_bot(&mut self) {
        log::info!("Challenging random bot...");

        if self.lichess_manager.in_game() {
            log::info!("...but already in game.");
            return;
        }

        let Ok(bots) = self.lichess_manager.actor.get_online_bots().await else {
            return;
        };

        let bots: Vec<User> = bots
            .into_iter()
            .filter(|bot| {
                let tos_violation = bot.tos_violation.unwrap_or(false);
                let disabled = bot.disabled.unwrap_or(false);
                let mut has_bullet = false;
                if let Some(bullet) = &bot.perfs.bullet {
                    has_bullet = bullet.games > 0;
                }
                return !tos_violation && !disabled && has_bullet;
            })
            .collect();

        let Some(bot) = bots.choose(&mut self.rng) else {
            return;
        };
        let settings = self.active_users.get_settings();

        let mut clocks = Vec::<(u32, u32)>::default();
        if bot.perfs.classical.is_some() && settings.game_modes.classical {
            clocks.push((1800, 0));
        }
        if bot.perfs.rapid.is_some() && settings.game_modes.rapid {
            clocks.push((600, 10));
        }
        if bot.perfs.blitz.is_some() && settings.game_modes.blitz {
            clocks.push((300, 3));
        }
        if bot.perfs.bullet.is_some() {
            clocks.push((120, 1));
        }

        let Some((limit, increment)) = clocks.choose(&mut self.rng) else {
            return;
        };

        let user = bot.username.to_string();
        log::info!("Creating challenge to bot {} ...", &user);

        let result = self.lichess_manager.actor.create_challenge(user, *limit, *increment).await;
        match result {
            Ok(challenge) => {
                log::info!("Created challenge: id {}", &challenge.challenge.base.id);
                self.lichess_manager.add_outbound(challenge)
            }
            Err(error) => log::error!("Create challenge error: {}", error),
        }
    }

    async fn make_move(&mut self, game_id: String) {
        let Some(game) = self.lichess_manager.get_game(&game_id) else {
            return;
        };

        if !game.is_our_turn || !self.active_users.votes_ready() {
            return;
        }

        log::info!("It's our turn and votes are ready in game {}", &game_id);

        let Some(vote) = self.active_users.get_top_vote() else {
            let move_gen = chess::MoveGen::new_legal(&game.board);
            if let Some(chess_move) = move_gen.choose(&mut self.rng) {
                log::info!("Making random move {} in game {}", chess_move.to_string(), &game_id);
                let result = self.lichess_manager.actor.make_move(&game_id, chess_move).await;
                if let Err(error) = result {
                    log::error!("Make move error: {}", error.to_string())
                }
            }
            return;
        };

        log::info!("Top vote acquired for game {}", &game_id);

        let success = match vote {
            Vote::Delay => true,
            Vote::Draw => self.lichess_manager.actor.offer_draw(&game_id).await.is_ok(),
            Vote::Resign => self.lichess_manager.actor.resign(&game_id).await.is_ok(),
            Vote::Move(chess_move) => self.lichess_manager.actor.make_move(&game_id, chess_move).await.is_ok(),
        };

        if success {
            if vote == Vote::Delay {
                self.active_users.add_delay();
            } else {
                self.active_users.reset_game_votes();
            }
        }
    }

    async fn process_twitch_action(&mut self, action: TwitchAction) {
        _ = action;
    }

    fn manage_state(&mut self) {
        if self.lichess_manager.in_game() {
            self.lichess_manager.manage_state();
        } else {
            self.lichess_manager.find_new_opponent();
        }

        self.active_users.update_model(&mut self.model);
        self.lichess_manager.update_model(&mut self.model);
    }

    async fn process_event(&mut self, event: Event) {
        match event {
            Event::LichessEvent(event) => self.process_lichess_event(event).await,
            Event::TwitchEvent(event) => self.process_twitch_event(event),
        }
    }

    async fn process_lichess_event(&mut self, event: LichessEvent) {
        match &event {
            LichessEvent::AccountEvent { event } => {
                type StreamEvent = lichess_api::model::board::stream::events::Event;
                match event {
                    StreamEvent::GameStart { game } => _ = self.event_subscriber.handle_game_start(&game.game_id).await,
                    StreamEvent::GameFinish { game } => {
                        _ = self.event_subscriber.handle_game_finished(&game.game_id).await
                    }
                    _ => {}
                }
            }
            LichessEvent::GameEvent { game_id, .. } => {
                _ = self.event_subscriber.handle_game_start(&game_id).await;
            }
        }

        self.lichess_manager.process_lichess_event(event);
    }

    fn process_twitch_event(&mut self, event: TwitchEvent) {
        match event {
            TwitchEvent::ChatCommand(ChatCommand { user, command }) => {
                self.process_chat_command(user, command);
            }
            TwitchEvent::ChatMessage(_) => {}
            TwitchEvent::BitsDonation(_) => {}
        }
    }

    fn process_chat_command(&mut self, user: String, command: TwitchCommand) {
        let chat_command = Command::new(user.to_string(), command.to_string());
        self.model.chat_commands.push(chat_command);

        match command {
            crate::twitch::command::Command::VoteMove { chess_move } => {
                self.process_move_vote(user, chess_move);
            }
            crate::twitch::command::Command::VoteSetting { setting, on } => {
                self.process_setting_vote(user, setting, on);
            }
        }
    }

    fn process_move_vote(&mut self, user: String, vote: String) {
        if let Some(vote) = self.convert_vote_string(vote) {
            self.active_users.add_move_vote(user, vote);
        }
    }

    fn convert_vote_string(&mut self, vote: String) -> Option<Vote> {
        let vote = vote.to_lowercase();

        if vote == "delay" && self.active_users.can_delay() {
            self::moves::Vote::Delay.into()
        } else if vote == "draw" {
            self::moves::Vote::Draw.into()
        } else if vote == "resign" {
            self::moves::Vote::Resign.into()
        } else if let Some(chess_move) = self.lichess_manager.convert_move(vote) {
            self::moves::Vote::Move(chess_move).into()
        } else {
            None
        }
    }

    fn process_setting_vote(&mut self, user: String, setting: Setting, on: bool) {
        self.active_users.add_settings_vote(user, setting, on);
    }
}

#[derive(Default, Clone)]
pub struct Settings {
    pub game_modes: GameModes,
    pub blitz: usize,
    pub rapid: usize,
    pub classical: usize,
    pub total: usize,
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
            "Bullet: always on".to_owned(),
            format!("Blitz: {} ({}%)", to_str(self.game_modes.blitz), self.blitz / total),
            format!("Rapid: {} ({}%)", to_str(self.game_modes.rapid), self.rapid / total),
            format!("Classical: {} ({}%)", to_str(self.game_modes.classical), self.classical / total),
        ]
    }
}

#[derive(Clone)]
pub struct GameModes {
    pub blitz: bool,
    pub rapid: bool,
    pub classical: bool,
}

impl Default for GameModes {
    fn default() -> Self {
        Self { blitz: true, rapid: true, classical: true }
    }
}
