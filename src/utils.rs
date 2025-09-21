use std::{
    env,
    io::Write as _,
    path::Path,
    process::{Command, Stdio},
    vec,
};

use crate::{cli::ParcelCommands, config::ParcelConfig};

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
pub fn choose(config_path: &Path, multi: bool) -> anyhow::Result<()> {
    use dialoguer::{FuzzySelect, MultiSelect, theme::ColorfulTheme};

    let config = ParcelConfig::load(config_path)?;
    let parcels = config.parcels.keys().collect::<Vec<_>>();
    if parcels.is_empty() {
        eprintln!("No parcels available. Please add parcels to the configuration file.");
        return Ok(());
    }

    if multi {
        let selection = MultiSelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Select a parcel to open")
            .items(&parcels)
            .interact_opt()?;

        if let Some(indices) = selection {
            for name in indices.iter().map(|&i| parcels[i].to_string()) {
                ParcelCommands::Open { name }.run(config_path.as_ref())?;
            }
        } else {
            println!("No parcels selected.");
        }
    } else {
        let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
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
    };

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
        "--ansi",
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
            "sh -c '{} --config {} list \"$1\" | bat --color=always -pp' sh {}",
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
