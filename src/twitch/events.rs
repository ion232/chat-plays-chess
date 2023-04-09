use std::str::FromStr;

use tokio::io::{split, AsyncBufReadExt, BufReader};
use tokio::task::JoinHandle;

use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::message::ServerMessage;
use twitch_irc::TwitchIRCClient;
use twitch_irc::{ClientConfig, SecureTCPTransport};

use crate::error::Result;
use crate::twitch::command::Command;

pub enum Event {
    ChatCommand(ChatCommand),
    ChatMessage(ChatMessage),
    BitsDonation(BitsDonation),
}

pub struct ChatCommand {
    pub user: String,
    pub command: Command,
}

pub struct ChatMessage {
    pub user: String,
    pub message: String,
}

pub struct BitsDonation {
    pub bits: u64,
    pub message: String,
}

impl crate::engine::events::EventSubscriber {
    pub async fn stream_twitch_irc_events(&self) -> Result<JoinHandle<()>> {
        self.stream_artifical_twitch_events().await
    }

    async fn stream_artifical_twitch_events(&self) -> Result<JoinHandle<()>> {
        let sender = self.sender.clone();
        let handle = tokio::spawn(async move {
            let stdin = tokio::io::stdin();
            let mut reader = BufReader::new(stdin);
            let mut line = "".to_string();
            while let Ok(_) = reader.read_line(&mut line).await {
                let (user, message) = line.split_once(":").unwrap();
                let twitch_event = if let Ok(command) = Command::from_str(&message) {
                    Event::ChatCommand(ChatCommand { user: user.to_string(), command })
                } else {
                    Event::ChatMessage(ChatMessage { user: user.to_string(), message: message.to_string() })
                };
                let twitch_event = crate::engine::events::Event::TwitchEvent(twitch_event);
                sender.send(Ok(twitch_event)).unwrap_or_default()
            }
        });

        Ok(handle)
    }

    async fn stream_real_twitch_events(&self) -> Result<JoinHandle<()>> {
        let config = ClientConfig::default();
        let (mut incoming_messages, client) =
            TwitchIRCClient::<SecureTCPTransport, StaticLoginCredentials>::new(config);
        let sender = self.sender.clone();

        let handle = tokio::spawn(async move {
            while let Some(message) = incoming_messages.recv().await {
                match message {
                    ServerMessage::Privmsg(private_message) => {
                        let user = private_message.sender.name;
                        let message = private_message.message_text;
                        let twitch_event = if let Ok(command) = Command::from_str(&message) {
                            Event::ChatCommand(ChatCommand { user, command })
                        } else {
                            Event::ChatMessage(ChatMessage { user, message })
                        };
                        let twitch_event = crate::engine::events::Event::TwitchEvent(twitch_event);
                        sender.send(Ok(twitch_event)).unwrap_or_default()
                    }
                    _ => {}
                }
            }
        });

        client.join("TTVPlaysChess".to_owned()).unwrap();
        Ok(handle)
    }
}
