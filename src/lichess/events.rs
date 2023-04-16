use async_std::stream::StreamExt;
use crossbeam_channel::Sender;
use lichess_api::model::bot;
use std::collections::HashMap;
use tokio::task::JoinHandle;

use crate::error::Result;
use crate::lichess::Context;

#[derive(Debug)]
pub enum Event {
    AccountEvent { event: bot::stream::events::Event },
    GameEvent { game_id: String, event: bot::stream::game::Event },
}

pub struct EventManager {
    context: Context,
    account_handle: Option<JoinHandle<()>>,
    game_handles: HashMap<String, JoinHandle<()>>,
}

impl EventManager {
    pub fn new(context: Context) -> Self {
        Self { context, account_handle: Default::default(), game_handles: Default::default() }
    }

    pub async fn stream_account(&mut self, sender: Sender<Result<Event>>) -> Result<()> {
        if self.account_handle.is_none() {
            self.account_handle = self.stream_account_events(sender).await?.into();
        }
        Ok(())
    }

    pub async fn stream_game(
        &mut self,
        sender: Sender<Result<Event>>,
        game_id: &str,
    ) -> Result<()> {
        if !self.game_handles.contains_key(game_id) {
            log::info!("Streaming game {} ...", &game_id);
            let handle = self.stream_game_events(sender, game_id).await?;
            self.game_handles.insert(game_id.to_string(), handle);
            log::info!("Game {} is now being streamed!", &game_id);
        }

        Ok(())
    }

    pub async fn finish_streaming_game(&mut self, game_id: &str) {
        log::info!("Waiting for game {} to finish...", &game_id);
        if let Some(handle) = self.game_handles.get_mut(game_id) {
            _ = handle.await;
        }
        log::info!("Game {} finished!", &game_id);
    }

    pub async fn shutdown(self) {
        if let Some(handle) = self.account_handle {
            _ = handle.await;
        }
        for (_, h) in self.game_handles.into_iter() {
            _ = h.await;
        }
    }

    async fn stream_account_events(&self, sender: Sender<Result<Event>>) -> Result<JoinHandle<()>> {
        let request = bot::stream::events::GetRequest::new();
        let mut stream = self.context.api.bot_stream_incoming_events(request).await?;

        let sender = sender.clone();

        Ok(tokio::task::spawn(async move {
            while let Some(result) = stream.next().await {
                let result = result
                    .map(|event| Event::AccountEvent { event })
                    .map_err(|e| crate::error::Error::LichessError(e));
                sender.send(result).unwrap_or_default();
            }
        }))
    }

    async fn stream_game_events(
        &self,
        sender: Sender<Result<Event>>,
        game_id: &str,
    ) -> Result<JoinHandle<()>> {
        let request = bot::stream::game::GetRequest::new(game_id);
        let mut stream = self.context.api.bot_stream_board_state(request).await?;

        let sender = sender.clone();
        let game_id = game_id.to_string();

        Ok(tokio::task::spawn(async move {
            while let Some(result) = stream.next().await {
                let result = result
                    .map(|event| Event::GameEvent { game_id: game_id.clone(), event })
                    .map_err(|e| crate::error::Error::LichessError(e));
                sender.send(result).unwrap_or_default();
            }
        }))
    }
}
