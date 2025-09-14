use std::{
    io::{self, Write as _},
    path::Path,
};

use anyhow::Result;
use clap::{CommandFactory as _, Parser, Subcommand, ValueEnum};

use crate::config::{Entry, ParcelConfig};

/// A tool to open groups of applications, files, and URLs
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
enum Chooser {
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
enum ParcelCommands {
    /// Opens a parcel by name
    Open { name: String },
    /// Opens a parcel by choosing from a list
    Choose {
        /// Choose a parcel to open using a fuzzy finder
        #[clap(long, value_enum, default_value_t = Chooser::default())]
        chooser: Chooser,
        /// Allow multiple selections
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
                Chooser::Dialoguer => utils::choose(config_path)?,
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

mod utils {
    use std::{
        env,
        process::{Command, Stdio},
        vec,
    };

    use super::*;

    pub fn default_config() -> String {
        let base = shellexpand::tilde("~/.config/kozutsumi/parcel");
        let yml = format!("{}.yml", base);
        let yaml = format!("{}.yaml", base);

        if Path::new(&yml).exists() { yml } else { yaml }
    }

    pub fn available_parcels(config: &ParcelConfig) -> String {
        config
            .parcels
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    }

    #[cfg(feature = "dialog")]
    pub fn choose(config_path: &Path) -> anyhow::Result<()> {
        let config = ParcelConfig::load(config_path)?;
        let parcels = config.parcels.keys().collect::<Vec<_>>();
        if parcels.is_empty() {
            eprintln!("No parcels available. Please add parcels to the configuration file.");
            return Ok(());
        }
        let selection =
            dialoguer::FuzzySelect::with_theme(&dialoguer::theme::ColorfulTheme::default())
                .with_prompt("Select a parcel to open")
                .items(&parcels)
                .default(0)
                .interact_opt()?;

        if let Some(index) = selection {
            let name = parcels[index].to_string();
            ParcelCommands::Open { name }.run(config_path.as_ref())?;
        } else {
            println!("No parcel selected.");
        }
        Ok(())
    }

    pub fn choose_fzf(config_path: &Path, multi: bool) -> anyhow::Result<()> {
        let current_exe = env::current_exe()?;
        let config = ParcelConfig::load(config_path)?;
        let parcels = config.parcels.keys().collect::<Vec<_>>();
        if parcels.is_empty() {
            eprintln!("No parcels available. Please add parcels to the configuration file.");
            return Ok(());
        }

        let mut args = vec![
            "--preview-window=right:60%:wrap",
            "--layout=reverse",
            "--bind=tab:down,shift-tab:up",
            "--cycle",
            "--no-sort",
            "--tmux=center,70%,40%",
        ];
        if multi {
            args.extend([
                "--multi",
                "--bind=ctrl-a:select-all",
                "--bind=space:toggle+down",
            ]);
        }
        let fzf = Command::new("fzf")
            .args(args)
            .arg("--preview")
            .arg(format!(
                "sh -c '{} --config {} list \"$1\"' sh {}",
                current_exe.to_string_lossy(),
                config_path.as_os_str().to_string_lossy(),
                "{}"
            ))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let mut stdin = fzf.stdin.as_ref().unwrap();
        for parcel in &parcels {
            writeln!(stdin, "{}", parcel)?;
        }

        let output = fzf.wait_with_output()?;
        if output.status.success() {
            let selection = String::from_utf8_lossy(&output.stdout);
            let name = selection.trim().to_string();
            if !name.is_empty() {
                ParcelCommands::Open { name }.run(config_path.as_ref())?;
            } else {
                eprintln!("No parcel selected.");
            }
            Ok(())
        } else {
            match output.status.code() {
                Some(130) | Some(1) => {
                    // 130: User cancelled (Ctrl-C)
                    //   1: No match found
                    eprintln!("No parcel selected.");
                    Ok(())
                }
                _ => anyhow::bail!("fzf failed with status: {}", output.status),
            }
        }
    }
}
