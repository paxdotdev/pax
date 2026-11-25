use std::{env, fs, process::ExitCode};
fn main() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let mut paths = env::args().skip(1);
    let input = fs::read_to_string(
        paths
            .next()
            .ok_or("expected input trace and output JSON paths")?,
    )?;
    let output = paths.next().ok_or("missing output JSON path")?;
    let result = pax_runtime::engine::selection_policy::replay_capture(&input)?;
    let ok = result["mismatches"].as_array().is_some_and(Vec::is_empty)
        && result["geometryDifferences"] == 0;
    fs::write(output, serde_json::to_vec_pretty(&result)?)?;
    Ok(if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}
