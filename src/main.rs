mod engine;
mod lichess;
mod stream;
mod twitch;

mod error;
mod logging;

use std::sync::Arc;

use engine::Engine;
use lichess::Context as LichessContext;
use stream::model::Model;
use tokio::sync::Mutex;
use twitch::Context as TwitchContext;

use stream::manager::Manager;
use stream::window::Window;

pub fn make_lichess_context() -> LichessContext {
    let client = reqwest::Client::builder().build().unwrap();
    let auth_token = "lip_bK42iXBZkPLjfXRzKY99".to_string().into();

    let bot_id = "twitch-bot-blue";
    let api = lichess_api::client::LichessApi::new(client, auth_token);

    LichessContext { bot_id, api }
}

pub fn make_twitch_context() -> TwitchContext {
    let channel_name = "TTVPlaysChess";
    let helix_auth = "".to_string();

    TwitchContext { channel_name, helix_auth }
}

pub async fn window() {
    let mut stream_manager = Manager::new();
    let mut window = Window::new();

    stream_manager.setup();

    let lichess_context = make_lichess_context();
    let twitch_context = make_twitch_context();
    let mut engine = Engine::new(lichess_context, twitch_context);

    engine.setup().await.unwrap();

    let mut now = std::time::SystemTime::now();
    loop {
        if engine.process().await.is_err() {
            log::error!("Error in engine process");
        }

        if let Some(elapsed) = now.elapsed().ok() {
            if elapsed.as_millis() >= 33 {
                stream_manager.write_frame(&engine.model, &mut window);
                now = std::time::SystemTime::now();
            }
        }
    }
}

#[tokio::main]
pub async fn main() {
    env_logger::init();

    log::info!("Starting up ChatPlaysChess!");
    window().await;
}
