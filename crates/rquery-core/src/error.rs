use thiserror::Error;

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("unsupported feature: {feature} — {message}")]
    Unsupported { feature: String, message: String },

    #[error("render error: {0}")]
    Other(String),
}

impl RenderError {
    pub fn unsupported(feature: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Unsupported {
            feature: feature.into(),
            message: message.into(),
        }
    }
}

pub type RenderResult<T> = Result<T, RenderError>;
