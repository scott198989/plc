#![forbid(unsafe_code)]

use std::io::{Read as _, Write as _};
use std::path::Path;

use phase2_independent_replay_verifier::verify_project_bytes;

const MAX_PROJECT_BYTES: u64 = 32 * 1024 * 1024;
const MAX_CLAIM_BYTES: u64 = 64 * 1024;

fn main() -> Result<(), String> {
    run()
}

fn run() -> Result<(), String> {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [mode, project] if mode == "derive" => {
            let bytes = read_bounded(Path::new(project), MAX_PROJECT_BYTES, "project")?;
            let result = verify_project_bytes(&bytes).map_err(|error| error.to_string())?;
            std::io::stdout()
                .write_all(&result.to_canonical_json())
                .map_err(|error| format!("canonical result write failed: {error}"))?;
            Ok(())
        }
        [mode, project, claim] if mode == "check-claim" => {
            let bytes = read_bounded(Path::new(project), MAX_PROJECT_BYTES, "project")?;
            let claimed = read_bounded(Path::new(claim), MAX_CLAIM_BYTES, "claim")?;
            let result = verify_project_bytes(&bytes).map_err(|error| error.to_string())?;
            result
                .verify_exact_claim(&claimed)
                .map_err(|error| error.to_string())?;
            std::io::stdout()
                .write_all(&result.to_canonical_json())
                .map_err(|error| format!("canonical result write failed: {error}"))?;
            Ok(())
        }
        _ => Err("usage: phase2-independent-replay-verifier derive <project.vlabproj> | check-claim <project.vlabproj> <canonical-result.json>".to_owned()),
    }
}

fn read_bounded(path: &Path, maximum: u64, label: &str) -> Result<Vec<u8>, String> {
    let file = std::fs::File::open(path)
        .map_err(|error| format!("{label} input could not be opened: {error}"))?;
    let declared = file
        .metadata()
        .map_err(|error| format!("{label} metadata is unavailable: {error}"))?
        .len();
    if declared == 0 || declared > maximum {
        return Err(format!("{label} input is outside its fixed byte limit"));
    }
    let capacity = usize::try_from(declared)
        .map_err(|_| format!("{label} input length is not representable"))?;
    let mut bytes = Vec::with_capacity(capacity);
    file.take(maximum + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("{label} input read failed: {error}"))?;
    if bytes.len() as u64 != declared || bytes.is_empty() || bytes.len() as u64 > maximum {
        return Err(format!("{label} input changed during its bounded read"));
    }
    Ok(bytes)
}
