pub mod events;
pub mod votes;

use std::time::Duration;

use lichess_api::model::users::User;
use lichess_api::model::Speed;

use rand::prelude::Distribution;
use rand::rngs::ThreadRng;
use rand::seq::IteratorRandom;
use rand::seq::SliceRandom;

use crate::error::Result;

use crate::engine::events::external;
use crate::engine::events::internal;
use crate::engine::events::stream;

use crate::lichess::action::AccountAction;
use crate::lichess::action::Action as LichessAction;
use crate::lichess::action::Actor as LichessActor;
use crate::lichess::action::GameAction;
use crate::lichess::challenge::ChallengeManager;
use crate::lichess::events::Event as LichessEvent;
use crate::lichess::game::GameManager;
use crate::lichess::Context as LichessContext;

use crate::stream::audio::Clip;
use crate::stream::model::Command;

use crate::stream::model::Side;
use crate::stream::model::State;
use crate::twitch::action::Action as TwitchAction;
use crate::twitch::command::Command as TwitchCommand;
use crate::twitch::command::Setting;
use crate::twitch::events::ChatCommand;
use crate::twitch::events::Event as TwitchEvent;
use crate::twitch::Context as TwitchContext;

use self::events::internal::Action;
use self::events::internal::GameNotification;
use self::events::internal::Notification;
use self::votes::game::Vote;

pub struct Engine {
    game_votes: self::votes::game::VoteTracker,
    settings_votes: self::votes::settings::VoteTracker,
    external_events: external::EventManager,
    internal_queue: internal::EventQueue,
    stream_events: stream::EventSender,
    challenge_manager: ChallengeManager,
    game_manager: GameManager,
    lichess_actor: LichessActor,
    is_running: bool,
    rng: ThreadRng,
}

impl Engine {
    pub fn new(
        stream_events: stream::EventSender,
        lichess_context: LichessContext,
        twitch_context: TwitchContext,
    ) -> Self {
        let our_id = lichess_context.our_id.to_string();
        let internal_queue = internal::EventQueue::default();
        internal_queue.event_sender().send_action(Action::FindNewGame);

        Engine {
            game_votes: self::votes::game::VoteTracker::new(
                &Speed::Blitz,
                internal_queue.event_sender(),
            ),
            settings_votes: self::votes::settings::VoteTracker::new(internal_queue.event_sender()),
            external_events: external::EventManager::new(lichess_context.clone(), twitch_context),
            stream_events,
            challenge_manager: ChallengeManager::new(
                our_id.to_string(),
                internal_queue.event_sender(),
            ),
            game_manager: GameManager::new(our_id, internal_queue.event_sender()),
            internal_queue,
            lichess_actor: LichessActor::new(lichess_context),
            is_running: true,
            rng: rand::thread_rng(),
        }
    }

    pub async fn setup(&mut self) -> Result<()> {
        self.external_events.subscribe_to_all().await?;

        // Wait a short amount of time for events to arrive.
        tokio::time::sleep(Duration::from_secs(3)).await;

        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut now = tokio::time::Instant::now();

        while self.is_running {
            self.process(&mut now).await?;
        }

        Ok(())
    }

    pub async fn process(&mut self, now: &mut tokio::time::Instant) -> Result<()> {
        // Update clock timers.
        // Would normally use events - but this way avoids log spam.
        if now.elapsed() > Duration::from_millis(1000) {
            self.game_manager.advance_clocks(now.elapsed());
            *now = tokio::time::Instant::now();

            if let Some(current_game) = self.game_manager.current_game() {
                let (side, timer) = if current_game.is_our_turn {
                    (Side::Ours, current_game.us.timer)
                } else {
                    (Side::Theirs, current_game.opponent.timer)
                };

                let game_update = stream::GameUpdate::Timer { side, timer };
                let notification = stream::Notification::GameUpdate(game_update);
                _ = self.stream_events.send(stream::Event::Notification(notification));
            }
        }

        // Check for errors as well and ensure we can recover from a broken or ended stream.
        if let Ok(Some(event)) = self.external_events.next_event() {
            log::info!("External event: {event:?}");
            self.process_external_event(event).await;
        } else {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }

        while let Some(event) = self.internal_queue.next() {
            log::info!("Internal event: {event:?}");
            self.process_internal_event(event).await;
        }

        Ok(())
    }

    async fn process_external_event(&mut self, event: external::Event) {
        match event {
            external::Event::Lichess(event) => self.process_lichess_event(event).await,
            external::Event::Twitch(event) => self.process_twitch_event(event),
        }
    }

    async fn process_internal_event(&mut self, event: internal::Event) {
        match event {
            internal::Event::Action(action) => {
                self.process_action(action).await;
            }
            internal::Event::Notification(notification) => {
                self.process_notification(notification);
            }
        }
    }

    async fn process_action(&mut self, action: Action) {
        match action {
            Action::Lichess(action) => self.process_lichess_action(action).await,
            Action::Twitch(action) => self.process_twitch_action(action).await,
            Action::PlayClip(clip) => {
                let action = stream::Action::PlayClip { clip };
                _ = self.stream_events.send(stream::Event::Action(action));
            }
            Action::FindNewGame => self.find_new_game().await,
            Action::SwitchGame(game) => self.game_manager.switch_game(&game),
            Action::Shutdown => self.is_running = false,
        }
    }

    fn process_notification(&mut self, notification: Notification) {
        match notification {
            Notification::ChatCommand(chat_command) => {
                let command = Command::new(chat_command.user, chat_command.command.to_string());
                let notification = stream::Notification::ChatCommand { command };
                _ = self.stream_events.send(stream::Event::Notification(notification));
            }
            Notification::OutboundChallengeNullified => {
                if self.game_manager.current_game().is_none() {
                    self.internal_queue.event_sender().send_action(Action::FindNewGame);
                }
            }
            Notification::GameVotesChanged => {
                let votes = self.game_votes.game_votes();
                let notification = stream::Notification::GameVotes { votes };
                _ = self.stream_events.send(stream::Event::Notification(notification));
            }
            Notification::SettingsChanged => {
                let settings = self.settings_votes.settings();
                let notification = stream::Notification::Settings { settings };
                _ = self.stream_events.send(stream::Event::Notification(notification));
            }
            Notification::ChallengeSent { id, rating } => {
                let notification =
                    stream::Notification::State { state: State::ChallengingUser { id, rating } };
                _ = self.stream_events.send(stream::Event::Notification(notification));
            }
            Notification::VotingFinished => {
                if let Some(Vote::Delay) = self.game_votes.get_top_vote() {
                    self.game_votes.enable();
                } else {
                    self.game_votes.disable();
                }
            }
            Notification::Game(notification) => match notification {
                GameNotification::NewCurrentGame => {
                    self.game_votes.enable();
                    self.game_votes.reset();

                    if let Some(game) = self.game_manager.current_game() {
                        let notification = stream::Notification::ActiveGame { game: game.clone() };
                        _ = self.stream_events.send(stream::Event::Notification(notification));

                        let action = stream::Action::PlayClip { clip: Clip::Start };
                        _ = self.stream_events.send(stream::Event::Action(action))
                    }
                }
                GameNotification::GameStarted { game_id } => {
                    self.challenge_manager.cancel_outbound();

                    if let Some(game) = &self.game_manager.current_game() {
                        if game.game_id == game_id {
                            return;
                        }
                    }

                    let mut event_sender = self.internal_queue.event_sender();

                    event_sender.send_action(Action::SwitchGame(game_id.to_string()));

                    tokio::task::spawn(async move {
                        tokio::time::sleep(Duration::from_secs(30)).await;
                        event_sender.send_notification(Notification::Game(
                            GameNotification::GameAbortable { game_id },
                        ));
                    });
                }
                GameNotification::GameAbortable { game_id } => {
                    // Attempt to abort the game.
                    let action = Action::Lichess(LichessAction::abort(game_id));
                    self.internal_queue.event_sender().send_action(action);
                }
                GameNotification::GameFinished => {
                    if self.game_manager.current_game().is_some() {
                        return;
                    }

                    self.internal_queue.event_sender().send_action(Action::FindNewGame);

                    if let Some(last_game) = self.game_manager.last_game() {
                        let notification =
                            stream::Notification::ActiveGame { game: last_game.clone() };
                        _ = self.stream_events.send(stream::Event::Notification(notification));
                    }

                    let notification = stream::Notification::State { state: State::GameFinished };
                    _ = self.stream_events.send(stream::Event::Notification(notification));
                }
                GameNotification::OurTurn { game_id } => {
                    let Some(game) = self.game_manager.current_game() else {
                        return;
                    };

                    self.game_votes.enable();

                    if game.game_id == game_id {
                        self.game_votes.schedule_action_vote(game_id);
                    }

                    let notification = stream::Notification::State { state: State::OurTurn };
                    _ = self.stream_events.send(stream::Event::Notification(notification));
                }
                GameNotification::TheirTurn { game_id } => {
                    // Not sure if we really need to do anything here?
                    log::info!("Opponents turn in game {}", game_id);
                    let notification = stream::Notification::State { state: State::TheirTurn };
                    _ = self.stream_events.send(stream::Event::Notification(notification));
                }
                GameNotification::PlayerMoved { game_id, was_us } => {
                    // If we moved, we can use this opportunity to switch to another game.
                    let Some(current_game) = self.game_manager.current_game() else {
                        return;
                    };
                    if game_id == current_game.game_id {
                        let game_update =
                            stream::GameUpdate::Board { board: current_game.board.clone() };
                        let notification = stream::Notification::GameUpdate(game_update);
                        _ = self.stream_events.send(stream::Event::Notification(notification));

                        let game_update = stream::GameUpdate::MoveHistory {
                            moves: current_game.move_history.clone(),
                        };
                        let notification = stream::Notification::GameUpdate(game_update);
                        _ = self.stream_events.send(stream::Event::Notification(notification));

                        let side = if was_us { Side::Ours } else { Side::Theirs };
                        let timer = if was_us {
                            current_game.us.timer
                        } else {
                            current_game.opponent.timer
                        };

                        let game_update = stream::GameUpdate::Timer { side, timer };
                        let notification = stream::Notification::GameUpdate(game_update);
                        _ = self.stream_events.send(stream::Event::Notification(notification));
                    }
                }
            },
        }
    }

    async fn process_lichess_action(&mut self, action: LichessAction) {
        match action {
            LichessAction::Account(action) => match action {
                AccountAction::AcceptChallenge { challenge_id } => {
                    _ = self.lichess_actor.accept_challenge(challenge_id).await;
                }
                AccountAction::CancelChallenge { challenge_id } => {
                    _ = self.lichess_actor.cancel_challenge(challenge_id).await;
                }
                AccountAction::DeclineChallenge { challenge_id, reason } => {
                    _ = self.lichess_actor.decline_challenge(challenge_id, reason).await;
                }
                AccountAction::ChallengeRandomBot => {
                    self.challenge_random_bot().await;
                }
            },
            LichessAction::Game { game_id, action } => match action {
                GameAction::Abort => {
                    _ = self.lichess_actor.abort(&game_id).await;
                }
                GameAction::Move => {
                    _ = self.make_move(game_id).await;
                }
                GameAction::OfferDraw => {
                    _ = self.lichess_actor.offer_draw(&game_id).await;
                }
                GameAction::Resign => {
                    _ = self.lichess_actor.resign(&game_id).await;
                }
            },
        }
    }

    async fn find_new_game(&mut self) {
        if self.game_manager.current_game().is_none() {
            self.find_new_opponent();
        } else {
            log::warn!("Cannot find new game - already in a game.")
        }
    }

    pub fn find_new_opponent(&mut self) {
        if let Some(game_id) = self.game_manager.oldest_game_id() {
            self.game_manager.switch_game(&game_id);
        } else {
            if self.challenge_manager.outbound().is_some() {
                self.challenge_manager.cancel_outbound();
            }

            self.internal_queue
                .event_sender()
                .send_action(LichessAction::challenge_random_bot().into());
        }
    }

    async fn challenge_random_bot(&mut self) {
        log::info!("Challenging random bot...");

        let Ok(bots) = self.lichess_actor.get_online_bots().await else {
            self.internal_queue
            .event_sender()
            .send_action(Action::Lichess(LichessAction::challenge_random_bot()));
            return;
        };

        let bots: Vec<User> = bots
            .into_iter()
            .filter(|bot| {
                let tos_violation = bot.tos_violation.unwrap_or(false);
                let disabled = bot.disabled.unwrap_or(false);

                let valid_blitz = bot
                    .perfs
                    .blitz
                    .as_ref()
                    .and_then(|blitz| {
                        Some(blitz.rating != 0 && blitz.prov.unwrap_or(true) && blitz.games > 0)
                    })
                    .unwrap_or(false);

                return !tos_violation && !disabled && valid_blitz;
            })
            .collect();

        if bots.is_empty() {
            self.internal_queue
                .event_sender()
                .send_action(Action::Lichess(LichessAction::challenge_random_bot()));
            return;
        }

        // Turns out to be a decent distribution.
        let weights = bots
            .iter()
            .map(|bot| (500_000.0 / bot.perfs.blitz.as_ref().unwrap().rating as f32) as u64);
        let distribution = rand::distributions::WeightedIndex::new(weights).unwrap();
        let bot = &bots[distribution.sample(&mut self.rng)];

        let settings = self.settings_votes.settings();

        let mut rating = bot.perfs.blitz.as_ref().unwrap().rating;
        let mut clocks = Vec::<(u32, u32)>::default();

        if bot.perfs.classical.is_some() && settings.game_modes.classical {
            rating = bot.perfs.classical.as_ref().unwrap().rating;
            clocks.push((1800, 0));
        }
        if bot.perfs.rapid.is_some() && settings.game_modes.rapid {
            rating = bot.perfs.rapid.as_ref().unwrap().rating;
            clocks.push((600, 10));
        }
        if bot.perfs.blitz.is_some() {
            rating = bot.perfs.blitz.as_ref().unwrap().rating;
            clocks.push((300, 3));
        }
        if bot.perfs.bullet.is_some() && settings.game_modes.bullet {
            rating = bot.perfs.bullet.as_ref().unwrap().rating;
            clocks.push((120, 1));
        }

        let Some((limit, increment)) = clocks.choose(&mut self.rng) else {
            return;
        };

        let user = bot.username.to_string();
        log::info!("Creating challenge to bot {} ...", &user);

        let result = self.lichess_actor.create_challenge(user, *limit, *increment).await;
        match result {
            Ok(challenge) => {
                log::info!("Created challenge: id {}", &challenge.challenge.base.id);
                self.internal_queue.event_sender().send_notification(Notification::ChallengeSent {
                    id: bot.id.to_string(),
                    rating,
                });
            }
            Err(error) => {
                log::error!("Create challenge error: {} - retrying", error);
                self.internal_queue
                    .event_sender()
                    .send_action(Action::Lichess(LichessAction::challenge_random_bot()));
            }
        }
    }

    async fn make_move(&mut self, game_id: String) {
        let Some(vote) = self.game_votes.get_top_vote() else {
            let Some(game) = self.game_manager.game(&game_id) else {
                return;
            };

            let move_generator = chess::MoveGen::new_legal(&game.board);
            if let Some(chess_move) = move_generator.choose(&mut self.rng) {
                log::info!("Making random move {} in game {}", chess_move.to_string(), &game_id);

                let result = self.lichess_actor.make_move(&game_id, chess_move).await;
                if let Err(error) = result {
                    log::error!("Make move error: {}", error.to_string());
                    // reschedule_action_vote(self.internal_queue.event_sender(), &game_id)
                } else {
                    self.game_votes.reset();
                }
            }

            return;
        };

        log::info!("Top vote acquired for game {}", &game_id);
        let success;

        match vote {
            self::votes::game::Vote::Delay => {
                self.game_votes.add_delay();
                self.game_votes.reset_voting();
                self.game_votes.schedule_action_vote(game_id);
                return;
            }
            self::votes::game::Vote::Draw => {
                success = self.lichess_actor.offer_draw(&game_id).await.is_ok()
            }
            self::votes::game::Vote::Resign => {
                success = self.lichess_actor.resign(&game_id).await.is_ok()
            }
            self::votes::game::Vote::Move(chess_move) => {
                success = self.lichess_actor.make_move(&game_id, chess_move).await.is_ok();
            }
        };

        if success {
            self.game_votes.reset();
        }
    }

    async fn process_twitch_action(&mut self, action: TwitchAction) {
        _ = action;
    }

    async fn process_lichess_event(&mut self, event: LichessEvent) {
        type AccountEvent = lichess_api::model::bot::stream::events::Event;
        type GameEvent = lichess_api::model::bot::stream::game::Event;

        match event {
            LichessEvent::AccountEvent { event } => match event {
                AccountEvent::Challenge { challenge } => {
                    self.challenge_manager.process_challenge(challenge)
                }
                AccountEvent::ChallengeCanceled { challenge } => {
                    self.challenge_manager.process_challenge_canceled(challenge)
                }
                AccountEvent::ChallengeDeclined { challenge } => {
                    self.challenge_manager.process_challenge_declined(challenge)
                }
                AccountEvent::GameStart { game } => {
                    self.game_manager.process_game_start(&game);
                    // Start steraming game events so we get updates.
                    _ = self.external_events.stream_game(&game.game_id).await;
                }
                AccountEvent::GameFinish { game } => {
                    self.game_manager.process_game_finish(&game);
                    // Cleanup finished task.
                    _ = self.external_events.finish_streaming_game(&game.game_id).await;
                    self.internal_queue.event_sender().send_action(Action::FindNewGame);
                }
            },
            LichessEvent::GameEvent { game_id, event } => {
                match event {
                    GameEvent::GameFull { game_full } => {
                        self.game_manager.process_game_full(&game_full);
                    }
                    GameEvent::GameState { game_state } => {
                        self.game_manager.process_game_update(&game_id, &game_state);
                    }
                    GameEvent::ChatLine { chat_line } => {
                        _ = chat_line;
                        // I don't have any use for these chat lines at the moment.
                    }
                    GameEvent::OpponentGone { opponent_gone } => {
                        self.internal_queue.event_sender().send_notification(Notification::Game(
                            GameNotification::GameAbortable { game_id },
                        ));
                        self.game_manager.process_opponent_gone(&opponent_gone);
                    }
                }
            }
        }
    }

    fn process_twitch_event(&mut self, event: TwitchEvent) {
        match event {
            TwitchEvent::ChatCommand(chat_command) => {
                self.process_chat_command(chat_command);
            }
            TwitchEvent::ChatMessage(_) => {
                // Don't need these - won't be showing them all on stream, for obvious reasons.
                // Legitimate chat commands will be shown instead.
            }
        }
    }

    fn process_chat_command(&mut self, chat_command: ChatCommand) {
        self.internal_queue
            .event_sender()
            .send_notification(Notification::ChatCommand(chat_command.clone()));

        let ChatCommand { user, command } = chat_command;

        match command {
            TwitchCommand::VoteGame { action } => {
                self.process_game_vote(user, action);
            }
            TwitchCommand::VoteSetting { setting, on } => {
                self.process_settings_vote(user, setting, on);
            }
        }
    }

    fn process_game_vote(&mut self, user: String, action: String) {
        let action = action.to_lowercase();

        let vote = if action == "delay" {
            self::votes::game::Vote::Delay.into()
        } else if action == "draw" {
            self::votes::game::Vote::Draw.into()
        } else if action == "resign" {
            self::votes::game::Vote::Resign.into()
        } else if let Some(chess_move) = self.game_manager.convert_move(action) {
            self::votes::game::Vote::Move(chess_move).into()
        } else {
            None
        };

        if let Some(vote) = vote {
            self.game_votes.add_vote(user, vote);
        }
    }

    fn process_settings_vote(&mut self, user: String, setting: Setting, on: bool) {
        self.settings_votes.add_vote(user, setting, on);
    }
}
