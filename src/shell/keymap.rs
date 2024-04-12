use std::collections::{BTreeMap, HashMap};
use std::fmt::Display;
use std::io;
use std::process::{Command, Stdio};

use copypasta::{ClipboardContext, ClipboardProvider};
use itertools::Itertools;
use leon::{ParseError, RenderError, Template, Values};
use once_cell::sync::Lazy;
use ron::error::SpannedError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::common::{Mode, Selection, VjjError, VjjResult};
use crate::fzf::FzfAction;
use crate::shell::command::VjjCommand;
use crate::shell::ShellContext;

// TODO: add user configurable keymap
static KEYMAP: Lazy<Result<KeyMap, SpannedError>> =
    Lazy::new(|| ron::from_str(include_str!("../../keymap.ron")));

pub fn get_keymap(mode: &Mode) -> VjjResult<&'static BTreeMap<Bindable, Keybind>> {
    match KEYMAP.as_ref() {
        Ok(keymap) => match keymap.get(mode.keymap()) {
            Some(keymap) => Ok(keymap),
            None => Err(VjjError::MisingKeymap(mode.keymap().to_string())),
        },
        Err(e) => Err(e.to_owned().into()),
    }
}

impl From<VjjError> for Vec<FzfAction> {
    fn from(value: VjjError) -> Self {
        vec![FzfAction::Execute {
            command: VjjCommand::Error(value.to_string()),
            interactive: false,
        }]
    }
}

pub fn handle_key_event(ctx: KeyHandlerContext) -> Option<Vec<FzfAction>> {
    if ctx.key == Bindable::new("?") {
        return Some(vec![FzfAction::Execute {
            command: VjjCommand::Help,
            interactive: false,
        }]);
    }

    let keymap = match get_keymap(&ctx.mode) {
        Ok(keymap) => keymap,
        Err(e) => return Some(e.into()),
    };
    let Some(Keybind(_, actions)) = keymap.get(&ctx.key) else {
        print!(
            "{}",
            FzfAction::ChangeHeader(
                which_key(&ctx.mode, &ctx.query).unwrap_or(ctx.mode.header().to_string())
            )
        );
        return None;
    };

    if actions.is_empty() {
        print!(
            "{}",
            FzfAction::ChangeHeader(
                which_key(&ctx.mode, &ctx.query).unwrap_or(ctx.mode.header().to_string())
            )
        );
        return None;
    }

    let mut fzf_actions = vec![FzfAction::ChangeHeader(
        which_key(&ctx.mode, "").unwrap_or(ctx.mode.header().to_string()),
    )];

    fzf_actions.append(
        &mut actions
            .iter()
            .map(|action| -> VjjResult<Vec<FzfAction>> {
                Ok(match action {
                    UserAction::Quit => vec![FzfAction::Abort],
                    UserAction::ReloadLog => vec![
                        FzfAction::Reload(VjjCommand::Log),
                        FzfAction::ChangeHeader(
                            which_key(&ctx.mode, &ctx.query)
                                .unwrap_or(ctx.mode.header().to_string()),
                        ),
                    ],
                    UserAction::Mode(mode) => {
                        let mode = mode.render_and_eval(&ctx)?;
                        vec![
                            FzfAction::ChangePrompt(mode.clone()),
                            FzfAction::ChangeHeader(
                                which_key(&mode, &ctx.query).unwrap_or(mode.header().to_string()),
                            ),
                            FzfAction::Reload(VjjCommand::Log),
                        ]
                    }
                    UserAction::Jujutsu(args) => {
                        let mut command = Command::new("jj");
                        let args = args.render(&ctx)?;
                        command
                            .arg("--color=always")
                            .arg("--no-pager")
                            .args(&args)
                            .stdout(Stdio::piped())
                            .stderr(Stdio::piped());
                        let output = command.output()?;

                        vec![
                            FzfAction::Reload(VjjCommand::Log),
                            FzfAction::ChangePreview(VjjCommand::Output(format!(
                                "{}{}",
                                String::from_utf8_lossy(&output.stdout),
                                String::from_utf8_lossy(&output.stderr)
                            ))),
                            FzfAction::ChangePreviewLabel(format!(
                                "Output (jj {})",
                                args.join(" ")
                            )),
                        ]
                    }
                    UserAction::JujutsuPaged(args) => {
                        vec![
                            FzfAction::Execute {
                                command: VjjCommand::Jujutsu(args.render(&ctx)?),
                                interactive: false,
                            },
                            FzfAction::Reload(VjjCommand::Log),
                        ]
                    }
                    UserAction::JujutsuInteractive(args) => {
                        vec![
                            FzfAction::Execute {
                                command: VjjCommand::Jujutsu(args.render(&ctx)?),
                                interactive: true,
                            },
                            FzfAction::Reload(VjjCommand::Log),
                        ]
                    }
                    UserAction::Yank(command) => {
                        ClipboardContext::new()?.set_contents(command.render(&ctx)?.evaluate()?)?;
                        vec![]
                    }
                    UserAction::ChangeRevset(command) => vec![
                        FzfAction::ChangeBorderLabel(command.render(&ctx)?.evaluate()?),
                        FzfAction::Reload(VjjCommand::Log),
                    ],
                    UserAction::Accept(command) => vec![FzfAction::Become(VjjCommand::Output(
                        command.render(&ctx)?.evaluate()?,
                    ))],
                })
            })
            .flat_map(|result| match result {
                Ok(actions) => actions,
                Err(e) => e.into(),
            })
            .collect_vec(),
    );

    Some(fzf_actions)
}

pub fn which_key(mode: &Mode, query: &str) -> Option<String> {
    let keymap = get_keymap(mode).ok()?;

    let options = keymap
        .iter()
        .filter(|(key, _)| match key {
            Bindable::Sequence(keys) => keys.starts_with(query) && keys.len() == query.len() + 1,
            _ => false,
        })
        .collect_vec();

    if options.is_empty() {
        return None;
    }

    let mut left = options;
    let right = left.split_off(left.len() / 2 + (left.len() % 2));

    let align = |options: Vec<(&Bindable, &Keybind)>| {
        let max_len = options
            .iter()
            .map(|(key, _)| key.to_string().len())
            .max()
            .unwrap_or(0);

        options
            .iter()
            .map(|(key, Keybind(help, actions))| {
                let color = if actions.is_empty() {
                    ansi_term::Color::Blue
                } else {
                    ansi_term::Color::Red
                };
                format!(
                    "{}  {help}",
                    color.paint(format!("{:width$}", key.to_string(), width = max_len)),
                )
            })
            .collect_vec()
    };

    let left = align(left);
    let right = align(right);

    let max_len = left.iter().map(|left| left.len()).max().unwrap_or(0);

    Some(
        left.iter()
            .zip_longest(right)
            .map(|item| match item {
                itertools::EitherOrBoth::Both(left, right) => {
                    format!("{left:width$} │ {right}", width = max_len)
                }
                itertools::EitherOrBoth::Left(left) => {
                    format!("{left:width$} │", width = max_len)
                }
                itertools::EitherOrBoth::Right(right) => {
                    format!("{:width$} │ {right}", "", width = max_len)
                }
            })
            .join("\n"),
    )
}

#[derive(Error, Debug)]
pub enum TemplateError {
    #[error("{0}")]
    Parse(#[from] ParseError),
    #[error("{0}")]
    Render(#[from] RenderError),
}

trait VjjTemplate {
    fn render(&self, ctx: &KeyHandlerContext) -> Result<Self, TemplateError>
    where
        Self: Sized;
}

impl VjjTemplate for String {
    fn render(&self, ctx: &KeyHandlerContext) -> Result<String, TemplateError>
    where
        Self: Sized,
    {
        let template: Template<'_> = Template::parse(self.as_ref())?;
        Ok(template.render(ctx)?)
    }
}

impl VjjTemplate for Vec<String> {
    fn render(&self, ctx: &KeyHandlerContext) -> Result<Vec<String>, TemplateError>
    where
        Self: Sized,
    {
        self.iter().map(|str| str.render(ctx)).collect()
    }
}

impl VjjTemplate for UserCommand {
    fn render(&self, ctx: &KeyHandlerContext) -> Result<Self, TemplateError>
    where
        Self: Sized,
    {
        Ok(match self {
            UserCommand::Shell { command } => UserCommand::Shell {
                command: command.render(ctx)?,
            },
            UserCommand::Plain(str) => UserCommand::Plain(str.render(ctx)?),
        })
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum UserCommand {
    Shell { command: String },
    Plain(String),
}

impl UserCommand {
    pub fn evaluate(&self) -> io::Result<String> {
        match self {
            UserCommand::Shell { command } => {
                let output = Command::new("/bin/sh").arg("-c").arg(command).output()?;

                Ok(String::from_utf8_lossy(&output.stdout)
                    .into_owned()
                    .trim_end()
                    .to_string())
            }
            UserCommand::Plain(str) => Ok(str.clone()),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum UserAction {
    Quit,
    ReloadLog,
    Mode(UserMode),
    #[serde(rename = "jj")]
    Jujutsu(Vec<String>),
    #[serde(rename = "jjp")]
    JujutsuPaged(Vec<String>),
    #[serde(rename = "jji")]
    JujutsuInteractive(Vec<String>),
    Yank(UserCommand),
    ChangeRevset(UserCommand),
    Accept(UserCommand),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum UserMode {
    Normal,
    Revset,
    Obslog(UserCommand),
}

impl UserMode {
    pub fn render_and_eval(&self, ctx: &KeyHandlerContext) -> VjjResult<Mode> {
        Ok(match self {
            UserMode::Normal => Mode::Normal,
            UserMode::Revset => Mode::Revset,
            UserMode::Obslog(command) => Mode::Obslog(command.render(ctx)?.evaluate()?),
        })
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Keybind(pub String, pub Vec<UserAction>);

#[derive(Debug, Deserialize, Serialize, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[serde(untagged)]
pub enum Bindable {
    Key(Key),
    Sequence(String),
}

impl From<Key> for Bindable {
    fn from(value: Key) -> Self {
        Self::Key(value)
    }
}

impl From<String> for Bindable {
    fn from(value: String) -> Self {
        Self::Sequence(value)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Key {
    Esc,
    Enter,
}

impl Bindable {
    pub fn new<S: ToString>(keys: S) -> Self {
        Self::Sequence(keys.to_string())
    }
}

impl Display for Bindable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Bindable::Key(Key::Esc) => "esc".to_string(),
                Bindable::Key(Key::Enter) => "enter".to_string(),
                Bindable::Sequence(keys) => keys.replace(' ', "<space>"),
            }
        )
    }
}

type KeyMap = HashMap<String, BTreeMap<Bindable, Keybind>>;

#[derive(Debug, Clone)]
pub struct KeyHandlerContext {
    key: Bindable,
    mode: Mode,
    query: String,
    selection: Selection,
}

impl KeyHandlerContext {
    pub fn new(key: Bindable, selection: Selection, shell_ctx: ShellContext) -> KeyHandlerContext {
        KeyHandlerContext {
            key,
            mode: shell_ctx.mode,
            query: shell_ctx.query,
            selection,
        }
    }
}

impl Values for KeyHandlerContext {
    fn get_value(&self, key: &str) -> Option<std::borrow::Cow<'_, str>> {
        Some(std::borrow::Cow::Owned(match key {
            "query" => self.query.clone(),
            "change:focused" => self.selection.change().focused?,
            "change:selected" => self.selection.change().selected.join("\n"),
            "change:selected_revset" => self.selection.change().selected_revset(),
            "commit:focused" => self.selection.commit().focused?,
            "commit:selected" => self.selection.commit().selected.join("\n"),
            "commit:selected_revset" => self.selection.commit().selected_revset(),
            _ => return None,
        }))
    }
}
