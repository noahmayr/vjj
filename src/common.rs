use std::{fmt::Display, io, str::FromStr};

use ctty::CttyError;
use ron::error::SpannedError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::shell::keymap::TemplateError;

#[derive(Error, Debug)]
pub enum VjjError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("ron error: {0}")]
    Ron(#[from] SpannedError),
    #[error("tty error: {0}")]
    Tty(#[from] CttyError),
    #[error("template error: {0}")]
    Template(#[from] TemplateError),
    #[error("boxed error: {0}")]
    Boxed(#[from] Box<dyn std::error::Error + Send + Sync + 'static>),
    #[error("missing keymap: {0}")]
    MisingKeymap(String),
}

pub type VjjResult<T> = Result<T, VjjError>;

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, Eq, Hash)]
#[serde(rename_all = "UPPERCASE")]
pub enum Mode {
    #[default]
    Normal,
    Revset,
    Obslog(String),
}

impl Mode {
    pub fn header(&self) -> &str {
        match self {
            Mode::Normal | Mode::Obslog(_) => "Press ? for help, q to quit",
            Mode::Revset => "Press ? for help, ctrl+c to quit",
        }
    }

    pub fn keymap(&self) -> &str {
        match self {
            Mode::Normal => "normal",
            Mode::Revset => "revset",
            Mode::Obslog(_) => "obslog",
        }
    }
}

impl FromStr for Mode {
    type Err = SpannedError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ron::from_str(s.split(':').next().unwrap_or_default())
    }
}

impl Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: ", ron::to_string(self).unwrap().as_str())
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Selection {
    change: (String, String),
    commit: (String, String),
}

impl Selection {
    pub fn fzf_template(change_index: usize, commit_index: usize) -> Self {
        Self {
            change: (
                format!("{{{change_index}}}"),
                format!("{{+{change_index}}}"),
            ),
            commit: (
                format!("{{{commit_index}}}"),
                format!("{{+{commit_index}}}"),
            ),
        }
    }

    pub fn change(&self) -> IdSelection {
        Self::parse(&self.change)
    }

    pub fn commit(&self) -> IdSelection {
        Self::parse(&self.commit)
    }

    fn parse(ids: &(String, String)) -> IdSelection {
        let focused = ids.0.trim_matches('\'');
        IdSelection {
            focused: if focused.is_empty() {
                None
            } else {
                Some(focused.to_string())
            },
            selected: ids
                .1
                .split_whitespace()
                .map(|id| id.trim_matches('\'').to_string())
                .collect(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct IdSelection {
    pub focused: Option<String>,
    pub selected: Vec<String>,
}

impl IdSelection {
    pub fn selected_revset(&self) -> String {
        self.selected.join("|")
    }
}
