use std::{
    ffi::OsStr,
    fs::{self, File},
    io::{self, Write},
    process::{Child, ChildStdin, Command, Output, Stdio},
};

use crate::common::VjjResult;

pub struct Pager {
    pub child: Child,
    pub stdin: ChildStdin,
}

pub fn get_pager() -> io::Result<Pager> {
    let tty = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")?;
    // Can use the tty_input for keys while also reading stdin for data.

    let mut child = Command::new("less")
        .arg("-KR")
        .arg("--tabs=4")
        .arg("-+FX")
        .stdin(Stdio::piped())
        .stdout(Stdio::from(tty))
        .spawn()?;
    let stdin = child.stdin.take().unwrap();
    Ok(Pager { child, stdin })
}

fn tty_stdin() -> VjjResult<File> {
    Ok(fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(ctty::get_path_for_dev(ctty::get_ctty_dev()?)?)?)
}

pub fn jj<I, S>(args: I, pager: bool, interactive: bool) -> VjjResult<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = Command::new("jj");
    command.arg("--color=always").arg("--no-pager").args(args);

    match (interactive, pager) {
        (false, false) => command.spawn()?.wait()?,
        (true, pager) => {
            command.stdin(tty_stdin()?);
            if pager {
                command.stderr(Stdio::piped());
                let output = command.spawn()?.wait_with_output()?;
                let Pager {
                    mut child,
                    mut stdin,
                } = get_pager()?;
                stdin.write_all(&output.stderr)?;
                child.wait()?;
                output.status
            } else {
                command.spawn()?.wait()?
            }
        }

        (false, true) => {
            command.stdout(Stdio::piped());
            command.stderr(Stdio::piped());
            let Output {
                status,
                stdout,
                stderr,
            } = command.output()?;
            let Pager {
                mut child,
                mut stdin,
            } = get_pager()?;
            stdin.write_all(&stdout)?;
            stdin.write_all(&stderr)?;
            drop(stdin);
            child.wait()?;

            status
        }
    };

    // if interactive {
    // } else {
    //     command.stdout(Stdio::piped());
    // };
    // command.stderr(Stdio::piped());

    // if pager {
    //     let output = command.spawn()?.wait_with_output()?;
    //     let Pager {
    //         mut child,
    //         mut stdin,
    //     } = get_pager()?;
    //     stdin.write(&output.stderr)?;
    //     stdin.write(&output.stdout)?;
    //     drop(stdin);
    //     child.wait()?;
    // } else {
    //     let output = command.output()?;
    //     io::stdout().write(&output.stdout)?;
    //     io::stderr().write(&output.stderr)?;
    // }

    Ok(())
}
