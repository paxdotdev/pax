use color_eyre::eyre::{eyre, Result, WrapErr};
use pax_compiler::{HotReloadMode, RunContext, RunTarget};
use std::{
    env,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[cfg(bundled_portal_agent)]
fn bundled_agent(home: &Path) -> Result<PathBuf> {
    use std::{
        fs,
        io::Write,
        os::unix::fs::{DirBuilderExt, PermissionsExt},
    };
    let dir = home
        .join("Library/Caches/dev.pax.portal/agents")
        .join(env!("PAX_BUNDLED_AGENT_DIGEST"));
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&dir)?;
    let path = dir.join("pax-portal-agent");
    if !path.is_file() {
        let mut staged = tempfile::NamedTempFile::new_in(dir)?;
        staged.write_all(include_bytes!(env!("PAX_BUNDLED_AGENT_PATH")))?;
        staged
            .as_file()
            .set_permissions(fs::Permissions::from_mode(0o700))?;
        staged.persist(&path)?;
    }
    Ok(path)
}

fn agent(cli: &Path) -> Result<PathBuf> {
    let home =
        PathBuf::from(env::var_os("HOME").ok_or_else(|| eyre!("Missing user home directory"))?);
    if let Some(path) = env::var_os("PAX_PORTAL_AGENT") {
        return PathBuf::from(path)
            .canonicalize()
            .wrap_err("Locate PAX_PORTAL_AGENT");
    }
    if let Some(sibling) = cli
        .parent()
        .map(|p| p.join("pax-portal-agent"))
        .filter(|p| p.is_file())
    {
        return Ok(sibling);
    }
    #[cfg(bundled_portal_agent)]
    {
        bundled_agent(&home)
    }
    #[cfg(not(bundled_portal_agent))]
    {
        let installed=home.join("Library/Application Support/dev.pax.portal/Pax Portal Agent.app/Contents/MacOS/Pax Portal Agent");
        if installed.is_file()
            && Command::new(&installed)
                .arg("--pax-cli-protocol")
                .output()
                .is_ok_and(|r| r.status.success() && r.stdout == b"1\n")
        {
            return Ok(installed);
        }
        Err(eyre!("This source-built CLI needs the matching Mac CLI bundle or PAX_PORTAL_AGENT pointing to its headless companion. Use pax-cli run --fg for a direct run."))
    }
}

/// The private stdin pipe follows the public process through Ctrl-C, SIGKILL and terminal exits.
pub(super) fn run(ctx: &RunContext, background: bool, libdev: Option<bool>) -> Result<()> {
    let cli = env::current_exe()?.canonicalize()?;
    let root = ctx.project_path.canonicalize()?;
    let mode = pax_compiler::development_reload_mode(ctx)?;
    if mode == HotReloadMode::All {
        eprintln!("Pax: --hot-reload=all now enables template reload only.");
    }
    let (target, device) = match ctx.target {
        RunTarget::Web => ("web".to_owned(), None),
        RunTarget::macOS => ("macos".to_owned(), None),
        RunTarget::iOS | RunTarget::iPadOS => {
            let (variant, id) = pax_compiler::managed_mobile_selection(ctx)?;
            (variant, Some(id))
        }
        _ => unreachable!("unsupported managed target"),
    };
    let opts = serde_json::json!({"cli":cli,"root":root,"target":target,"device":device,"port":ctx.port,
        "background":background,"libdev":libdev,"reload":if mode.reloads_pax() {"pax"}else{"off"},"verbose":ctx.verbose,"team":ctx.ios_development_team});
    let helper = agent(&cli)?;
    let mut child = Command::new(&helper)
        .arg("--pax-cli-run-v1")
        .arg(serde_json::to_string(&opts)?)
        .env("PAX_PORTAL_CALLER_PIPE", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .wrap_err_with(|| {
            format!(
                "Start Pax Portal Agent at {}. Use --fg to run directly.",
                helper.display()
            )
        })?;
    // Child::wait closes stdin. Hold the writer until all receipts have been read.
    let lifetime = child.stdin.take();
    let receipts = BufReader::new(child.stdout.take().unwrap());
    let mut observer = ctx.lifecycle_observer.as_ref();
    for receipt in receipts.lines() {
        writeln!(std::io::stdout().lock(), "{}", receipt?)?;
        if let Some(observer) = observer.take() {
            observer.run_ready(ctx.target.clone());
        }
    }
    drop(lifetime);
    let status = child.wait()?;
    if !status.success() {
        return Err(eyre!("Pax Portal Agent exited with {status}. See the diagnostic above. The direct runner is available with --fg."));
    }
    Ok(())
}

pub(super) fn stop(root: &Path) -> Result<()> {
    let helper = agent(&env::current_exe()?)?;
    let result = Command::new(helper)
        .arg("--pax-cli-stop-v1")
        .arg(root.canonicalize()?)
        .status()?;
    if !result.success() {
        return Err(eyre!("Stopping the managed project failed ({result})"));
    }
    Ok(())
}
