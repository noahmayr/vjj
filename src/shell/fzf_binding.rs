use std::{fmt::Display, str::FromStr};

use ron::error::SpannedError;
use serde::{Deserialize, Serialize};

use crate::{
    common::Selection,
    fzf::FzfAction,
    shell::{
        keymap::{handle_key_event, Bindable, KeyHandlerContext},
        ShellContext,
    },
};

use super::keymap::Key;

pub fn fzf_handler(handler: FzfBindHandler, ctx: ShellContext) -> Option<Vec<FzfAction>> {
    match handler {
        FzfBindHandler::Focus {
            change,
            commit,
            action,
        } => match change.trim_matches('\'').is_empty() || commit.trim_matches('\'').is_empty() {
            true => Some(vec![match action.as_str() {
                "up" => FzfAction::Up,
                _ => FzfAction::Down,
            }]),
            false => None,
        },
        FzfBindHandler::Input { kind, selection } => {
            let mut actions = handle_key_event(KeyHandlerContext::new(
                match kind {
                    InputKind::Change => Bindable::new(ctx.query.clone()),
                    InputKind::Enter => Bindable::Key(Key::Enter),
                    InputKind::Esc => Bindable::Key(Key::Esc),
                },
                selection,
                ctx.clone(),
            ))?;
            actions.push(FzfAction::ClearQuery);
            Some(actions)
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum FzfBindHandler {
    Focus {
        change: String,
        commit: String,
        action: String,
    },
    Input {
        kind: InputKind,
        selection: Selection,
    },
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum InputKind {
    Change,
    Enter,
    Esc,
}

impl FromStr for FzfBindHandler {
    type Err = SpannedError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ron::from_str(s)
    }
}

impl Display for FzfBindHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&ron::ser::to_string(self).unwrap())?;
        Ok(())
    }
}
