use std::{
    fmt::Display,
    io,
    ops::{Deref, DerefMut},
    process::{Child, Command},
};

use itertools::Itertools;

use crate::{
    common::Mode,
    shell::{command::VjjCommand, fzf_binding::FzfBindHandler},
    VjjShellExpression,
};

pub struct Fzf {
    child: Child,
}

impl Fzf {
    pub fn build() -> FzfBuilder {
        FzfBuilder {
            command: Command::new("fzf"),
        }
    }
}

impl Deref for Fzf {
    type Target = Child;

    fn deref(&self) -> &Self::Target {
        &self.child
    }
}
impl DerefMut for Fzf {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.child
    }
}

pub struct FzfBuilder {
    command: Command,
}

impl FzfBuilder {
    pub fn add_flag<S: Display>(&mut self, flag: S) -> &mut Self {
        self.command.arg(format!("--{flag}"));
        self
    }

    pub fn add_flag_with_value<N: Display, V: Display>(&mut self, name: N, value: V) -> &mut Self {
        self.add_flag(format!("{name}={value}"));
        self
    }

    pub fn bind(&mut self, bind: Bind) -> &mut Self {
        self.add_flag_with_value(
            "bind",
            match bind {
                Bind::Actions(event, actions) => format!(
                    "{event}:{}",
                    actions.iter().map(|action| action.to_string()).join("+")
                ),
                Bind::Transform(event, handler) => {
                    format!(
                        "{event}:transform:{}",
                        VjjShellExpression::Handler(handler.clone())
                    )
                }
            },
        );
        self
    }

    pub fn spawn(&mut self) -> io::Result<Fzf> {
        self.command.env("SHELL", std::env::current_exe()?);
        Ok(Fzf {
            child: self.command.spawn()?,
        })
    }
}

impl From<Fzf> for Child {
    fn from(val: Fzf) -> Self {
        val.child
    }
}

#[derive(Debug, Clone, strum::Display)]
#[strum(serialize_all = "lowercase")]
pub enum Event {
    Change,
    Enter,
    Start,
    Focus,
    Esc,
}

#[derive(Debug, Clone)]
pub enum Bind {
    Actions(Event, Vec<FzfAction>),
    Transform(Event, FzfBindHandler),
}

#[derive(Debug, Clone, strum::AsRefStr)]
#[strum(serialize_all = "kebab_case")]
#[allow(dead_code)]
pub enum FzfAction {
    ClearQuery,
    Abort,
    Up,
    Down,
    ChangePrompt(Mode),
    ChangeBorderLabel(String),
    ChangePreview(VjjCommand),
    Preview(VjjCommand),
    ChangeHeader(String),
    Become(VjjCommand),
    Execute {
        command: VjjCommand,
        interactive: bool,
    },
    ExecuteSilent(VjjCommand),
    Reload(VjjCommand),
}

impl FzfAction {
    fn params(&self) -> Option<String> {
        match self {
            FzfAction::ClearQuery | FzfAction::Abort | FzfAction::Up | FzfAction::Down => None,
            FzfAction::ChangePrompt(value) => Some(value.to_string()),
            FzfAction::ChangeHeader(value) | FzfAction::ChangeBorderLabel(value) => {
                Some(value.to_owned())
            }
            FzfAction::ChangePreview(command)
            | FzfAction::Preview(command)
            | FzfAction::Become(command)
            | FzfAction::ExecuteSilent(command)
            | FzfAction::Reload(command) => {
                Some(VjjShellExpression::Command(command.clone()).to_string())
            }
            FzfAction::Execute {
                command,
                interactive,
            } => Some(
                VjjShellExpression::PagedCommand {
                    command: command.clone(),
                    interactive: *interactive,
                }
                .to_string(),
            ),
        }
    }
}

impl Display for FzfAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = self.as_ref();
        if let Some(params) = self.params() {
            write!(f, "{name}({params})")
        } else {
            write!(f, "{name}")
        }
    }
}
