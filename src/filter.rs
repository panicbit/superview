use std::io::{self, Write, BufWriter};
use snafu::ResultExt;
use tempfile::NamedTempFile;

pub struct FilterConfig {
    pub width: u32,
    pub height: u32,
    pub target_width: u32,
}

pub fn generate(config: FilterConfig) -> Result<(NamedTempFile, NamedTempFile), Error> {
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

#[derive(Debug, Snafu)]
pub enum Error {
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
}
