use std::{
    collections::HashMap,
    fmt::Display,
    io,
    path::{Path, PathBuf},
    process::Output,
};

use config::{Config, ConfigError, File, FileFormat};
use serde::{Deserialize, Serialize};
use std::process::Command;
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
/// - File paths are prefixed with `~` or `/`
/// - Application names have no prefix
/// - URLs can be prefixed with `http:`, `https:`
//    , or no prefix at all (example.com)
/// - Shell commands are prefixed with `sh:`
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Entry {
    /// An application name to be opened.
    ///
    /// - On macOS, you can just specify the name of the application (e.g., Safari)
    App(String),
    /// A file path to be opened.
    /// Must be prefixed with `~` or `/`.
    ///
    /// # Examples
    /// - `/Users/username/Documents` (absolute path)
    /// - `~/Documents` (home directory)
    ///
    /// If the path is a directory, it will be opened in the default file manager.
    /// If the path is a file, it will be opened with the default application for that file type.
    File(PathBuf),
    /// A URL to be opened.
    ///
    /// - This can be any valid URL, such as `http://example.com`,
    ///   `https://example.com`, or `ftp://example.com`...
    /// - This also supports URIs, allowing you to execute
    ///   specific actions within apps (e.g., `spotify://`, `raycast://`).
    Url(Url),
    /// A shell command to be executed.
    /// Must be prefixed with `sh:`.
    ///
    /// The command will be executed using the `sh` shell.
    ///
    /// **USE WITH CAUTION, AS THIS CAN EXECUTE ANY COMMAND ON YOUR SYSTEM.**
    #[cfg(feature = "shell")]
    Shell(String),
}

impl Entry {
    #[cfg(target_os = "macos")]
    /// Open the entry using the appropriate method based on its type.
    pub fn open(&self) -> io::Result<Output> {
        let output = match self {
            Self::App(app) => Command::new("open").arg("-a").arg(app).output()?,
            Self::File(path_buf) => Command::new("open").arg(path_buf).output()?,
            Self::Url(url) => Command::new("open").arg(url.as_str()).output()?,
            #[cfg(feature = "shell")]
            Self::Shell(cmd) => Command::new("sh").arg("-c").arg(cmd).output()?,
        };
        Ok(output)
    }
}

impl<'de> Deserialize<'de> for Entry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match String::deserialize(deserializer)? {
            #[cfg(feature = "shell")]
            s if s.starts_with("sh:") => Ok(Self::Shell(s[3..].to_string())),
            s if s.starts_with(['/', '~']) => {
                Ok(Self::File(shellexpand::tilde(&s).into_owned().into()))
            }
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
            #[cfg(feature = "shell")]
            Self::Shell(cmd) => write!(f, "{}", cmd),
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
