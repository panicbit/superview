#[macro_use] extern crate serde_derive;
#[macro_use] extern crate snafu;
extern crate serde_json as json;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::process::Command;
use std::string;
use std::io;
use snafu::ResultExt;

mod probe;
use probe::probe;

mod reencode;
use reencode::reencode;

mod filter;
use filter::FilterConfig;

#[cfg(target_os = "windows")]
const FFMPEG_EXE: &str = "ffmpeg.exe";
#[cfg(not(target_os = "windows"))]
const FFMPEG_EXE: &str = "ffmpeg";

pub type Result<T = (), E = Error> = std::result::Result<T, E>;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Could not open input {}: {}", input.display(), source))]
    OpenInput {
        input: PathBuf,
        source: io::Error,
    },
    #[snafu(display("Could not get ffmpeg codec list: {}", source))]
    GetCodecs {
        source: io::Error,
    },
    #[snafu(display("Invalid UTF-8: {}", what))]
    InvalidUtf8 {
        what: String,
        source: string::FromUtf8Error,
    },
    #[snafu(display("Could not probe '{}': {}", input.display(), source))]
    ProbeInput {
        input: PathBuf,
        source: probe::Error,
    },
    #[snafu(display("Could not generate filter: {}", source))]
    GenerateFilter {
        source: filter::Error,
    },
    #[snafu(display("Could not reencode: {}", source))]
    Reencode {
        source: reencode::Error
    }
}

pub fn superview(input: &Path, output: &Path, bitrate: Option<u32>) -> Result {
    // Check the input file can be opened successfully
    File::open(input)
        .context(OpenInput { input })?;

    show_codec_support()?;

    let specs = probe(input).context(ProbeInput { input })?;
    let stream = &specs.streams[0];
    let bitrate = bitrate.unwrap_or(stream.bitrate);
    let filter_config = FilterConfig {
        width: stream.width,
        height: stream.height,
        target_width: (stream.width as f64 / (4. / 3.) * (16. / 9.)) as u32 / 2 * 2,
    };

    eprintln!("Scaling input file {} (codec: {}, duration: {} secs) from {}*{} to {}*{} using superview scaling",
        input.display(),
        stream.codec_name,
        stream.duration as u32,
        filter_config.width,
        filter_config.height,
        filter_config.target_width,
        filter_config.height,
    );

    let (x_filt, y_filt) = filter::generate(filter_config).context(GenerateFilter)?;

    eprintln!("Filter files generated, re-encoding video at bitrate {:.1} MB/s", bitrate as f64 / 1024. / 1024.);

    reencode(reencode::Config {
        input,
        output,
        bitrate,
        codec_name: &stream.codec_name,
        duration: stream.duration,
        x_filt: x_filt.path(),
        y_filt: y_filt.path(),
    }).context(Reencode)?;

    Ok(())
}

fn show_codec_support() -> Result {
    let output = Command::new(FFMPEG_EXE).arg("-codecs").output()
        .context(GetCodecs)?;
    let stdout = String::from_utf8(output.stdout)
        .context(InvalidUtf8 { what: "codec list" })?;
    
    eprintln!("H.264 supported: {}", stdout.contains("H.264"));
    eprintln!("H.265 supported: {}", stdout.contains("H.265"));

    Ok(())
}
