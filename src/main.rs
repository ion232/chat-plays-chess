use chat_plays_chess::engine;

use chat_plays_chess::lichess;
use chat_plays_chess::stream;
use chat_plays_chess::twitch;

use chat_plays_chess::config;
use chat_plays_chess::error;

use std::path::PathBuf;
use std::thread::JoinHandle;

use config::Config;

use engine::events::stream::{EventReceiver, EventSender};
use engine::Engine;

use error::Result;

use lichess::Context as LichessContext;
use twitch::Context as TwitchContext;

use stream::manager::Manager;

pub fn main() -> Result<()> {
    init_logger();

    let config = config::load_config()?;
    run(config)
}

pub fn init_logger() {
    env_logger::init();
    log::info!("Starting up ChatPlaysChess!");
}

pub fn run(config: Config) -> crate::error::Result<()> {
    let (sender, receiver) = crossbeam_channel::unbounded();

    let stream_manager = run_stream_manager(receiver, config.livestream.clone());
    run_engine(sender, config)?;

    let stream_manager = stream_manager.join().expect("Failed to join stream manager handle");
    // let engine = engine.join().expect("Failed to join engine handle");

    stream_manager?;

    Ok(())
}

pub fn run_engine(stream_events: EventSender, config: Config) -> Result<()> {
    // std::thread::spawn(move || {
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build();
    let Ok(runtime) = runtime else {
            return Err(error::Error::Unknown("tokio runtime failed to build".to_string()));
        };

    runtime.block_on(async move {
        let mut engine = make_engine(stream_events, config);
        engine.setup().await?;
        engine.run().await
    })
    // })
}

pub fn run_stream_manager(
    stream_events: EventReceiver,
    config: config::Livestream,
) -> JoinHandle<Result<()>> {
    std::thread::spawn(move || {
        let video_fifo = PathBuf::from(config.video.fifo);
        let mut stream_manager = Manager::new(stream_events, video_fifo)?;
        stream_manager.setup();
        stream_manager.run();

        Ok(())
    })
}

pub fn make_engine(stream_events: EventSender, config: Config) -> Engine {
    let lichess_context = make_lichess_context(&config.lichess);
    let twitch_context = make_twitch_context(&config.twitch);
    Engine::new(stream_events, lichess_context, twitch_context)
}

pub fn make_lichess_context(config: &config::Lichess) -> LichessContext {
    let our_id = config.account.to_string();

    let client = reqwest::Client::builder().build().unwrap();
    let access_token = config.access_token.to_string().into();
    let api = lichess_api::client::LichessApi::new(client, access_token);

    LichessContext { our_id, api }
}

pub fn make_twitch_context(config: &config::Twitch) -> TwitchContext {
    TwitchContext { channel_name: config.channel.to_string() }
}
