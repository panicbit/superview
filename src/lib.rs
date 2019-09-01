#[macro_use] extern crate serde_derive;
#[macro_use] extern crate snafu;
extern crate serde_json as json;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::process::Command;
use std::string;
use std::io::{self, Write, BufWriter};
use snafu::ResultExt;
use tempfile::NamedTempFile;

mod probe;
use probe::probe;

mod reencode;
use reencode::reencode;

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
    #[snafu(display("Could not create filter file: {}", source))]
    CreateFilter {
        source: io::Error,
    },
    #[snafu(display("Could not write to filter file: {}", source))]
    WriteFilter {
        source: io::Error,
    },
    #[snafu(display("Could not write to filter file: {}", source))]
    FlushFilter {
        source: io::IntoInnerError<BufWriter<NamedTempFile>>,
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

    let (x_filt, y_filt) = generate_filters(filter_config)?;

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

struct FilterConfig {
    width: u32,
    height: u32,
    target_width: u32,
}

fn generate_filters(config: FilterConfig) -> Result<(NamedTempFile, NamedTempFile)> {
    let FilterConfig { width, height, target_width } = config;
    let x_filt = NamedTempFile::new().context(CreateFilter)?;
    let y_filt = NamedTempFile::new().context(CreateFilter)?;
    let mut x_filt = BufWriter::new(x_filt);
    let mut y_filt = BufWriter::new(y_filt);

    write!(x_filt, "P2 {} {} 65535\n", target_width, height).context(WriteFilter)?;
    write!(y_filt, "P2 {} {} 65535\n", target_width, height).context(WriteFilter)?;

    for y in 0..height {
        for x in 0..target_width {
            let x = x as f64;
            let tx = (x / target_width as f64 - 0.5) * 2.;
            let sx = x - (target_width - width) as f64 / 2.;
            let mut offset = tx.powi(2) * ((target_width - width) as f64 / 2.);

            if tx < 0. {
                offset *= -1.;
            }

            write!(x_filt, "{} ", (sx - offset) as i32).context(WriteFilter)?;
            write!(y_filt, "{} ", y).context(WriteFilter)?;
        }

        write!(x_filt, "\n").context(WriteFilter)?;
        write!(y_filt, "\n").context(WriteFilter)?;
    }

    let x_filt = x_filt.into_inner().context(FlushFilter)?;
    let y_filt = y_filt.into_inner().context(FlushFilter)?;

    Ok((x_filt, y_filt))
}

fn show_codec_support() -> Result {
    let output = Command::new("ffmpeg").arg("-codecs").output()
        .context(GetCodecs)?;
    let stdout = String::from_utf8(output.stdout)
        .context(InvalidUtf8 { what: "codec list" })?;
    
    eprintln!("H.264 supported: {}", stdout.contains("H.264"));
    eprintln!("H.265 supported: {}", stdout.contains("H.265"));

    Ok(())
}
