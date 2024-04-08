pub mod command;
pub mod exec;
pub mod fzf_binding;
pub mod keymap;

use std::{fmt::Display, str::FromStr};

use itertools::Itertools;
use ron::error::SpannedError;
use serde::{Deserialize, Serialize};

use crate::{common::Mode, fzf::FzfAction};

use self::{
    command::{vjj_command, VjjCommand},
    fzf_binding::{fzf_handler, FzfBindHandler},
};

pub fn vjj_shell(expression: VjjShellExpression) -> Option<String> {
    let mode: Mode = Mode::from_str(std::env::var("FZF_PROMPT").unwrap_or_default().as_str())
        .unwrap_or_default();

    let query = std::env::var("FZF_QUERY").unwrap_or_default();
    let revset = std::env::var("FZF_BORDER_LABEL")
        .ok()
        .filter(|s| !s.is_empty());

    let ctx = ShellContext {
        mode,
        query,
        revset,
    };
    match expression {
        VjjShellExpression::Handler(handler) => {
            return fzf_handler(handler, ctx).and_then(actions_output)
        }
        VjjShellExpression::Command(command) => match vjj_command(command, ctx, false, false) {
            Ok(_) => (),
            Err(e) => {
                eprintln!("{}", e)
            }
        },
        VjjShellExpression::PagedCommand {
            command,
            interactive,
        } => match vjj_command(command, ctx, true, interactive) {
            Ok(_) => (),
            Err(e) => {
                eprintln!("{}", e)
            }
        },
    };
    None
}

fn actions_output(actions: Vec<FzfAction>) -> Option<String> {
    if actions.is_empty() {
        None
    } else {
        Some(actions.iter().map(ToString::to_string).join("+"))
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum VjjShellExpression {
    Handler(FzfBindHandler),
    Command(VjjCommand),
    PagedCommand {
        command: VjjCommand,
        interactive: bool,
    },
}

impl Display for VjjShellExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&ron::ser::to_string(self).unwrap())?;
        Ok(())
    }
}

impl FromStr for VjjShellExpression {
    type Err = SpannedError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ron::from_str(s)
    }
}

#[derive(Debug, Clone)]
pub struct ShellContext {
    mode: Mode,
    query: String,
    revset: Option<String>,
}
