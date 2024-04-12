use std::fmt::Display;
use std::str::FromStr;

use ron::error::SpannedError;
use serde::{Deserialize, Serialize};

use super::command::VjjCommand;
use super::keymap::Key;
use crate::common::Selection;
use crate::fzf::FzfAction;
use crate::shell::keymap::{handle_key_event, Bindable, KeyHandlerContext};
use crate::shell::ShellContext;

pub fn fzf_handler(handler: FzfBindHandler, ctx: ShellContext) -> Vec<FzfAction> {
    match handler {
        FzfBindHandler::Focus {
            change,
            commit,
            action,
        } => match change.trim_matches('\'').is_empty() || commit.trim_matches('\'').is_empty() {
            true => vec![match action.as_str() {
                "up" => FzfAction::Up,
                _ => FzfAction::Down,
            }],
            false => vec![
                FzfAction::ChangePreview(VjjCommand::Show(commit)),
                FzfAction::ChangePreviewLabel("Preview (jj show)".to_string()),
            ],
        },
        FzfBindHandler::Input { kind, selection } => {
            match handle_key_event(KeyHandlerContext::new(
                match kind {
                    InputKind::Change => Bindable::new(ctx.query.clone()),
                    InputKind::Enter => Bindable::Key(Key::Enter),
                    InputKind::Esc => Bindable::Key(Key::Esc),
                },
                selection,
                ctx.clone(),
            )) {
                Some(mut actions) => {
                    actions.push(FzfAction::ClearQuery);
                    actions
                }
                None => vec![],
            }
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
