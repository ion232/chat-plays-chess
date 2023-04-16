use crossbeam_channel::{Receiver, Sender};

use crate::{
    engine::votes::settings::Settings,
    lichess::game::Game,
    stream::{
        audio::Clip,
        model::{Command, GameVotes, Notice, Side, State, Timer},
    },
};

pub type EventSender = Sender<Event>;
pub type EventReceiver = Receiver<Event>;

pub enum Event {
    Action(Action),
    Notification(Notification),
}

pub enum Action {
    PlayClip { clip: Clip },
    Shutdown,
}

pub enum Notification {
    ActiveGame { game: Game },
    ChatCommand { command: Command },
    Notice { notice: Notice },
    State { state: State },
    Settings { settings: Settings },
    GameVotes { votes: GameVotes },
    GameUpdate(GameUpdate),
}

pub enum GameUpdate {
    Board { board: chess::Board },
    MoveHistory { moves: Vec<String> },
    Timer { side: Side, timer: Timer },
}
