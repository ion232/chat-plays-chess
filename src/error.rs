#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("lichess error: {0}")]
    LichessError(#[from] lichess_api::error::Error),

    #[error("receive error: {0}")]
    ReceiveError(#[from] crossbeam_channel::RecvError),

    #[error("external event send error: {0}")]
    ExternalEventSendError(
        #[from] crossbeam_channel::SendError<crate::engine::events::external::Event>,
    ),

    #[error("send error: {0}")]
    InternalEventSendError(
        #[from] crossbeam_channel::SendError<crate::engine::events::internal::Event>,
    ),

    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("unknown error: {0}")]
    Unknown(String),
}

pub type Result<T> = std::result::Result<T, Error>;
