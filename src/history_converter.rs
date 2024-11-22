use std::{fmt::Display, ops::Deref, path::PathBuf, sync::LazyLock};

use anyhow::{bail, Result};
use clap::Parser;
use regex::Regex;
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, BufReader},
};

/// A zsh history entry
#[derive(Debug)]
pub struct Entry {
    /// The command executed.
    pub cmd: String,
    /// The time the command was executed. Set to 0 if the time information is not available.
    pub when: i64,
}

impl Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "- cmd: {}\n  when: {}", self.cmd, self.when)
    }
}

/// A marker trait to represent the state of the converter.
pub trait State {}

/// A zsh history to fish history converter. To prevent the impossible operation from executing
/// (i.e. run convert before checking if the history file exists), we use a state machine to track
/// the state of the converter. The state transitions are:
///
/// Uninitialized -> Initialized
pub struct Converter<S>
where
    S: State,
{
    state: S,
}

/// Convenient deref implementation to access the inner state.
impl<S> Deref for Converter<S>
where
    S: State,
{
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

/// The uninitialized state of the converter.
#[derive(Debug, Parser)]
#[clap(about, version)]
pub struct Uninitialized {
    /// The path to the zsh history file.
    #[arg()]
    pub zsh_history: PathBuf,
}
impl State for Uninitialized {}

/// The initialized state of the converter.
#[derive(Debug)]
pub struct Initialized {
    file: File,
}
impl State for Initialized {}

impl Converter<Uninitialized> {
    /// Create a new converter from the given path.
    pub async fn new(path: &PathBuf) -> Result<Converter<Initialized>> {
        Ok(Converter {
            state: Initialized { file: File::open(path).await? },
        })
    }

    /// Parse the command line arguments, check if the zsh history file exists, and return a new
    /// converter.
    pub async fn from_args() -> Result<Converter<Initialized>> {
        let Uninitialized { zsh_history } = Uninitialized::parse();
        if !zsh_history.exists() {
            bail!("zsh history file does not exist: {}", zsh_history.display());
        }
        Self::new(&zsh_history).await
    }
}

impl Converter<Initialized> {
    /// Convert the zsh history file to fish history.
    pub async fn convert(&self) -> Result<Vec<Entry>> {
        let mut buf = Vec::new();
        let mut entries = Vec::new();

        // [`try_clone`] shares the underlying file handle with the original file, so the cost of
        // cloning is minimal, I believe.
        let mut file = BufReader::new(self.file.try_clone().await?);

        loop {
            buf.clear();
            let bytes_read = file.read_until(b'\n', &mut buf).await?;

            if bytes_read == 0 {
                break; // EOF
            }

            if let Some(entry) = Self::parse_zsh_history_line(&buf) {
                entries.push(entry)
            }
        }

        Ok(entries)
    }

    // zsh history format is typically: ": timestamp:0;command", or simply "command"
    fn parse_zsh_history_line(bytes: &[u8]) -> Option<Entry> {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^: (\d+):(?:0;)?(.+)$").unwrap());

        let line = Self::decode(bytes);
        let line = line.trim();

        // Skip multi-line history
        if line.ends_with(r#"\"#) {
            return None;
        }

        if let Some(caps) = RE.captures(line) {
            if let Ok(when) = caps[1].parse::<i64>() {
                return Some(Entry { cmd: caps[2].to_string(), when });
            }
        }

        // If no match, treat the whole line as a command
        Some(Entry { cmd: line.to_string(), when: 0 })
    }

    // zsh treats non-ASCII characters strangely. See also: https://syossan.hateblo.jp/entry/2017/10/09/181928
    fn decode(bytes: &[u8]) -> String {
        let mut buf = Vec::new();

        let mut marked = false;
        bytes.iter().for_each(|byte| match byte {
            0x83 => {
                marked = true;
            }
            b if marked => {
                buf.push(b ^ 0b0010_0000);
                marked = false;
            }
            b => buf.push(*b),
        });

        // assuming we now have a valid UTF-8 string
        String::from_utf8_lossy(&buf).into_owned()
    }
}
