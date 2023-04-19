use std::time::Instant;
use std::{collections::HashMap, time::Duration};

use lichess_api::model::{
    challenges::{decline::Reason, ChallengeJson, Status},
    Title, VariantKey,
};
use tokio::task::JoinHandle;

use crate::engine::events::internal::Action;
use crate::engine::events::internal::EventSender;
use crate::engine::events::internal::Notification;
use crate::lichess::action::Action as LichessAction;

pub type ChallengeId = String;

const MAX_OUTBOUND_CHALLENGE_WAIT_TIME: Duration = Duration::from_secs(20);

pub struct ChallengeManager {
    our_id: String,
    outbound: Option<OutboundChallenge>,
    event_sender: EventSender,
}

pub struct OutboundChallenge {
    challenge: Challenge,
    cancel_handle: JoinHandle<()>,
}

#[derive(Clone)]
pub struct Challenge {
    pub challenge: ChallengeJson,
    pub timestamp: Instant,
}

impl ChallengeManager {
    pub fn new(our_id: String, event_sender: EventSender) -> Self {
        Self { our_id, outbound: Default::default(), event_sender }
    }

    pub fn outbound(&self) -> &Option<OutboundChallenge> {
        &self.outbound
    }

    pub fn cancel_outbound(&mut self) {
        if let Some(outbound) = &self.outbound {
            outbound.cancel_handle.abort();

            // let challenge_id = outbound.challenge.challenge.base.id.to_string();
            // let action = Action::Lichess(LichessAction::cancel_challenge(challenge_id));
            // self.event_sender.send_action(action);
        }
        self.outbound = None;
    }

    pub fn process_challenge(&mut self, challenge: ChallengeJson) {
        log::info!("Challenge event received: id: {}", challenge.base.id);

        match challenge.base.status {
            Status::Created => self.process_challenge_created(challenge),
            Status::Offline => self.process_challenge_offline(challenge),
            Status::Canceled => self.process_challenge_canceled(challenge),
            Status::Declined => self.process_challenge_declined(challenge),
            Status::Accepted => self.process_challenge_accepted(challenge),
        }
    }

    fn process_challenge_created(&mut self, challenge: ChallengeJson) {
        log::info!("Challenge {} created", &challenge.base.id);

        let challenge_id = challenge.base.id.to_string();
        let challenger = challenge.base.challenger.user.id.to_string();

        if challenger != self.our_id {
            return;
        }

        let mut event_sender = self.event_sender.clone();
        let handle = tokio::task::spawn(async move {
            tokio::time::sleep(MAX_OUTBOUND_CHALLENGE_WAIT_TIME).await;
            let action = Action::Lichess(LichessAction::cancel_challenge(challenge_id));
            event_sender.send_action(action);
        });

        self.outbound = OutboundChallenge::new(challenge, handle).into();
    }

    fn process_challenge_offline(&mut self, challenge: ChallengeJson) {
        log::info!("Challenge opponent offline: {}", challenge.base.id);
        self.nullify_challenge(challenge);
    }

    pub fn process_challenge_canceled(&mut self, challenge: ChallengeJson) {
        log::info!("Challenge canceled: {}", challenge.base.id);
        self.nullify_challenge(challenge);
    }

    pub fn process_challenge_declined(&mut self, challenge: ChallengeJson) {
        log::info!("Challenge declined: {}", challenge.base.id);
        self.nullify_challenge(challenge);
    }

    fn process_challenge_accepted(&mut self, challenge: ChallengeJson) {
        log::info!("Challenge accepted: {}", challenge.base.id);
        self.nullify_challenge(challenge);
    }

    pub fn nullify_challenge(&mut self, challenge: ChallengeJson) {
        let mut is_outbound = false;

        if let Some(outbound) = &self.outbound {
            if challenge.base.id == outbound.challenge.challenge.base.id {
                self.event_sender.send_notification(Notification::OutboundChallengeNullified);
                outbound.cancel_handle.abort();
                is_outbound = true;
            }
        };

        if is_outbound {
            self.outbound = None;
        }
    }
}

impl OutboundChallenge {
    pub fn new(challenge: ChallengeJson, cancel_handle: JoinHandle<()>) -> Self {
        Self { challenge: Challenge::new(challenge), cancel_handle }
    }
}

impl Challenge {
    pub fn new(challenge: ChallengeJson) -> Self {
        Self { challenge, timestamp: std::time::Instant::now() }
    }
}
