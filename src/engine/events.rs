use std::collections::HashMap;

use crossbeam_channel::{Receiver, Sender};
use tokio::task::JoinHandle;

use crate::error::Result;

use crate::lichess::events::Event as LichessEvent;
use crate::lichess::Context as LichessContext;
use crate::twitch::events::Event as TwitchEvent;
use crate::twitch::Context as TwitchContext;

pub struct EventSubscriber {
    pub(crate) receiver: Receiver<Result<Event>>,
    pub(crate) sender: Sender<Result<Event>>,
    pub(crate) lichess_context: LichessContext,
    pub(crate) twitch_context: TwitchContext,
    lichess_account_handle: Option<JoinHandle<()>>,
    lichess_game_handles: HashMap<String, JoinHandle<()>>,
    twitch_irc_handle: Option<JoinHandle<()>>,
}

pub enum Event {
    LichessEvent(LichessEvent),
    TwitchEvent(TwitchEvent),
}

impl EventSubscriber {
    pub fn new(lichess_context: LichessContext, twitch_context: TwitchContext) -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        Self {
            receiver,
            sender,
            lichess_account_handle: Default::default(),
            lichess_game_handles: Default::default(),
            twitch_irc_handle: Default::default(),
            lichess_context,
            twitch_context,
        }
    }

    pub async fn subscribe_to_all(&mut self) -> Result<()> {
        self.lichess_account_handle = self.stream_lichess_account_events().await?.into();
        // self.twitch_irc_handle = self.stream_twitch_irc_events().await?.into();

        Ok(())
    }

    pub async fn handle_game_start(&mut self, game_id: &str) -> Result<()> {
        if !self.lichess_game_handles.contains_key(game_id) {
            log::info!("Streaming game {} ...", &game_id);
            let handle = self.stream_lichess_game_events(game_id).await?;
            self.lichess_game_handles.insert(game_id.to_string(), handle);
            log::info!("Game {} is now being streamed!", &game_id);
        }

        Ok(())
    }

    pub async fn handle_game_finished(&mut self, game_id: &str) {
        log::info!("Waiting for game {} to finish...", &game_id);
        if let Some(handle) = self.lichess_game_handles.get_mut(game_id) {
            _ = handle.await;
        }
        log::info!("Game {} finished!", &game_id);
    }

    pub async fn shutdown(self) {
        if let Some(handle) = self.lichess_account_handle {
            _ = handle.await;
        }
        if let Some(handle) = self.twitch_irc_handle {
            _ = handle.await;
        }
        for (_, h) in self.lichess_game_handles.into_iter() {
            _ = h.await;
        }
    }

    pub async fn next_event(&self) -> Result<Event> {
        self.receiver.recv().map_err(|e| crate::error::Error::ReceiveError(e))?
    }

    pub fn has_event(&self) -> bool {
        !self.receiver.is_empty()
    }
}
