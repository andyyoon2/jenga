use regex::Regex;
use std::{str::FromStr, sync::LazyLock};

static REMOTE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"https://([A-Za-z0-9-_]+).com/([A-Za-z0-9-_\d]+)/([A-Za-z0-9-_\d]+)(?:.git)?")
        .unwrap()
});

#[derive(Debug, PartialEq, Eq)]
pub struct Remote {
    pub owner: String,
    pub repository: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RemoteParseError {
    InvalidFormat,
    NotImplemented,
}

impl FromStr for Remote {
    type Err = RemoteParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let caps = REMOTE_RE
            .captures(s)
            .ok_or(RemoteParseError::InvalidFormat)?;
        if &caps[1] != "github" {
            return Err(RemoteParseError::NotImplemented);
        }
        Ok(Self {
            owner: caps[2].to_owned(),
            repository: caps[3].to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_string_format() {
        let remote = Remote::from_str("https://github.com/foo/bar.git").unwrap();
        assert_eq!(remote.owner, "foo");
        assert_eq!(remote.repository, "bar");
    }

    #[test]
    fn errors_on_invalid_string_format() {
        let result = Remote::from_str("https://some-url.com");
        assert_eq!(result, Err(RemoteParseError::InvalidFormat));
    }

    #[test]
    fn does_not_support_gitlab() {
        let result = Remote::from_str("https://gitlab.com/foo/bar.git");
        assert_eq!(result, Err(RemoteParseError::NotImplemented));
    }
}
