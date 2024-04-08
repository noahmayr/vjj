use std::{fmt::Display, io::Write, str::FromStr};

use crate::common::{Mode, Selection, VjjError, VjjResult};
use itertools::Itertools;
use ron::error::SpannedError;
use serde::{Deserialize, Serialize};

use super::{
    exec::{get_pager, jj, Pager},
    keymap::{get_keymap, Keybind},
    ShellContext,
};

pub fn vjj_command(
    command: VjjCommand,
    ctx: ShellContext,
    pager: bool,
    interactive: bool,
) -> VjjResult<()> {
    match command {
        VjjCommand::Show(selection) => {
            if let Some(id) = selection.commit().focused {
                jj(
                    ["--ignore-working-copy", "show", id.as_str()],
                    pager,
                    interactive,
                )?;
            }
        }
        VjjCommand::Log => {
            let log_args = [
                "--ignore-working-copy",
                "--config-toml",
                include_str!("../../jj-config-overrides.toml"),
            ];
            match ctx.mode {
                Mode::Normal | Mode::Revset => match ctx.revset {
                    Some(revset) => jj(
                        log_args.iter().chain(&["log", "-r", revset.as_str()]),
                        pager,
                        interactive,
                    )?,
                    None => jj(log_args.iter().chain(&["log"]), pager, interactive)?,
                },
                Mode::Obslog(rev) => jj(
                    log_args.iter().chain(&["obslog", "-r", rev.as_str()]),
                    pager,
                    interactive,
                )?,
            }
        }
        VjjCommand::Help => {
            let keymap = match get_keymap(&ctx.mode) {
                Ok(keymap) => keymap,
                Err(VjjError::MisingKeymap(_)) => {
                    return vjj_command(
                        VjjCommand::Output("No Commands Available".to_string()),
                        ctx,
                        pager,
                        interactive,
                    )
                }
                Err(e) => {
                    return vjj_command(VjjCommand::Error(e.to_string()), ctx, pager, interactive)
                }
            };

            let max_len = keymap
                .keys()
                .map(|key| key.to_string().len())
                .max()
                .unwrap_or(0);

            vjj_command(
                VjjCommand::Output(format!(
                    "### {} MODE HELP ###\n{}",
                    ctx.mode.keymap().to_uppercase(),
                    keymap
                        .iter()
                        .map(|(key, Keybind(help, _))| format!(
                            "{:width$}  {help}",
                            key.to_string(),
                            width = max_len
                        ))
                        .join("\n")
                )),
                ctx,
                pager,
                interactive,
            )?
        }
        VjjCommand::Output(output) => {
            if pager {
                let Pager {
                    mut child,
                    mut stdin,
                } = get_pager()?;
                writeln!(stdin, "{}", output)?;
                drop(stdin);
                child.wait()?;
            } else {
                println!("{}", output);
            }
        }
        VjjCommand::Jujutsu(args) => {
            jj(args, pager, interactive)?;
        }
        VjjCommand::Error(output) => {
            if pager {
                let Pager {
                    mut child,
                    mut stdin,
                } = get_pager()?;
                writeln!(stdin, "{}", output)?;
                drop(stdin);
                child.wait()?;
            } else {
                eprintln!("{}", output);
            }
        }
    };
    Ok(())
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum VjjCommand {
    Log,
    Help,
    Show(Selection),
    Jujutsu(Vec<String>),
    Output(String),
    Error(String),
}

impl Display for VjjCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&ron::ser::to_string(self).unwrap())?;
        Ok(())
    }
}

impl FromStr for VjjCommand {
    type Err = SpannedError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ron::from_str(s)
    }
}
