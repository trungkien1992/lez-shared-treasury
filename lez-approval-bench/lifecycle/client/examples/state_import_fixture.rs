#[path = "../tests/support/mod.rs"]
mod support;
use anyhow::{Result, ensure};
use std::{
    fs,
    io::Write,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        args.len() == 2,
        "usage: state_import_fixture NEW_REPORT_DIRECTORY NEW_TEST_NULLIFIER_KEY_FILE"
    );
    let directory = std::path::absolute(&args[0])?;
    let key_path = std::path::absolute(&args[1])?;
    ensure!(
        !directory.exists() && !key_path.exists(),
        "choose fresh output and key paths"
    );
    let project = support::root()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .canonicalize()?;
    ensure!(
        !key_path
            .parent()
            .unwrap()
            .canonicalize()?
            .starts_with(project),
        "keep the local key outside the project"
    );
    let mut fixture = support::fixture(true, true, true);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_millis()
        .try_into()?;
    fixture.checkpoint.observed_at_ms = now;
    fixture.checkpoint.valid_until_ms = now + 300_000;
    let evidence = borsh::to_vec(&fixture.evidence)?;
    let state = treasury_client::state_import::verify(
        &evidence,
        &fixture.checkpoint,
        &fixture.nullifier_key,
        now,
    )?;
    use std::os::unix::fs::DirBuilderExt;
    fs::DirBuilder::new().mode(0o700).create(&directory)?;
    write_new(&directory.join("evidence.borsh"), &evidence)?;
    write_new(
        &directory.join("trusted-checkpoint.json"),
        &serde_json::to_vec_pretty(&fixture.checkpoint)?,
    )?;
    write_new(
        &directory.join("context.json"),
        &serde_json::to_vec_pretty(state.context())?,
    )?;
    write_new(
        &directory.join("verification.json"),
        &serde_json::to_vec_pretty(&state.report()?)?,
    )?;
    write_new(&directory.join("README.txt"),b"Historical local test fixture. Checkpoint generated from a known archived state, not a live node or consensus proof. Replay uses fixture validity timestamps. Public test-only LEZ key material must never secure funds. Checkpoint expires five minutes after generation. Read client/STATE_IMPORT.md.\n")?;
    write_new(&key_path, hex::encode(fixture.nullifier_key).as_bytes())?;
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"report_directory":directory,"local_key_path":key_path,"historical_test_fixture":true,"live_node_queried":false,"verified_state":state.report()?})
        )?
    );
    Ok(())
}
