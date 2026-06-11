use regex::Regex;
use std::{error::Error, fmt, sync::LazyLock};

static REMOTE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"https://([A-Za-z0-9-_]+).com/([A-Za-z0-9-_\d]+)/([A-Za-z0-9-_\d]+)(?:.git)?")
        .unwrap()
});

#[derive(Debug, PartialEq, Eq)]
pub struct Remote {
    pub base_url: String,
    pub name: String,
    pub owner: String,
    pub repository: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RemoteParseError {
    InvalidFormat,
    NotImplemented,
}

impl Remote {
    pub fn from_url_str(s: &str, name: &str) -> Result<Self, RemoteParseError> {
        let caps = REMOTE_RE
            .captures(s)
            .ok_or(RemoteParseError::InvalidFormat)?;
        if &caps[1] != "github" {
            return Err(RemoteParseError::NotImplemented);
        }
        Ok(Self {
            base_url: format!("https://{}.com", &caps[1]), // TODO: Bad
            name: name.to_owned(),
            owner: caps[2].to_owned(),
            repository: caps[3].to_owned(),
        })
    }
}

impl fmt::Display for RemoteParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat => write!(f, "Invalid remote format"),
            Self::NotImplemented => write!(f, "Sorry, this remote is not supported"),
        }
    }
}

impl Error for RemoteParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_string_format() {
        let remote = Remote::from_url_str("https://github.com/foo/bar.git", "origin").unwrap();
        assert_eq!(remote.owner, "foo");
        assert_eq!(remote.repository, "bar");
    }

    #[test]
    fn errors_on_invalid_string_format() {
        let result = Remote::from_url_str("https://some-url.com", "origin");
        assert_eq!(result, Err(RemoteParseError::InvalidFormat));
    }

    #[test]
    fn does_not_support_gitlab() {
        let result = Remote::from_url_str("https://gitlab.com/foo/bar.git", "origin");
        assert_eq!(result, Err(RemoteParseError::NotImplemented));
    }
}
