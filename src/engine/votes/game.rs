use std::{collections::HashMap, time::Duration};

use lichess_api::model::Speed;
use tokio::task::JoinHandle;

use crate::lichess::action::Action as LichessAction;
use crate::{
    engine::events::internal::EventSender,
    stream::model::{Delays, VoteStats},
};
use crate::{
    engine::events::internal::{Action, Notification},
    lichess::game::GameId,
};

use super::Username;

pub struct VoteTracker {
    enabled: bool,
    delays: Delays,
    votes: HashMap<Username, Option<Vote>>,
    vote_duration: Duration,
    vote_timer_handle: Option<JoinHandle<()>>,
    event_sender: EventSender,
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum Vote {
    Delay,
    Draw,
    Resign,
    Move(chess::ChessMove),
}

impl VoteTracker {
    pub fn new(speed: &Speed, event_sender: EventSender) -> Self {
        let (max_delays, vote_duration) = match speed {
            Speed::UltraBullet => (3, 2),
            Speed::Bullet => (5, 5),
            Speed::Blitz => (6, 12),
            Speed::Rapid => (8, 36),
            Speed::Classical => (10, 72),
            _ => (1, 1),
        };
        let vote_duration = Duration::from_secs(vote_duration);

        Self {
            enabled: false,
            delays: Delays::new(max_delays),
            votes: Default::default(),
            vote_duration,
            vote_timer_handle: None,
            event_sender,
        }
    }

    pub fn add_vote(&mut self, user: Username, vote: Vote) {
        if !self.enabled {
            return;
        }

        if !self.delays.can_delay() && vote == Vote::Delay {
            return;
        };

        _ = self.votes.insert(user, vote.into());
    }

    pub fn add_delay(&mut self) {
        self.delays.add_delay();
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn schedule_move(&mut self, game_id: GameId) {
        let mut event_sender = self.event_sender.clone();
        let vote_duration = self.vote_duration.clone();

        self.vote_timer_handle = tokio::task::spawn(async move {
            tokio::time::sleep(vote_duration).await;
            event_sender.send_action(Action::Lichess(LichessAction::make_move(game_id)));
            event_sender.send_notification(Notification::VotingFinished);
        })
        .into();
    }

    pub fn game_votes(&self) -> crate::stream::model::GameVotes {
        let mut game_votes = crate::stream::model::GameVotes {
            votes: Default::default(),
            delays: self.delays.clone(),
        };

        for vote in self.votes.values() {
            let Some(vote_string) = vote.map(|vote| vote.to_string()) else {
                continue;
            };

            let Some(vote_stats) = game_votes.votes.get_mut(&vote_string) else {
                let vote_stats = VoteStats {
                    vote_changes: 0,
                    total_votes: 1,
                };
                game_votes.votes.insert(vote_string, vote_stats);
                continue;
            };

            vote_stats.total_votes += 1;
        }

        game_votes
    }

    pub fn get_top_vote(&self) -> Option<Vote> {
        let mut vote_counts = HashMap::<Vote, u32>::default();

        for vote in self.votes.values() {
            if let Some(vote) = vote {
                vote_counts.entry(*vote).and_modify(|count| *count += 1).or_insert(0);
            }
        }

        vote_counts.iter().max_by_key(|e| e.1).map(|e| e.0.clone())
    }

    pub fn reset(&mut self) {
        self.delays = Default::default();
        self.reset_voting();
    }

    pub fn reset_voting(&mut self) {
        self.votes.clear();
    }
}

impl ToString for Vote {
    fn to_string(&self) -> String {
        match self {
            Vote::Delay => "delay".to_string(),
            Vote::Draw => "draw".to_string(),
            Vote::Resign => "resign".to_string(),
            Vote::Move(chess_move) => chess_move.to_string(),
        }
    }
}
