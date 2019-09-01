use std::process::{Command, ExitStatus, Stdio};
use std::path::Path;
use std::io::{self, BufReader, BufRead, Write};
use snafu::ResultExt;

pub fn reencode(config: Config) -> Result<(), Error> {
    let mut ffmpg = Command::new("ffmpeg")
        .args(&[
            "-hide_banner",
            "-progress", "pipe:1",
            "-loglevel", "panic",
            "-y",
            "-re",
        ])
        .arg("-i").arg(config.input)
        .args(&["-f", "pgm_pipe"])
        .arg("-i").arg(config.x_filt)
        .args(&["-f", "pgm_pipe"])
        .arg("-i").arg(config.y_filt)
        .args(&["-filter_complex", "remap,format=yuv444p,format=yuv420p"])
        .arg("-c:v").arg(config.codec_name)
        .arg("-b:v").arg(config.bitrate.to_string())
        .args(&[
            "-c:a", "copy",
            "-x265-params", "log-level=error",
        ])
        .arg(config.output)
        .stdout(Stdio::piped())
        .spawn()
        .context(ProcessError)?;
    
    let stdout = ffmpg.stdout.take().unwrap();
    let stdout = BufReader::new(stdout);

    for line in stdout.lines() {
        let line = match line {
            Ok(line) => line,
            Err(_) => break,
        };

        const OUT_TIME_MS: &str = "out_time_ms=";

        // println!("ffmpeg out: {}", line);

        if line.starts_with(OUT_TIME_MS) {
            let progress = &line[OUT_TIME_MS.len()..];
            let progress = progress.parse::<f64>().unwrap_or(0.);
            let progress = progress / (config.duration * 10_000.);

            eprint!("\rEncoding progress: {:.2}%", progress);
            io::stderr().flush().ok();
        }
    }

    eprintln!();

    let status = ffmpg.wait().context(ProcessError)?;
    ensure!(status.success(), FfmpegError { status });

    Ok(())
}

pub struct Config<'a> {
    pub input: &'a Path,
    pub output: &'a Path,
    pub x_filt: &'a Path,
    pub y_filt: &'a Path,
    pub codec_name: &'a str,
    pub bitrate: u32,
    pub duration: f64,
}

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Could not run ffmpeg: {}", source))]
    ProcessError {
        source: io::Error,
    },
    #[snafu(display("ffmpeg exited unsuccessfully ({})", status))]
    FfmpegError {
        status: ExitStatus,
    },
}
