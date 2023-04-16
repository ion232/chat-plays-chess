use std::{fs::File, io::Read};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

pub fn load_config() -> Result<Config> {
    let args: Vec<String> = std::env::args().collect();
    let args_length = args.len();

    if args_length != 2 {
        let message = format!("Invalid arguments length {}.", args_length);
        return Err(Error::Unknown(message));
    }

    let Some(config_path) = args.get(1) else {
        return Err(Error::Unknown("Failed to get config path".to_string()));
    };

    let mut config_file = File::open(config_path)?;

    let mut contents = String::new();
    config_file.read_to_string(&mut contents)?;

    let config: Config = serde_json::from_str(&contents)?;

    Ok(config)
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Config {
    pub lichess: Lichess,
    pub twitch: Twitch,
    pub livestream: Livestream,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Lichess {
    pub account: String,
    pub access_token: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Twitch {
    pub channel: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Livestream {
    pub video: Video,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Video {
    pub fifo: String,
}
