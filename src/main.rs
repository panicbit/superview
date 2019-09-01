use std::path::PathBuf;
use std::process;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "superview")]
struct Opt {
    /// The input video filename
    #[structopt(short, long)]
    input: PathBuf,
    /// The output video filename
    #[structopt(short, long, default_value = "output.mp4")]
    output: PathBuf,
    /// The bitrate in bytes/second to encode in. If not specified, take the same bitrate as the input file
    #[structopt(short, long)]
    bitrate: Option<u32>,
}

fn main() {
    let opt = Opt::from_args();

    let res = superview::superview(
        &opt.input,
        &opt.output,
        opt.bitrate,
    );

    if let Err(e) = res {
        eprintln!("Error: {}", e);
        process::exit(-1);
    }

    eprintln!("Done! You can open the output file '{}' to see the result", opt.output.display());
}
