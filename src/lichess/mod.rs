pub mod action;
pub mod challenge;
pub mod events;
pub mod game;

use lichess_api::client::LichessApi;

#[derive(Clone)]
pub struct Context {
    pub our_id: String,
    pub api: LichessApi<reqwest::Client>,
}
