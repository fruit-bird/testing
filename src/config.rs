use std::{
    collections::HashMap,
    fmt::Display,
    io,
    path::{Path, PathBuf},
};

use config::{Config, ConfigError, File, FileFormat};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Serialize, Deserialize)]
pub struct ParcelConfig {
    #[serde(flatten)]
    pub parcels: HashMap<String, Vec<Entry>>,
}

impl ParcelConfig {
    pub fn load(config_path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let conf = Config::builder()
            .add_source(
                File::with_name(&config_path.as_ref().to_string_lossy()).format(FileFormat::Yaml),
            )
            .build()?
            .try_deserialize()?;

        Ok(conf)
    }
}

/// Representation of the type of the entry in each parcel.
/// Could be the name of an application, a file path, a URL, or a shell command.
///
/// - File paths are prefixed with `fs:`
/// - Application names have no prefix,
///   and are opened with the `open` command on macOS, `xdg-open` on Linux, and `start` on Windows.
/// - URLs are automatically detected by the `open` command,
///   and can be prefixed with `http:`, `https:`
//    , or no prefix at all (example.com)
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Entry {
    App(String),
    File(PathBuf),
    Url(Url),
}

impl Entry {
    /// Open the entry using the appropriate method based on its type.
    pub fn open(&self) -> io::Result<()> {
        match self {
            Self::App(app) => open::that_detached(app),
            Self::File(path_buf) => open::that_detached(path_buf),
            Self::Url(url) => open::that_detached(url.as_str()),
        }
    }
}

impl<'de> Deserialize<'de> for Entry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match String::deserialize(deserializer)? {
            s if s.starts_with("fs:") => Ok(Self::File(PathBuf::from(
                shellexpand::tilde(&s[3..]).into_owned(),
            ))),
            s if let Ok(url) = Url::parse(&s) => Ok(Self::Url(url)),
            s => Ok(Self::App(s)),
        }
    }
}

impl Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::App(name) => write!(f, "{}", name),
            Self::File(path) => write!(f, "{}", path.to_string_lossy()),
            Self::Url(url) => write!(f, "{}", url),
        }
    }
}

impl Display for ParcelConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (name, entries) in &self.parcels {
            writeln!(f, "{}:", name)?;
            for entry in entries {
                writeln!(f, "- {}", entry)?;
            }
        }
        Ok(())
    }
}
