use thiserror::Error;

/// Errors produced by `fge-audio` WAV decoding, audio device setup, or command channel operations.
#[derive(Debug, Error)]
pub enum AudioError {
    #[error("invalid WAV data: {reason}")]
    InvalidWav { reason: String },

    #[error("audio I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("no audio output device available")]
    DeviceUnavailable,

    #[error("failed to build audio output stream: {reason}")]
    StreamBuild { reason: String },

    #[error("audio mixer command channel disconnected")]
    ChannelDisconnected,
}
