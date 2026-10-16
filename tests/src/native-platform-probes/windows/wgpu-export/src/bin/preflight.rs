use anyhow::{bail, Context, Result};
use std::{fs::OpenOptions, io::Write, path::PathBuf, process::ExitCode};

const USAGE: &str = "pax-wgpu-preflight [--allow-software-diagnostic] [--report <new-path>]";

fn run() -> Result<bool> {
    let mut arguments = std::env::args_os().skip(1);
    let mut allow_software = false;
    let mut report_path = None;
    while let Some(argument) = arguments.next() {
        if argument == "--allow-software-diagnostic" && !allow_software {
            allow_software = true;
        } else if argument == "--report" && report_path.is_none() {
            report_path = Some(PathBuf::from(
                arguments.next().context("--report needs a path")?,
            ));
        } else if argument == "--help" {
            println!("{USAGE}\nStreams JSON events to stdout. Report path must not already exist.\nUse check-capabilities.ps1 or an external process supervisor for a deadline.");
            return Ok(true);
        } else {
            bail!("unknown or repeated argument: {argument:?}\nusage: {USAGE}");
        }
    }

    // Reserve before driver initialization. A crash leaves an empty report, and
    // an old success can never be overwritten or mistaken for this invocation.
    let mut report = report_path
        .map(|path| {
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .with_context(|| format!("cannot reserve new report {}", path.display()))
        })
        .transpose()?;
    let result = pax_windows_wgpu_export_probe::preflight::run(allow_software);
    if let Some(output) = &mut report {
        serde_json::to_writer_pretty(&mut *output, &result)?;
        writeln!(output)?;
        output.flush()?;
    }
    Ok(result["passed"] == true)
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("{error:#}");
            ExitCode::from(2)
        }
    }
}
