use crossbeam_channel::{Receiver, Sender};

use crate::lichess::action::Action as LichessAction;
use crate::lichess::game::GameId;
use crate::stream::audio::Clip;
use crate::twitch::action::Action as TwitchAction;
use crate::twitch::events::ChatCommand;

pub struct EventQueue {
    sender: Sender<Event>,
    receiver: Receiver<Event>,
}

#[derive(Clone)]
pub struct EventSender {
    sender: Sender<Event>,
}

#[derive(Debug)]
pub enum Event {
    Action(Action),
    Notification(Notification),
}

#[derive(Debug)]
pub enum Action {
    Lichess(LichessAction),
    Twitch(TwitchAction),
    PlayClip(Clip),
    FindNewGame,
    SwitchGame(GameId),
    Shutdown,
}

#[derive(Debug)]
pub enum Notification {
    ChatCommand(ChatCommand),
    VotingFinished,
    OutboundChallengeNullified,
    GameVotesChanged,
    SettingsChanged,
    ChallengeSent { id: String, rating: u32 },
    Game(GameNotification),
}

#[derive(Debug)]
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
        _ = self.sender.send(Event::Action(action));
    }

    pub fn send_notification(&mut self, notification: Notification) {
        _ = self.sender.send(Event::Notification(notification));
    }
}

impl ToString for Action {
    fn to_string(&self) -> String {
        format!("{self:?}")
    }
}

impl From<LichessAction> for Action {
    fn from(action: LichessAction) -> Self {
        Action::Lichess(action)
    }
}
