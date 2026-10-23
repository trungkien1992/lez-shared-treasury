mod fixture;
mod measured;
mod validation;

use anyhow::{Context, Result, bail, ensure};
use lee::{
    PrivacyPreservingTransaction,
    privacy_preserving_transaction::{circuit::Proof, message::Message, witness_set::WitnessSet},
    program::Program,
};
use risc0_zkvm::{ExecutorEnv, Receipt};
use serde_json::{Value, json};
use std::{borrow::Cow, fs, path::PathBuf, time::Instant};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    ensure!(
        args.len() == 4,
        "usage: vault-host execute|prove|succinct|check|check-succinct|validate M REPORT_DIR"
    );
    let mode = &args[1];
    let m: u32 = args[2].parse()?;
    let dir = PathBuf::from(&args[3]);
    fs::create_dir_all(&dir)?;
    ensure!(
        std::env::var("RISC0_DEV_MODE").unwrap_or_else(|_| "0".into()) == "0",
        "development mode forbidden"
    );
    let program = Program::new(Cow::Borrowed(vault_methods::VAULT_GUEST_ELF))?;
    let fixture = fixture::make(&program, m)?;
    if mode == "validate" {
        let result = validation::run(&program, &fixture)?;
        measured::write_json(&dir.join("validation.json"), &result)?;
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }
    if mode == "check" || mode == "check-succinct" {
        let format = if mode == "check" {
            "composite"
        } else {
            "succinct"
        };
        let bytes = fs::read(dir.join(format!("transaction-{format}.borsh")))?;
        let tx: PrivacyPreservingTransaction = borsh::from_slice(&bytes)?;
        let receipt: Receipt =
            bincode::deserialize(&fs::read(dir.join(format!("ppe-{format}.receipt")))?)?;
        measured::reject_fake(&receipt)?;
        receipt.verify(lee::PRIVACY_PRESERVING_CIRCUIT_ID)?;
        let output: lee_core::PrivacyPreservingCircuitOutput = borsh::from_slice(
            lee_core::from_frame(&receipt.journal.bytes).context("invalid circuit journal")?,
        )?;
        fixture.check_output(&output)?;
        ensure!(
            Message::from_circuit_output(vec![], output) == tx.message,
            "receipt/transaction mismatch"
        );
        let result = check_transaction(&fixture, &tx)?;
        measured::write_json(&dir.join(format!("fresh-check-{format}.json")), &result)?;
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }
    ensure!(
        ["execute", "prove", "succinct"].contains(&mode.as_str()),
        "unknown mode"
    );
    let started = Instant::now();
    let mut runner = measured::Runner {
        prove: mode != "execute",
        dir: dir.clone(),
        stages: vec![],
        assumptions: ExecutorEnv::builder(),
    };
    let (output, receipt) = measured::transaction_output(
        fixture.root()?,
        fixture.witnesses.clone(),
        &program,
        &mut runner,
    )?;
    fixture.check_output(&output)?;
    let mut report = json!({
        "mode": mode, "threshold":m,"members":7,"upstream_commit":"411adc8fdb3f4c3af64354c4fc26d3e4a6a3d3f4",
        "guest_image_id": program.id(), "guest_sha256":measured::sha(program.elf()),
        "ppe_image_id":lee::PRIVACY_PRESERVING_CIRCUIT_ID,
        "ppe_elf_sha256":measured::sha(lee::PRIVACY_PRESERVING_CIRCUIT_ELF),
        "segment_limit_po2":measured::SEGMENT_PO2,"session_limit_cycles":measured::SESSION_LIMIT,
        "private_notes":2,"dummy_notes":0,"ciphertext_padding":1024,
        "program_identity":"shadow","member_account_ids":fixture.member_ids.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "post_state_checks":"both encrypted notes decrypt to exact expected accounts; vault 1000→963, recipient 0→37, proposal counter 0→1",
        "stages":runner.stages,
        "limitations":["local LEE state transition, no RPC/sequencer/Bedrock deployment", "preinitialized private vault genesis fixture", "no enrollment or reconfiguration workflow", "two notes; no wallet seven-note padding", "synthetic network domain and public deterministic test keys"]
    });
    if let Some(receipt) = receipt {
        save_transaction(&fixture, &output, &receipt, &dir, "composite", &mut report)?;
        // Preserve a complete, verified transaction before optional compression.
        report["wall_seconds_this_invocation"] = json!(started.elapsed().as_secs_f64());
        measured::write_json(&dir.join("prove.json"), &report)?;
        if mode == "succinct" {
            let (compressed, metadata) = measured::compress(&receipt, &dir)?;
            report["compression"] = metadata;
            save_transaction(
                &fixture,
                &output,
                &compressed,
                &dir,
                "succinct",
                &mut report,
            )?;
        }
    }
    report["wall_seconds_this_invocation"] = json!(started.elapsed().as_secs_f64());
    measured::write_json(&dir.join(format!("{mode}.json")), &report)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn save_transaction(
    fixture: &fixture::Fixture,
    output: &lee_core::PrivacyPreservingCircuitOutput,
    receipt: &Receipt,
    dir: &std::path::Path,
    format: &str,
    report: &mut Value,
) -> Result<()> {
    let proof = Proof::from_inner(borsh::to_vec(&receipt.inner)?);
    let message = Message::from_circuit_output(vec![], borsh::from_slice(&borsh::to_vec(output)?)?);
    let witness = WitnessSet::for_message(&message, proof, &[]);
    let tx = PrivacyPreservingTransaction::new(message, witness);
    let checks = check_transaction(fixture, &tx)?;
    let borsh_bytes = borsh::to_vec(&tx)?;
    ensure!(
        borsh::from_slice::<PrivacyPreservingTransaction>(&borsh_bytes)? == tx,
        "Borsh roundtrip failed"
    );
    let core_bytes = tx.to_bytes();
    ensure!(
        PrivacyPreservingTransaction::from_bytes(&core_bytes)? == tx,
        "core encoding roundtrip failed"
    );
    fs::write(
        dir.join(format!("transaction-{format}.borsh")),
        &borsh_bytes,
    )?;
    fs::write(dir.join(format!("transaction-{format}.core")), &core_bytes)?;
    // The outer common::LeeTransaction wire enum is one Borsh u8 discriminant
    // (1 = PrivacyPreserving) followed by this exact Borsh payload. We do not
    // import common, which pulls unrelated Bedrock networking dependencies.
    let mut envelope = vec![1u8];
    envelope.extend_from_slice(&borsh_bytes);
    fs::write(
        dir.join(format!("transaction-{format}.lee-envelope")),
        &envelope,
    )?;
    report[format] = json!({"checks":checks,"inner_proof_bytes":tx.witness_set.proof().clone().into_inner().len(),
        "core_transaction_bytes":core_bytes.len(), "borsh_transaction_bytes":borsh_bytes.len(),
        "lee_envelope_bytes":envelope.len(),"envelope_encoding":"pinned common::LeeTransaction layout; no RPC test",
        "borsh_sha256":measured::sha(&borsh_bytes), "envelope_sha256":measured::sha(&envelope),
        "fits_debug_1mib_minus_2048":envelope.len() <= 1024*1024-2048});
    Ok(())
}

fn check_transaction(
    fixture: &fixture::Fixture,
    tx: &PrivacyPreservingTransaction,
) -> Result<Value> {
    let before = borsh::to_vec(&fixture.state)?;
    let mut expired_state = fixture.state.clone();
    let expired = expired_state
        .transition_from_privacy_preserving_transaction(tx, 10, fixture::EXPIRES)
        .expect_err("transaction must expire at its exclusive upper bound");
    ensure!(
        matches!(expired, lee::error::LeeError::OutOfValidityWindow),
        "unexpected expiry error: {expired:?}"
    );
    ensure!(
        borsh::to_vec(&expired_state)? == before,
        "expiry failure changed state"
    );
    let mut state = fixture.state.clone();
    let start = Instant::now();
    state.transition_from_privacy_preserving_transaction(tx, 10, fixture::EXPIRES - 1)?;
    let validation_seconds = start.elapsed().as_secs_f64();
    ensure!(
        borsh::to_vec(&state)? != before,
        "accepted transaction did not update state"
    );
    for commitment in tx.message.commitments() {
        ensure!(
            state.get_proof_for_commitment(&commitment).is_some(),
            "post-state commitment missing"
        );
    }
    let accepted = borsh::to_vec(&state)?;
    let replay = state
        .transition_from_privacy_preserving_transaction(tx, 11, fixture::EXPIRES - 1)
        .expect_err("replayed transaction must fail");
    ensure!(
        borsh::to_vec(&state)? == accepted,
        "replay failure changed state"
    );
    let mut altered = tx.clone();
    altered.message.timestamp_validity_window = (..fixture::EXPIRES + 1).into();
    let mut fresh = fixture.state.clone();
    let tamper = fresh
        .transition_from_privacy_preserving_transaction(&altered, 10, fixture::EXPIRES - 1)
        .expect_err("changing the proof-bound message must fail");
    ensure!(
        matches!(tamper, lee::error::LeeError::InvalidPrivacyPreservingProof),
        "unexpected tamper error: {tamper:?}"
    );
    ensure!(
        borsh::to_vec(&fresh)? == before,
        "tamper failure changed state"
    );
    if !tx.message.public_actions.is_empty() {
        bail!("public account effects unexpected");
    }
    Ok(
        json!({"valid_transition":true,"validation_seconds":validation_seconds,
        "expiry_rejected":format!("{expired:?}"),"replay_rejected":format!("{replay:?}"),
        "modified_message_rejected":format!("{tamper:?}"),"failed_checks_preserved_state":true,
        "new_commitments_present":true,"public_effects":0,"public_signatures":tx.witness_set.signatures_and_public_keys().len()}),
    )
}
