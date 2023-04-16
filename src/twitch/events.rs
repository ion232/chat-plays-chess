use std::str::FromStr;

use crossbeam_channel::Sender;
use tokio::task::JoinHandle;

use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::message::ServerMessage;
use twitch_irc::TwitchIRCClient;
use twitch_irc::{ClientConfig, SecureTCPTransport};

use crate::error::Result;
use crate::twitch::command::Command;
use crate::twitch::Context;

pub struct EventManager {
    pub(crate) context: Context,
    twitch_irc_handle: Option<JoinHandle<()>>,
}

#[derive(Debug)]
pub enum Event {
    ChatCommand(ChatCommand),
    ChatMessage(ChatMessage),
}

#[derive(Debug)]
pub struct ChatCommand {
    pub user: String,
    pub command: Command,
}

#[derive(Debug)]
pub struct ChatMessage {
    pub user: String,
    pub message: String,
}

impl EventManager {
    pub fn new(context: Context) -> Self {
        Self { context, twitch_irc_handle: Default::default() }
    }

    pub async fn stream_twitch_irc_events(
        &self,
        sender: Sender<Result<Event>>,
    ) -> Result<JoinHandle<()>> {
        let channel = self.context.channel_name.to_string();
        let sender = sender.clone();

        let handle = tokio::spawn(async move {
            let config = ClientConfig::default();
            let (mut incoming_messages, client) =
                TwitchIRCClient::<SecureTCPTransport, StaticLoginCredentials>::new(config);

            client.join(channel).unwrap();

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

                        sender.send(Ok(twitch_event)).unwrap_or_default()
                    }
                    _ => {},
                }
            }
            log::warn!("Twitch IRC stream task finished!")
        });

        Ok(handle)
    }

    pub async fn shutdown(self) {
        if let Some(handle) = self.twitch_irc_handle {
            _ = handle.await;
        }
    }
}
