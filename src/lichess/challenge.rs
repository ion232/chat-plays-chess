use std::collections::HashMap;
use std::time::Instant;

use lichess_api::model::challenges::ChallengeJson;

pub type ChallengeId = String;

#[derive(Clone)]
pub struct Challenge {
    pub challenge: ChallengeJson,
    pub timestamp: Instant,
}

impl Challenge {
    pub fn new(challenge: ChallengeJson) -> Self {
        Self { challenge, timestamp: std::time::Instant::now() }
    }
}

#[derive(Default)]
pub struct ChallengeManager {
    inbound_rated: HashMap<ChallengeId, Challenge>,
    outbound_challenge: Option<Challenge>,
}

impl ChallengeManager {
    pub fn get_outbound(&self) -> Option<Challenge> {
        self.outbound_challenge.clone().into()
    }

    pub fn set_outbound(&mut self, challenge: ChallengeJson) {
        let challenge = Challenge::new(challenge);
        if let Some(current_challenge) = &self.outbound_challenge {
            let id = &challenge.challenge.base.id;
            let current_id = &current_challenge.challenge.base.id;
            log::warn!("Evicting existing outbound challenge {} for {}", current_id, id)
        }
        self.outbound_challenge = challenge.into();
    }

    pub fn clear_outbound(&mut self) {
        self.outbound_challenge = None;
    }

    pub fn add_inbound(&mut self, challenge: ChallengeJson) {
        let id = challenge.base.id.clone();
        let rated = challenge.base.rated;

        let challenge = Challenge::new(challenge);
        if rated {
            self.inbound_rated.insert(id, challenge);
        }
    }

    pub fn nullify_challenge(&mut self, challenge: ChallengeJson) {
        let id = challenge.base.id;

        self.inbound_rated.remove(&id);

        if let Some(outbound) = &self.outbound_challenge {
            if id == outbound.challenge.base.id {
                self.outbound_challenge = None;
            }
        }
    }

    pub fn remove_latest_inbound(&mut self) -> Option<ChallengeJson> {
        fn min(map: &HashMap<ChallengeId, Challenge>) -> Option<ChallengeId> {
            map.iter().min_by(|l, r| l.1.timestamp.cmp(&r.1.timestamp)).and_then(|(id, _)| id.to_owned().into())
        }

        if let Some(id) = min(&self.inbound_rated) {
            self.inbound_rated.remove(&id).map(|c| c.challenge)
        } else {
            None
        }
    }

    pub fn has_any_challenge(&self) -> bool {
        self.inbound_rated.len() > 0 || self.outbound_challenge.is_some()
    }
}
