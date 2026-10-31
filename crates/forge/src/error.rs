#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("the token is missing, expired or rejected")]
    Unauthorized,
    #[error("the token is not allowed to do this")]
    Forbidden,
    #[error("not found (wrong repository, or CI is not enabled for it)")]
    NotFound,
    #[error("rate limited{}", .0.map(|s| format!(", retry in {s}s")).unwrap_or_default())]
    RateLimited(Option<u64>),
    #[error("{0} is not supported by this platform")]
    Unsupported(&'static str),
    #[error("HTTP {status}: {message}")]
    Http { status: u16, message: String },
    #[error("network error: {0}")]
    Network(String),
    #[error("unexpected response: {0}")]
    Parse(String),
}

pub type Result<T> = std::result::Result<T, Error>;
