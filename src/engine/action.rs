use crossbeam_channel::Receiver;
use crossbeam_channel::Sender;

use crate::lichess::action::AccountAction;
use crate::lichess::action::Action as LichessAction;
use crate::lichess::action::GameAction;

use crate::lichess::game::Game;
use crate::twitch::action::Action as TwitchAction;

pub enum Action {
    Lichess(LichessAction),
    Twitch(TwitchAction),
    ResetVoteTimer,
    SwitchGame(Game),
    Shutdown,
}

impl From<LichessAction> for Action {
    fn from(action: LichessAction) -> Self {
        Action::Lichess(action)
    }
}

#[derive(Clone)]
pub struct ActionSender {
    sender: Sender<Action>,
}

pub struct ActionReceiver {
    receiver: Receiver<Action>,
}

impl ActionSender {
    pub fn new(sender: Sender<Action>) -> Self {
        Self { sender }
    }

    pub fn send(&mut self, action: Action) {
        let name = match &action {
            Action::Lichess(action) => match action {
                LichessAction::Account(action) => match action {
                    AccountAction::AcceptChallenge { challenge_id } => {
                        format!("challenge accept ({})", challenge_id)
                    }
                    AccountAction::CancelChallenge { challenge_id } => {
                        format!("challenge cancel ({})", challenge_id)
                    }
                    AccountAction::DeclineChallenge { challenge_id, .. } => {
                        format!("challenge decline ({})", challenge_id)
                    }
                    AccountAction::ChallengeRandomBot => "challenge bot".to_string(),
                },
                LichessAction::Game { game_id, action } => match action {
                    GameAction::Abort => format!("abort game ({})", game_id),
                    GameAction::Move => format!("game move ({})", game_id),
                    GameAction::OfferDraw => format!("draw game ({})", game_id),
                    GameAction::Resign => format!("resign game ({})", game_id),
                },
            },
            Action::Twitch(..) => "twitch".to_string(),
            Action::ResetVoteTimer => "reset vote timer".to_string(),
            Action::SwitchGame(..) => "switch game".to_string(),
            Action::Shutdown => "shutdown".to_string(),
        };
        log::info!("Sending {} action", name);

        _ = self.sender.send(action);
    }
}

impl ActionReceiver {
    pub fn new(receiver: Receiver<Action>) -> Self {
        Self { receiver }
    }

    pub fn next(&mut self) -> Option<Action> {
        if !self.receiver.is_empty() {
            self.receiver.recv().ok()
        } else {
            None
        }
    }
}
