use std::time::Duration;

use async_std::stream::StreamExt;

use lichess_api::model::account::profile::Profile;
use lichess_api::model::challenges::decline::Reason;
use lichess_api::model::challenges::{ChallengeBase, ChallengeCreated, CreateChallenge};
use lichess_api::model::users::User;
use lichess_api::model::VariantKey;

use crate::error::Result;

use crate::lichess::Context;

pub struct Actor {
    pub context: Context,
}

impl Actor {
    pub fn new(context: Context) -> Self {
        Self { context }
    }

    pub async fn get_account(&self) -> Result<Profile> {
        tokio::time::sleep(Duration::from_millis(100)).await;

        type Request = lichess_api::model::account::profile::GetRequest;
        self.context
            .api
            .get_profile(Request::new())
            .await
            .map_err(|e| crate::error::Error::LichessError(e))
    }

    pub async fn get_online_bots(&self) -> Result<Vec<User>> {
        tokio::time::sleep(Duration::from_millis(100)).await;

        let bot_count = 200;

        type Request = lichess_api::model::bot::online::GetRequest;
        let mut bot_stream = self
            .context
            .api
            .bot_get_online(Request::new(bot_count))
            .await
            .map_err(|e| crate::error::Error::LichessError(e))?;

        let mut bots = Vec::<User>::with_capacity(bot_count as usize);
        while let Some(Ok(user)) = bot_stream.next().await {
            bots.push(user);
        }

        tokio::time::sleep(Duration::from_millis(1000)).await;

        Ok(bots)
    }

    pub async fn create_challenge(
        &self,
        username: String,
        limit: u32,
        increment: u32,
    ) -> Result<ChallengeCreated> {
        tokio::time::sleep(Duration::from_millis(100)).await;

        let base = ChallengeBase {
            clock_limit: limit.into(),
            clock_increment: increment.into(),
            days: None,
            variant: VariantKey::Standard,
            fen: None,
        };
        let challenge = CreateChallenge {
            base,
            rated: true,
            keep_alive_stream: false,
            accept_by_token: None,
            message: None,
            rules: "noGiveTime,noRematch".to_string(),
        };

        type Request = lichess_api::model::challenges::create::PostRequest;
        self.context
            .api
            .create_challenge(Request::new(&username, challenge))
            .await
            .map_err(|e| crate::error::Error::LichessError(e))
    }

    pub async fn accept_challenge(&self, challenge_id: String) -> Result<bool> {
        log::info!("Accepting challenge: id {}", &challenge_id);
        tokio::time::sleep(Duration::from_millis(100)).await;

        type Request = lichess_api::model::challenges::accept::PostRequest;
        self.context
            .api
            .accept_challenge(Request::new(challenge_id))
            .await
            .map_err(|e| crate::error::Error::LichessError(e))
    }

    pub async fn cancel_challenge(&self, challenge_id: String) -> Result<bool> {
        log::info!("Canceling challenge: id {}", &challenge_id);
        tokio::time::sleep(Duration::from_millis(100)).await;

        type Request = lichess_api::model::challenges::cancel::PostRequest;
        self.context
            .api
            .cancel_challenge(Request::new(challenge_id, None))
            .await
            .map_err(|e| crate::error::Error::LichessError(e))
    }

    pub async fn decline_challenge(&self, challenge_id: String, reason: Reason) -> Result<bool> {
        log::info!("Declining challenge: id {}", &challenge_id);
        tokio::time::sleep(Duration::from_millis(100)).await;

        type Request = lichess_api::model::challenges::decline::PostRequest;
        self.context
            .api
            .decline_challenge(Request::new(challenge_id, reason))
            .await
            .map_err(|e| crate::error::Error::LichessError(e))
    }

    pub async fn abort(&self, game_id: &str) -> Result<bool> {
        log::info!("Aborting game {}", &game_id);
        tokio::time::sleep(Duration::from_millis(100)).await;

        type Request = lichess_api::model::bot::abort::PostRequest;
        self.context
            .api
            .bot_abort_game(Request::new(&game_id))
            .await
            .map_err(|e| crate::error::Error::LichessError(e))
    }

    pub async fn make_move(&self, game_id: &str, chess_move: chess::ChessMove) -> Result<bool> {
        log::info!("Making move {}", &game_id);
        tokio::time::sleep(Duration::from_millis(200)).await;

        type Request = lichess_api::model::bot::r#move::PostRequest;
        let chess_move = chess_move.to_string();
        self.context
            .api
            .bot_make_move(Request::new(&game_id, &chess_move, false))
            .await
            .map_err(|e| crate::error::Error::LichessError(e))
    }

    pub async fn offer_draw(&self, game_id: &str) -> Result<bool> {
        log::info!("Offering to draw game {}", &game_id);
        tokio::time::sleep(Duration::from_millis(100)).await;

        type Request = lichess_api::model::bot::draw::PostRequest;
        self.context
            .api
            .bot_draw_game(Request::new(&game_id, true))
            .await
            .map_err(|e| crate::error::Error::LichessError(e))
    }

    pub async fn resign(&self, game_id: &str) -> Result<bool> {
        log::info!("Resigning game {}", &game_id);
        tokio::time::sleep(Duration::from_millis(100)).await;

        type Request = lichess_api::model::bot::resign::PostRequest;
        self.context
            .api
            .bot_resign_game(Request::new(&game_id))
            .await
            .map_err(|e| crate::error::Error::LichessError(e))
    }
}

pub enum Action {
    Account(AccountAction),
    Game { game_id: String, action: GameAction },
}

impl Action {
    pub fn accept_challenge(challenge_id: String) -> Self {
        let accept = AccountAction::AcceptChallenge { challenge_id };
        Self::Account(accept)
    }

    pub fn cancel_challenge(challenge_id: String) -> Self {
        let cancel = AccountAction::CancelChallenge { challenge_id };
        Self::Account(cancel)
    }

    pub fn decline_challenge(challenge_id: String, reason: Reason) -> Self {
        let decline = AccountAction::DeclineChallenge { challenge_id, reason };
        Self::Account(decline)
    }

    pub fn challenge_random_bot() -> Self {
        Self::Account(AccountAction::ChallengeRandomBot)
    }

    pub fn abort(game_id: String) -> Self {
        Self::Game { game_id, action: GameAction::Abort }
    }

    pub fn make_move(game_id: String) -> Self {
        Self::Game { game_id, action: GameAction::Move }
    }

    pub fn offer_draw(game_id: String) -> Self {
        Self::Game { game_id, action: GameAction::Move }
    }

    pub fn resign(game_id: String) -> Self {
        Self::Game { game_id, action: GameAction::Move }
    }
}

pub enum AccountAction {
    AcceptChallenge { challenge_id: String },
    CancelChallenge { challenge_id: String },
    DeclineChallenge { challenge_id: String, reason: Reason },
    ChallengeRandomBot,
}

pub enum GameAction {
    Abort,
    Move,
    OfferDraw,
    Resign,
}
