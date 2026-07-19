pub mod env;
pub mod serde;
pub mod statusline;

use process_control::{ChildExt, Control};
use std::ffi::OsStr;
use std::fmt::Debug;
use std::fs;
use std::io::{Error, ErrorKind, Result, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::context::Context;
use crate::context::Shell;

/// Default timeout for command execution in milliseconds
pub const DEFAULT_COMMAND_TIMEOUT_MS: u64 = 500;

/// Create a `PathBuf` from an absolute path, where the root directory will be mocked in test
#[cfg(not(test))]
#[inline]
#[allow(dead_code)]
pub fn context_path<S: AsRef<OsStr> + ?Sized>(_context: &Context, s: &S) -> PathBuf {
    PathBuf::from(s)
}

/// Create a `PathBuf` from an absolute path, where the root directory will be mocked in test
#[cfg(test)]
#[allow(dead_code)]
pub fn context_path<S: AsRef<OsStr> + ?Sized>(context: &Context, s: &S) -> PathBuf {
    let requested_path = PathBuf::from(s);

    if requested_path.is_absolute() {
        let mut path = PathBuf::from(context.root_dir.path());
        path.extend(requested_path.components().skip(1));
        path
    } else {
        requested_path
    }
}

/// Return the string contents of a file
pub fn read_file<P: AsRef<Path> + Debug>(file_name: P) -> Result<String> {
    log::trace!("Trying to read from {file_name:?}");

    let result = fs::read_to_string(file_name);

    if result.is_err() {
        log::debug!("Error reading file: {result:?}");
    } else {
        log::trace!("File read successfully");
    }

    result
}

/// Write a string to a file
#[cfg(test)]
pub fn write_file<P: AsRef<Path>, S: AsRef<str>>(file_name: P, text: S) -> Result<()> {
    let file_name = file_name.as_ref();
    let text = text.as_ref();

    log::trace!("Trying to write {text:?} to {file_name:?}");
    let mut file = match std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(file_name)
    {
        Ok(file) => file,
        Err(err) => {
            log::warn!("Error creating file: {err:?}");
            return Err(err);
        }
    };

    match file.write_all(text.as_bytes()) {
        Ok(()) => {
            log::trace!("File {file_name:?} written successfully");
        }
        Err(err) => {
            log::warn!("Error writing to file: {err:?}");
            return Err(err);
        }
    }
    file.sync_all()
}

/// Write contents to a file by first writing to a temporary file
/// and then move it to the target location in place
/// Only overwrites existing files if `force` is true
pub fn write_file_atomic<P: AsRef<Path>, S: AsRef<str>>(
    target_path: P,
    text: S,
    force: bool,
) -> std::result::Result<(), String> {
    let target_path = target_path.as_ref();
    let text = text.as_ref();

    log::trace!("Trying to write {text:?} to {target_path:?}");

    #[cfg_attr(not(unix), allow(unused_mut))]
    let mut builder = tempfile::Builder::new();

    // On Unix, the default permissions are too restrictive, so we need to relax them
    // This should be safe because we're creating a temporary file in the same directory
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let permissions = target_path.metadata().as_ref().map_or_else(
            |_| {
                let all_read_write = 0o666;
                std::fs::Permissions::from_mode(all_read_write)
            },
            fs::Metadata::permissions,
        );

        builder.permissions(permissions);
    }

    let Some(parent_dir) = target_path.parent() else {
        return Err(format!(
            "Unable to determine parent directory of {target_path:?}"
        ));
    };

    let mut temp_file = builder
        .tempfile_in(parent_dir)
        .map_err(|e| format!("Error creating temporary file: {e}"))?;

    if let Err(err) = temp_file.write_all(text.as_bytes()) {
        return Err(format!("Error writing to temporary file: {err}"));
    }

    let result = if force {
        temp_file.persist(target_path)
    } else {
        temp_file.persist_noclobber(target_path)
    };

    result.map_err(|e| {
        if !force && e.error.kind() == ErrorKind::AlreadyExists {
            "Error saving file, use --force to overwrite existing configuration file".to_string()
        } else {
            format!("Error moving temporary file to target location: {e}")
        }
    })?;

    log::trace!("File {target_path:?} written successfully");

    Ok(())
}

/// Reads command output from stderr or stdout depending on to which stream program streamed it's output
pub fn get_command_string_output(command: CommandOutput) -> String {
    if command.stdout.is_empty() {
        command.stderr
    } else {
        command.stdout
    }
}

/// Attempt to resolve `binary_name` from and creates a new `Command` pointing at it
/// This allows executing cmd files on Windows and prevents running executable from cwd on Windows
/// This function also initializes std{err,out,in} to protect against processes changing the console mode
pub fn create_command<T: AsRef<OsStr>>(binary_name: T) -> Result<Command> {
    let binary_name = binary_name.as_ref();
    log::trace!("Creating Command for binary {binary_name:?}");

    let full_path = match which::which(binary_name) {
        Ok(full_path) => {
            log::trace!("Using {full_path:?} as {binary_name:?}");
            full_path
        }
        Err(error) => {
            log::trace!("Unable to find {binary_name:?} in PATH, {error:?}");
            return Err(Error::new(ErrorKind::NotFound, error));
        }
    };

    #[allow(clippy::disallowed_methods)]
    let mut cmd = Command::new(full_path);
    cmd.stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .stdin(Stdio::null());

    Ok(cmd)
}

#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
}

impl PartialEq for CommandOutput {
    fn eq(&self, other: &Self) -> bool {
        self.stdout == other.stdout && self.stderr == other.stderr
    }
}

#[cfg(test)]
pub fn display_command<T: AsRef<OsStr> + Debug, U: AsRef<OsStr> + Debug>(
    cmd: T,
    args: &[U],
) -> String {
    std::iter::once(cmd.as_ref())
        .chain(args.iter().map(AsRef::as_ref))
        .map(|i| i.to_string_lossy().into_owned())
        .collect::<Vec<String>>()
        .join(" ")
}

/// Execute a command and return the output on stdout and stderr if successful
pub fn exec_cmd<T: AsRef<OsStr> + Debug, U: AsRef<OsStr> + Debug>(
    cmd: T,
    args: &[U],
    time_limit: Duration,
) -> Option<CommandOutput> {
    log::trace!("Executing command {cmd:?} with args {args:?}");
    #[cfg(test)]
    if let Some(o) = mock_cmd(&cmd, args) {
        return o;
    }
    internal_exec_cmd(cmd, args, time_limit)
}

#[cfg(test)]
pub fn mock_cmd<T: AsRef<OsStr> + Debug, U: AsRef<OsStr> + Debug>(
    cmd: T,
    args: &[U],
) -> Option<Option<CommandOutput>> {
    let command = display_command(&cmd, args);
    let out = match command.as_str() {
        "dummy_command" => Some(CommandOutput {
            stdout: String::from("stdout ok!\n"),
            stderr: String::from("stderr ok!\n"),
        }),
        "python --version" => None,
        "python2 --version" => Some(CommandOutput {
            stdout: String::default(),
            stderr: String::from("Python 2.7.17\n"),
        }),
        "python3 --version" => Some(CommandOutput {
            stdout: String::from("Python 3.8.0\n"),
            stderr: String::default(),
        }),
        "pyenv version-name" => Some(CommandOutput {
            stdout: String::from("system\n"),
            stderr: String::default(),
        }),
        _ => return None,
    };
    Some(out)
}

/// Wraps ANSI color escape sequences in the shell-appropriate wrappers.
pub fn wrap_colorseq_for_shell(ansi: String, shell: Shell) -> String {
    const ESCAPE_BEGIN: char = '\u{1b}';
    const ESCAPE_END: char = 'm';
    wrap_seq_for_shell(ansi, shell, ESCAPE_BEGIN, ESCAPE_END)
}

/// Many shells cannot deal with raw unprintable characters and miscompute the cursor position,
/// leading to strange visual bugs like duplicated/missing chars. This function wraps a specified
/// sequence in shell-specific escapes to avoid these problems.
pub fn wrap_seq_for_shell(
    ansi: String,
    shell: Shell,
    escape_begin: char,
    escape_end: char,
) -> String {
    let (beg, end) = match shell {
        // \[ and \]
        Shell::Bash => ("\u{5c}\u{5b}", "\u{5c}\u{5d}"),
        // %{ and %}
        Shell::Tcsh | Shell::Zsh => ("\u{25}\u{7b}", "\u{25}\u{7d}"),
        _ => return ansi,
    };

    // ANSI escape codes cannot be nested, so we can keep track of whether we're
    // in an escape or not with a single boolean variable
    let mut escaped = false;
    let final_string: String = ansi
        .chars()
        .map(|x| {
            if x == escape_begin && !escaped {
                escaped = true;
                format!("{beg}{escape_begin}")
            } else if x == escape_end && escaped {
                escaped = false;
                format!("{escape_end}{end}")
            } else {
                x.to_string()
            }
        })
        .collect();
    final_string
}

fn internal_exec_cmd<T: AsRef<OsStr> + Debug, U: AsRef<OsStr> + Debug>(
    cmd: T,
    args: &[U],
    time_limit: Duration,
) -> Option<CommandOutput> {
    let mut cmd = create_command(cmd).ok()?;
    cmd.args(args);
    exec_timeout(&mut cmd, time_limit)
}

pub fn exec_timeout(cmd: &mut Command, time_limit: Duration) -> Option<CommandOutput> {
    let start = Instant::now();
    let process = match cmd.spawn() {
        Ok(process) => process,
        Err(error) => {
            log::info!("Unable to run {:?}, {:?}", cmd.get_program(), error);
            return None;
        }
    };
    match process
        .controlled_with_output()
        .time_limit(time_limit)
        .terminate_for_timeout()
        .wait()
    {
        Ok(Some(output)) => {
            let stdout_string = match String::from_utf8(output.stdout) {
                Ok(stdout) => stdout,
                Err(error) => {
                    log::warn!("Unable to decode stdout: {error:?}");
                    return None;
                }
            };
            let stderr_string = match String::from_utf8(output.stderr) {
                Ok(stderr) => stderr,
                Err(error) => {
                    log::warn!("Unable to decode stderr: {error:?}");
                    return None;
                }
            };

            log::trace!(
                "stdout: {:?}, stderr: {:?}, exit code: \"{:?}\", took {:?}",
                stdout_string,
                stderr_string,
                output.status.code(),
                start.elapsed()
            );

            if !output.status.success() {
                return None;
            }

            Some(CommandOutput {
                stdout: stdout_string,
                stderr: stderr_string,
            })
        }
        Ok(None) => {
            log::warn!("Executing command {:?} timed out.", cmd.get_program());
            log::warn!(
                "You can set command_timeout in your config to a higher value to allow longer-running commands to keep executing."
            );
            None
        }
        Err(error) => {
            log::info!(
                "Executing command {:?} failed by: {:?}",
                cmd.get_program(),
                error
            );
            None
        }
    }
}

pub fn home_dir() -> Option<PathBuf> {
    dirs::home_dir()
}

pub trait PathExt {
    /// Get device / volume info
    fn device_id(&self) -> Option<u64>;
}

#[cfg(windows)]
impl PathExt for Path {
    fn device_id(&self) -> Option<u64> {
        // Maybe it should use unimplemented!
        Some(42u64)
    }
}

#[cfg(not(windows))]
impl PathExt for Path {
    #[cfg(target_os = "linux")]
    fn device_id(&self) -> Option<u64> {
        use std::os::linux::fs::MetadataExt;
        Some(self.metadata().ok()?.st_dev())
    }

    #[cfg(all(unix, not(target_os = "linux")))]
    fn device_id(&self) -> Option<u64> {
        use std::os::unix::fs::MetadataExt;
        Some(self.metadata().ok()?.dev())
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn exec_mocked_command() {
        let result = exec_cmd(
            "dummy_command",
            &[] as &[&OsStr],
            Duration::from_millis(DEFAULT_COMMAND_TIMEOUT_MS),
        );
        let expected = Some(CommandOutput {
            stdout: String::from("stdout ok!\n"),
            stderr: String::from("stderr ok!\n"),
        });

        assert_eq!(result, expected);
    }

    // While the exec_cmd should work on Windows some of these tests assume a Unix-like
    // environment.

    #[test]
    #[cfg(not(windows))]
    fn exec_no_output() {
        let result = internal_exec_cmd(
            "true",
            &[] as &[&OsStr],
            Duration::from_millis(DEFAULT_COMMAND_TIMEOUT_MS),
        );
        let expected = Some(CommandOutput {
            stdout: String::new(),
            stderr: String::new(),
        });

        assert_eq!(result, expected);
    }

    #[test]
    #[cfg(not(windows))]
    fn exec_with_output_stdout() {
        let result = internal_exec_cmd(
            "/bin/sh",
            &["-c", "echo hello"],
            Duration::from_millis(DEFAULT_COMMAND_TIMEOUT_MS),
        );
        let expected = Some(CommandOutput {
            stdout: String::from("hello\n"),
            stderr: String::new(),
        });

        assert_eq!(result, expected);
    }

    #[test]
    #[cfg(not(windows))]
    fn exec_with_output_stderr() {
        let result = internal_exec_cmd(
            "/bin/sh",
            &["-c", "echo hello >&2"],
            Duration::from_millis(DEFAULT_COMMAND_TIMEOUT_MS),
        );
        let expected = Some(CommandOutput {
            stdout: String::new(),
            stderr: String::from("hello\n"),
        });

        assert_eq!(result, expected);
    }

    #[test]
    #[cfg(not(windows))]
    fn exec_with_output_both() {
        let result = internal_exec_cmd(
            "/bin/sh",
            &["-c", "echo hello; echo world >&2"],
            Duration::from_millis(DEFAULT_COMMAND_TIMEOUT_MS),
        );
        let expected = Some(CommandOutput {
            stdout: String::from("hello\n"),
            stderr: String::from("world\n"),
        });

        assert_eq!(result, expected);
    }

    #[test]
    #[cfg(not(windows))]
    fn exec_with_non_zero_exit_code() {
        let result = internal_exec_cmd(
            "false",
            &[] as &[&OsStr],
            Duration::from_millis(DEFAULT_COMMAND_TIMEOUT_MS),
        );
        let expected = None;

        assert_eq!(result, expected);
    }

    #[test]
    #[cfg(not(windows))]
    fn exec_slow_command() {
        let result = internal_exec_cmd(
            "sleep",
            &["500"],
            Duration::from_millis(DEFAULT_COMMAND_TIMEOUT_MS),
        );
        let expected = None;

        assert_eq!(result, expected);
    }

    #[test]
    fn test_color_sequence_wrappers() {
        let test0 = "\x1b2mhellomynamekeyes\x1b2m"; // BEGIN: \x1b     END: m
        let test1 = "\x1b]330;mlol\x1b]0m"; // BEGIN: \x1b     END: m
        let test2 = "\u{1b}J"; // BEGIN: \x1b     END: J
        let test3 = "OH NO"; // BEGIN: O    END: O
        let test4 = "herpaderp";
        let test5 = "";

        let zresult0 = wrap_seq_for_shell(test0.to_string(), Shell::Zsh, '\x1b', 'm');
        let zresult1 = wrap_seq_for_shell(test1.to_string(), Shell::Zsh, '\x1b', 'm');
        let zresult2 = wrap_seq_for_shell(test2.to_string(), Shell::Zsh, '\x1b', 'J');
        let zresult3 = wrap_seq_for_shell(test3.to_string(), Shell::Zsh, 'O', 'O');
        let zresult4 = wrap_seq_for_shell(test4.to_string(), Shell::Zsh, '\x1b', 'm');
        let zresult5 = wrap_seq_for_shell(test5.to_string(), Shell::Zsh, '\x1b', 'm');

        assert_eq!(&zresult0, "%{\x1b2m%}hellomynamekeyes%{\x1b2m%}");
        assert_eq!(&zresult1, "%{\x1b]330;m%}lol%{\x1b]0m%}");
        assert_eq!(&zresult2, "%{\x1bJ%}");
        assert_eq!(&zresult3, "%{OH NO%}");
        assert_eq!(&zresult4, "herpaderp");
        assert_eq!(&zresult5, "");

        let bresult0 = wrap_seq_for_shell(test0.to_string(), Shell::Bash, '\x1b', 'm');
        let bresult1 = wrap_seq_for_shell(test1.to_string(), Shell::Bash, '\x1b', 'm');
        let bresult2 = wrap_seq_for_shell(test2.to_string(), Shell::Bash, '\x1b', 'J');
        let bresult3 = wrap_seq_for_shell(test3.to_string(), Shell::Bash, 'O', 'O');
        let bresult4 = wrap_seq_for_shell(test4.to_string(), Shell::Bash, '\x1b', 'm');
        let bresult5 = wrap_seq_for_shell(test5.to_string(), Shell::Bash, '\x1b', 'm');

        assert_eq!(&bresult0, "\\[\x1b2m\\]hellomynamekeyes\\[\x1b2m\\]");
        assert_eq!(&bresult1, "\\[\x1b]330;m\\]lol\\[\x1b]0m\\]");
        assert_eq!(&bresult2, "\\[\x1bJ\\]");
        assert_eq!(&bresult3, "\\[OH NO\\]");
        assert_eq!(&bresult4, "herpaderp");
        assert_eq!(&bresult5, "");
    }

    #[test]
    fn test_get_command_string_output() {
        let case1 = CommandOutput {
            stdout: String::from("stdout"),
            stderr: String::from("stderr"),
        };
        assert_eq!(get_command_string_output(case1), "stdout");
        let case2 = CommandOutput {
            stdout: String::new(),
            stderr: String::from("stderr"),
        };
        assert_eq!(get_command_string_output(case2), "stderr");
    }

    #[test]
    fn test_write_file_atomic() -> Result<()> {
        // Create a temporary file for testing
        let tmp_dir = tempdir()?;
        let path = tmp_dir.path().join("test_config.toml");

        let expected = "test data";
        write_file_atomic(&path, expected, false).unwrap();

        let actual_data = read_file(&path)?;
        assert_eq!(actual_data, expected);

        tmp_dir.close()
    }

    #[test]
    fn test_write_file_atomic_already_exists() -> Result<()> {
        let tmp_dir = tempdir()?;
        let tmp_file_path = tmp_dir.path().join("test_config.toml");

        write_file(&tmp_file_path, "existing data")?;

        let err = write_file_atomic(&tmp_file_path, "should not contain this", false).unwrap_err();
        assert!(err.contains("--force"));

        let actual_data = read_file(&tmp_file_path)?;
        assert_eq!(actual_data, "existing data");

        tmp_dir.close()
    }

    #[test]
    fn test_write_file_atomic_overwrite() -> Result<()> {
        let tmp_dir = tempdir()?;
        let path = tmp_dir.path().join("test_config.toml");

        write_file(&path, "existing data")?;

        let expected = "test data";
        write_file_atomic(&path, expected, true).unwrap();

        let actual = read_file(&path)?;
        assert_eq!(actual, expected);

        tmp_dir.close()
    }
}
