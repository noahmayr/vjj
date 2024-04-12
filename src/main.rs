pub mod common;
mod fzf;
mod shell;

use clap::Parser;
use common::{Mode, Selection};

use fzf::{Bind, Event, Fzf, FzfAction};
use shell::{
    command::VjjCommand,
    fzf_binding::{FzfBindHandler, InputKind},
    keymap::which_key,
    vjj_shell, VjjShellExpression,
};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short)]
    command: Option<VjjShellExpression>,
}

fn main() {
    let args = Args::parse();

    if let Some(expression) = args.command {
        if let Some(output) = vjj_shell(expression) {
            if output.is_empty() {
                return;
            }
            println!("{}", output);
        }
        return;
    }

    let mode = Mode::default();

    let (change_index, commit_index) = (2, 3);

    let mut fzf = Fzf::build()
        .add_flag("ansi")
        .add_flag("no-cycle")
        .add_flag("no-sort")
        .add_flag("no-info")
        .add_flag("multi")
        .add_flag("phony")
        .add_flag_with_value("color", "header:-1")
        .add_flag_with_value("with-nth", "1,4..")
        .add_flag_with_value("layout", "reverse-list")
        .add_flag_with_value("height", "100%")
        .add_flag_with_value("preview-window", "right")
        .add_flag_with_value("delimiter", "\u{200B}")
        .add_flag_with_value("prompt", &mode.to_string())
        .add_flag_with_value(
            "header",
            which_key(&mode, "").unwrap_or(mode.header().to_string()),
        )
        .add_flag_with_value(
            "preview",
            &VjjShellExpression::Command(VjjCommand::Show(Selection::fzf_template(
                change_index,
                commit_index,
            ))),
        )
        .add_flag_with_value(
            "preview",
            &VjjShellExpression::Command(VjjCommand::Show(Selection::fzf_template(
                change_index,
                commit_index,
            ))),
        )
        .bind(Bind::Actions(
            Event::Start,
            vec![FzfAction::Reload(VjjCommand::Log)],
        ))
        .bind(Bind::Actions(
            Event::Start,
            vec![FzfAction::Reload(VjjCommand::Log)],
        ))
        .bind(Bind::Transform(
            Event::Change,
            FzfBindHandler::Input {
                kind: InputKind::Change,
                selection: Selection::fzf_template(change_index, commit_index),
            },
        ))
        .bind(Bind::Transform(
            Event::Enter,
            FzfBindHandler::Input {
                kind: InputKind::Enter,
                selection: Selection::fzf_template(change_index, commit_index),
            },
        ))
        .bind(Bind::Transform(
            Event::Esc,
            FzfBindHandler::Input {
                kind: InputKind::Esc,
                selection: Selection::fzf_template(change_index, commit_index),
            },
        ))
        .bind(Bind::Transform(
            Event::Focus,
            FzfBindHandler::Focus {
                change: format!("{{{change_index}}}"),
                commit: format!("{{{commit_index}}}"),
                action: "{fzf:action}".to_string(),
            },
        ))
        .spawn()
        .unwrap();
    fzf.wait().unwrap();
}
