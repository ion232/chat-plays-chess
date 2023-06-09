use std::str::FromStr;

use lazy_static::lazy_static;
use regex::Regex;

#[derive(Clone, Debug)]
pub enum Command {
    VoteGame { action: String },
    VoteSetting { setting: Setting, on: bool },
}

impl ToString for Command {
    fn to_string(&self) -> String {
        match self {
            Command::VoteGame { action } => {
                format!("{}", &action)
            }
            Command::VoteSetting { setting, on } => {
                let on = if *on { "on" } else { "off" };
                format!("{} {}", setting.to_string(), on)
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum Setting {
    GameMode(GameMode),
}

impl ToString for Setting {
    fn to_string(&self) -> String {
        match self {
            Setting::GameMode(game_mode) => {
                format!("{}", game_mode.to_string())
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum GameMode {
    Bullet,
    Rapid,
    Classical,
}

impl ToString for GameMode {
    fn to_string(&self) -> String {
        match self {
            Self::Bullet => "bullet",
            Self::Rapid => "rapid",
            Self::Classical => "classical",
        }
        .to_string()
    }
}

impl FromStr for Command {
    type Err = crate::error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref COMMAND_REGEX: Regex =
                Regex::new(r"!(game|bullet|rapid|classical)\s+(\w+)").unwrap();
        }

        let Some(captures) = COMMAND_REGEX.captures(s) else {
            return Err(crate::error::Error::RegexError);
        };

        // Capture group 0 is the whole string.
        if captures.len() != 3 {
            return Err(crate::error::Error::RegexError);
        }

        let command = captures.get(1).unwrap().as_str();

        let arg1 = captures.get(2).unwrap().as_str().to_string();
        let on = match arg1.as_str() {
            "on" => true,
            "off" => false,
            _ => false,
        };

        return match command {
            "game" => Ok(Command::VoteGame { action: arg1 }),
            "bullet" => {
                Ok(Command::VoteSetting { setting: Setting::GameMode(GameMode::Bullet), on })
            }
            "rapid" => Ok(Command::VoteSetting { setting: Setting::GameMode(GameMode::Rapid), on }),
            "classical" => {
                Ok(Command::VoteSetting { setting: Setting::GameMode(GameMode::Classical), on })
            }
            _ => Err(crate::error::Error::RegexError),
        };
    }
}
