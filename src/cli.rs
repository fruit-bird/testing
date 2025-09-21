use std::{io, path::Path};

use anyhow::Result;
use clap::{CommandFactory as _, Parser, Subcommand, ValueEnum};

use crate::config::{Entry, ParcelConfig};
use crate::utils;

/// A tool to open groups of applications, files, folders, and URLs
#[derive(Debug, Parser)]
#[clap(version, author, about)]
pub struct ParcelCLI {
    #[clap(subcommand)]
    command: ParcelCommands,
    /// Override the default config path
    #[clap(short, long, default_value_t = utils::default_config())]
    config: String,
}

#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum Chooser {
    #[default]
    Fzf,
    #[cfg(feature = "dialog")]
    Dialoguer,
}

impl ParcelCLI {
    pub fn run(&self) -> anyhow::Result<()> {
        self.command.run(Path::new(&self.config))
    }
}

#[derive(Debug, Subcommand)]
pub enum ParcelCommands {
    /// Opens a parcel by name
    Open { name: String },
    /// Opens a parcel by choosing from a list
    Choose {
        /// Choose a parcel to open using a fuzzy finder
        #[clap(long, value_enum, default_value_t)]
        chooser: Chooser,
        /// Allow multiple selections (only with fzf)
        #[clap(long, default_value_t = false)]
        multi: bool,
    },
    /// Lists all available parcels
    List {
        /// Name of the parcel to list items for
        name: Option<String>,
        /// Output in JSON format, useful for scripting
        #[cfg(feature = "json")]
        #[clap(long, default_value = "false")]
        json: bool,
    },
    /// Generate shell completions
    Completions {
        /// The shell to generate the completions for
        #[clap(value_enum)]
        shell: clap_complete::Shell,
    },
}

impl ParcelCommands {
    pub fn run(&self, config_path: &Path) -> anyhow::Result<()> {
        let config = ParcelConfig::load(config_path)?;
        match self {
            Self::Open { name } => Self::open(&config, name)?,
            Self::Choose { chooser, multi } => match chooser {
                Chooser::Fzf => utils::choose_fzf(config_path, *multi)?,
                #[cfg(feature = "dialog")]
                Chooser::Dialoguer => utils::choose(config_path, *multi)?,
            },

            #[cfg(feature = "json")]
            Self::List { json, .. } if *json => println!("{}", serde_json::to_string(&config)?),
            Self::List { name: Some(n), .. } => Self::list_parcel(&config, n)?,
            Self::List { .. } => println!("{}", config),

            Self::Completions { shell } => {
                let mut cmd = ParcelCLI::command();
                let name = cmd.get_name().to_string();
                clap_complete::generate(*shell, &mut cmd, name, &mut io::stdout());
            }
        }
        Ok(())
    }

    fn open(config: &ParcelConfig, name: &str) -> anyhow::Result<()> {
        config
            .parcels
            .get(name)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Parcel `{}` not found. Available parcels: {}",
                    name,
                    utils::available_parcels(config)
                )
            })?
            .iter()
            .map(Entry::open)
            .filter_map(Result::ok)
            .for_each(|_| { /* Successfully opened an entry */ });

        Ok(())
    }

    fn list_parcel(config: &ParcelConfig, name: &str) -> anyhow::Result<()> {
        if let Some(entries) = config.parcels.get(name) {
            for entry in entries {
                println!("- {}", entry);
            }
            Ok(())
        } else {
            anyhow::bail!(
                "Parcel `{}` not found. Available parcels: {}",
                name,
                utils::available_parcels(config)
            );
        }
    }
}
