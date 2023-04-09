use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use lichess_api::model::Speed;

use crate::{
    engine::users::Username,
    stream::model::{Delays, VoteStats},
};

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum Vote {
    Delay,
    Draw,
    Resign,
    Move(chess::ChessMove),
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

pub struct GameVotes {
    delays: Delays,
    user_moves: HashMap<Username, Option<Vote>>,
    last_vote_time: Instant,
    vote_duration: Duration,
}

impl GameVotes {
    pub fn new(speed: &Speed) -> Self {
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
            delays: Delays::new(max_delays),
            user_moves: Default::default(),
            last_vote_time: Instant::now(),
            vote_duration,
        }
    }

    pub fn reset_vote_timer(&mut self) {
        self.last_vote_time = Instant::now();
    }

    pub fn votes_ready(&self) -> bool {
        self.last_vote_time.elapsed() >= self.vote_duration
    }

    pub fn add_delay(&mut self) {
        self.delays.add_delay();
    }

    pub fn get_delays(&self) -> Delays {
        self.delays.clone()
    }

    pub fn add_vote(&mut self, user: Username, vote: Vote) {
        _ = self.user_moves.insert(user, vote.into());
    }

    pub fn get_model_game_votes(&self) -> crate::stream::model::GameVotes {
        let mut game_votes = crate::stream::model::GameVotes { votes: Default::default(), delays: self.get_delays() };

        for v in self.user_moves.values() {
            let Some(vote_string) = v.map(|v| v.to_string()) else {
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

        for vote in self.user_moves.values() {
            if let Some(vote) = vote {
                vote_counts.entry(*vote).and_modify(|count| *count += 1).or_insert(0);
            }
        }

        vote_counts.iter().max_by_key(|e| e.1).map(|e| e.0.clone())
    }

    pub fn reset(&mut self) {
        self.delays = Default::default();
        self.user_moves.clear();
        self.last_vote_time = Instant::now();
    }
}
