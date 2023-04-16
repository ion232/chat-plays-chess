use crossbeam_channel::{Receiver, Sender};

use crate::lichess::action::Action as LichessAction;
use crate::lichess::game::GameId;
use crate::stream::model::Command;
use crate::twitch::action::Action as TwitchAction;

pub struct EventQueue {
    sender: Sender<Event>,
    receiver: Receiver<Event>,
}

#[derive(Clone)]
pub struct EventSender {
    sender: Sender<Event>,
}

pub enum Event {
    Action(Action),
    Notification(Notification),
}

pub enum Action {
    Lichess(LichessAction),
    Twitch(TwitchAction),
    FindNewGame,
    SwitchGame(GameId),
    Shutdown,
}

pub enum Notification {
    ChatCommand { command: Command },
    VotingFinished,
    OutboundChallengeNullified,
    GameVotesChanged,
    SettingsChanged,
    OpponentSearchStarted,
    Game(GameNotification),
}

pub enum GameNotification {
    NewCurrentGame,
    GameStarted { game_id: GameId },
    GameAbortable { game_id: GameId },
    GameFinished,
    OurTurn { game_id: GameId },
    TheirTurn { game_id: GameId },
    PlayerMoved { game_id: GameId, was_us: bool },
}

impl Default for EventQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl EventQueue {
    pub fn new() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        Self { sender, receiver }
    }

    pub fn event_sender(&self) -> EventSender {
        EventSender::new(self.sender.clone())
    }

    pub fn next(&mut self) -> Option<Event> {
        if !self.receiver.is_empty() {
            self.receiver.recv().ok()
        } else {
            None
        }
    }
}

impl EventSender {
    pub fn new(sender: Sender<Event>) -> Self {
        Self { sender }
    }

    pub fn send_action(&mut self, action: Action) {
        // let name = match &action {
        //     Action::Lichess(action) => match action {
        //         LichessAction::Account(action) => match action {
        //             AccountAction::AcceptChallenge { challenge_id } => {
        //                 format!("challenge accept ({})", challenge_id)
        //             }
        //             AccountAction::CancelChallenge { challenge_id } => {
        //                 format!("challenge cancel ({})", challenge_id)
        //             }
        //             AccountAction::DeclineChallenge { challenge_id, .. } => {
        //                 format!("challenge decline ({})", challenge_id)
        //             }
        //             AccountAction::ChallengeRandomBot => "challenge bot".to_string(),
        //         },
        //         LichessAction::Game { game_id, action } => match action {
        //             GameAction::Abort => format!("abort game ({})", game_id),
        //             GameAction::Move => format!("game move ({})", game_id),
        //             GameAction::OfferDraw => format!("draw game ({})", game_id),
        //             GameAction::Resign => format!("resign game ({})", game_id),
        //         },
        //     },
        //     Action::Twitch(..) => "twitch".to_string(),
        //     Action::ResetVoteTimer => "reset vote timer".to_string(),
        //     Action::SwitchGame(..) => "switch game".to_string(),
        //     Action::Shutdown => "shutdown".to_string(),
        // };
        // log::info!("Sending {} action", name);

        _ = self.sender.send(Event::Action(action));
    }

    pub fn send_notification(&mut self, notification: Notification) {
        _ = self.sender.send(Event::Notification(notification));
    }
}

impl From<LichessAction> for Action {
    fn from(action: LichessAction) -> Self {
        Action::Lichess(action)
    }
}
