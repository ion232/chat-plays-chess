pub mod action;
pub mod challenge;
pub mod events;
pub mod game;
pub mod manager;

use lichess_api::client::LichessApi;

#[derive(Clone)]
pub struct Context {
    pub bot_id: &'static str,
    pub api: LichessApi<reqwest::Client>,
}
