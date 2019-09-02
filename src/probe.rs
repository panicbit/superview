use snafu::ResultExt;
use std::path::{Path, PathBuf};
use std::io;
use std::process::{Command, ExitStatus, Stdio};
use std::str::FromStr;
use std::fmt;
use serde::de;
use serde::Deserialize;

#[cfg(target_os = "windows")]
const FFPROBE_EXE: &str = "ffprobe.exe";
#[cfg(not(target_os = "windows"))]
const FFPROBE_EXE: &str = "ffprobe";

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Could not ffprobe: {}", source))]
    ProcessError {
        source: io::Error,
    },
    #[snafu(display("'{}' exited unsuccessfully ({})", command, status))]
    FfmpegError {
        command: String,
        status: ExitStatus,
    },
    #[snafu(display("Could not parse ffprobe output for '{}': {}", path.display(), source))]
    ParseProbeOutput {
        path: PathBuf,
        source: json::Error,
    },
}

pub fn probe(path: &Path) -> Result<Specs, Error> {
    let output = Command::new(FFPROBE_EXE)
        .arg("-i").arg(path)
        .args(&[
            "-v", "error",
            "-select_streams", "v:0",
            "-show_entries", "stream=codec_name,width,height,duration,bit_rate",
            "-print_format", "json"
        ])
        .stderr(Stdio::piped())
        .output()
        .context(ProcessError)?;

    ensure!(output.status.success(), FfmpegError {
        command: FFPROBE_EXE,
        status: output.status,
    });
    
    let specs = json::from_slice::<Specs>(&output.stdout)
        .context(ParseProbeOutput { path })?;

    Ok(specs)
}

#[derive(Deserialize)]
pub struct Specs {
    pub streams: Vec<Stream>,
}

#[derive(Deserialize)]
pub struct Stream {
    pub codec_name: String,
    pub width: u32,
    pub height: u32,
    #[serde(deserialize_with="from_str")]
    pub duration: f64,
    #[serde(deserialize_with="from_str")]
    #[serde(rename="bit_rate")]
    pub bitrate: u32,
}

fn from_str<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where T: FromStr,
          T::Err: fmt::Display,
          D: de::Deserializer<'de>
{
    let s = String::deserialize(deserializer)?;
    T::from_str(&s).map_err(de::Error::custom)
}
