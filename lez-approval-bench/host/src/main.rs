use anyhow::{bail, ensure, Context, Result};
use approval_core::{fixtures, verify, Input, VerifiedApprovalSet};
use approval_methods::{
    APPROVAL_GUEST_ELF, APPROVAL_GUEST_ID, APPROVAL_VALIDATION_ELF, APPROVAL_VALIDATION_ID,
};
use risc0_zkvm::{
    compute_image_id, Executor, ExecutorEnv, ExternalProver, InnerReceipt, Prover, ProverOpts,
    Receipt,
};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

fn validate_accelerated_guest(prover: &ExternalProver, out: &Path) -> Result<()> {
    let mut cases: Vec<(String, Input)> = vec![("valid_5_of_7".into(), fixtures::make(5, 7))];
    for mutation in 0..20 {
        let mut input = fixtures::make(5, 7);
        let name = match mutation {
            0 => {
                input.approvals.pop();
                "insufficient_approvals"
            }
            1 => {
                input.approvals[4] = input.approvals[0].clone();
                "duplicate_approval"
            }
            2 => {
                input.approvals[4].member_index = 5;
                "wrong_signer"
            }
            3 => {
                input.configuration.epoch += 1;
                "stale_epoch"
            }
            4 => {
                input
                    .action
                    .instruction
                    .resize(approval_core::MAX_ACTION_BYTES + 1, 0);
                "oversized_action"
            }
            5 => {
                input.configuration.member_keys[6] = input.configuration.member_keys[0];
                "duplicate_key"
            }
            6 => {
                input.configuration.member_keys[6] = [255; 32];
                "invalid_key"
            }
            7 => {
                input.approvals[0].signature.pop();
                "short_signature"
            }
            8 => {
                input.approvals[0].member_index = 7;
                "invalid_index"
            }
            9 => {
                input.approvals.reverse();
                "reversed_approvals"
            }
            10 => {
                input.action.network_id[0] ^= 1;
                "changed_network"
            }
            11 => {
                input.action.instance_id[0] ^= 1;
                "changed_instance"
            }
            12 => {
                input.action.program_id[0] ^= 1;
                "changed_program"
            }
            13 => {
                input.action.proposal_id[0] ^= 1;
                "changed_proposal"
            }
            14 => {
                input.action.target_program[0] ^= 1;
                "changed_target"
            }
            15 => {
                input.action.expires_at += 1;
                "changed_expiry"
            }
            16 => {
                input.action.instruction.push(1);
                "changed_instruction"
            }
            17 => {
                input.configuration.threshold = 4;
                "changed_threshold"
            }
            18 => {
                input.configuration.threshold = 0;
                "zero_threshold"
            }
            _ => {
                input.configuration.threshold = 8;
                "threshold_exceeds_members"
            }
        };
        cases.push((name.into(), input));
    }
    let expected: Vec<_> = cases
        .iter()
        .map(|(_, input)| (input.clone(), format!("{:?}", verify(input))))
        .collect();
    let env = ExecutorEnv::builder()
        .write(&expected)?
        .segment_limit_po2(18)
        .session_limit(Some(64 * 1024 * 1024))
        .build()?;
    let started = Instant::now();
    let session = prover.execute(env, APPROVAL_VALIDATION_ELF)?;
    ensure!(
        matches!(session.exit_code, risc0_zkvm::ExitCode::Halted(0)),
        "validation guest failed"
    );
    let (case_count, vector_count): (u32, u32) = session.journal.decode()?;
    ensure!(
        case_count == cases.len() as u32 && vector_count == 15,
        "validation count mismatch"
    );
    let report = serde_json::json!({
        "status": "accelerated_guest_validation_passed", "is_proving_evidence": false,
        "approval_cases": case_count, "published_bip340_32_byte_vectors": vector_count,
        "host_backend": "upstream crates.io", "guest_backend": "pinned official RISC Zero patches",
        "case_names": cases.iter().map(|(name, _)| name).collect::<Vec<_>>(),
        "validation_image_id_words_u32": APPROVAL_VALIDATION_ID,
        "user_cycles": session.cycles(), "execution_wall_ms": started.elapsed().as_secs_f64() * 1000.0,
    });
    let path = out.with_extension("validation.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(&report)? + "\n")?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn main() -> Result<()> {
    let mut m: u32 = 5;
    let mut n: usize = 7;
    let mut out = PathBuf::from("reports/m5-n7.json");
    let mut r0vm = PathBuf::from("r0vm");
    let mut receipt_kind = "composite".to_owned();
    let mut verify_receipt: Option<PathBuf> = None;
    let mut guest_program: Option<PathBuf> = None;
    let mut mode = "prove".to_owned();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let value = args.next().context("every option requires a value")?;
        match arg.as_str() {
            "--m" => m = value.parse()?,
            "--n" => n = value.parse()?,
            "--out" => out = value.into(),
            "--r0vm" => r0vm = value.into(),
            "--receipt" => receipt_kind = value,
            "--verify-receipt" => verify_receipt = Some(value.into()),
            "--guest-program" => guest_program = Some(value.into()),
            "--mode" => mode = value,
            _ => bail!("unknown option {arg}"),
        }
    }
    ensure!(
        m > 0 && m as usize <= n && n <= approval_core::MAX_MEMBERS,
        "invalid M/N"
    );
    if let Ok(value) = std::env::var("RISC0_DEV_MODE") {
        ensure!(
            value == "0" || value.eq_ignore_ascii_case("false"),
            "development mode refused"
        );
    }

    let input = fixtures::make(m, n);
    let expected = verify(&input).map_err(|e| anyhow::anyhow!("native control: {e:?}"))?;
    if let Some(path) = verify_receipt {
        ensure!(
            fs::metadata(&path)?.len() <= 100 * 1024 * 1024,
            "receipt exceeds benchmark file limit"
        );
        let receipt: Receipt = bincode::deserialize(&fs::read(&path)?)?;
        ensure!(
            !matches!(&receipt.inner, InnerReceipt::Fake(_)),
            "fake receipt refused"
        );
        let expected_id = match &guest_program {
            Some(program) => compute_image_id(&fs::read(program)?)?,
            None => APPROVAL_GUEST_ID.into(),
        };
        receipt.verify(expected_id)?;
        let journal: VerifiedApprovalSet = receipt.journal.decode()?;
        ensure!(
            journal == expected,
            "journal does not match requested M/N fixture"
        );
        if mode == "check-receipt" {
            // Exercise integrity verification using the decoded receipt API,
            // without assumptions about its serialized byte layout.
            let mut modified = receipt.clone();
            ensure!(!modified.journal.bytes.is_empty(), "journal is empty");
            modified.journal.bytes[0] ^= 1;
            let error = modified
                .verify(expected_id)
                .err()
                .context("modified journal unexpectedly verified")?;
            println!(
                "{}",
                serde_json::json!({
                    "status": "saved_receipt_integrity_checked",
                    "original_receipt_verified": true,
                    "modified_journal_rejected": true,
                    "verification_error": error.to_string(),
                    "original_file_modified": false,
                })
            );
            return Ok(());
        }
        println!(
            "{}",
            serde_json::json!({
                "status": "saved_receipt_verified", "dev_mode": false,
                "m": m, "n": n, "signer_indices": journal.signer_indices,
                "image_id_words_u32": expected_id.as_words(),
            })
        );
        return Ok(());
    }
    ensure!(
        guest_program.is_none(),
        "--guest-program is only for saved-receipt verification"
    );
    let input_words = risc0_zkvm::serde::to_vec(&input)?;
    ensure!(
        mode == "prove" || mode == "execute" || mode == "validate",
        "mode must be prove, execute, or validate"
    );
    let prover = ExternalProver::new("local-r0vm", &r0vm);
    if mode == "validate" {
        return validate_accelerated_guest(&prover, &out);
    }
    let environment = ExecutorEnv::builder()
        .write(&input)?
        .segment_limit_po2(18)
        .session_limit(Some(64 * 1024 * 1024))
        .build()?;
    if mode == "execute" {
        eprintln!("Executing guest only, without proving: M={m}, N={n}");
        let started = Instant::now();
        let session = prover.execute(environment, APPROVAL_GUEST_ELF)?;
        let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
        ensure!(
            matches!(session.exit_code, risc0_zkvm::ExitCode::Halted(0)),
            "guest did not halt successfully"
        );
        let actual: VerifiedApprovalSet = session.journal.decode()?;
        ensure!(actual == expected, "execution journal differs from fixture");
        let report = serde_json::json!({
            "status": "executed_without_proof", "is_proving_evidence": false,
            "m": m, "n": n, "user_cycles": session.cycles(),
            "segments": session.segments.len(), "execution_wall_ms": elapsed_ms,
            "image_id_words_u32": APPROVAL_GUEST_ID,
            "segment_cycles": session.segments.iter().map(|s| s.cycles).collect::<Vec<_>>(),
            "segment_po2": session.segments.iter().map(|s| s.po2).collect::<Vec<_>>(),
        });
        let path = out.with_extension("execution.json");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_string_pretty(&report)? + "\n")?;
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    let opts = match receipt_kind.as_str() {
        "composite" => ProverOpts::composite(),
        "succinct" => ProverOpts::succinct(),
        _ => bail!("receipt kind must be composite or succinct"),
    }
    .with_dev_mode(false);
    eprintln!("Starting REAL local proof: M={m}, N={n}, receipt={receipt_kind}");
    let started = Instant::now();
    let info = prover.prove_with_opts(environment, APPROVAL_GUEST_ELF, &opts)?;
    let proving_ms = started.elapsed().as_secs_f64() * 1000.0;
    ensure!(
        !matches!(&info.receipt.inner, InnerReceipt::Fake(_)),
        "fake receipt refused"
    );
    let verification_started = Instant::now();
    info.receipt.verify(APPROVAL_GUEST_ID)?;
    let verification_ms = verification_started.elapsed().as_secs_f64() * 1000.0;
    let actual: VerifiedApprovalSet = info.receipt.journal.decode()?;
    ensure!(
        actual == expected,
        "journal differs from independently checked fixture"
    );
    let actual_kind = match &info.receipt.inner {
        InnerReceipt::Composite(_) => "composite",
        InnerReceipt::Succinct(_) => "succinct",
        InnerReceipt::Groth16(_) => "groth16",
        _ => "other",
    };
    ensure!(
        actual_kind == receipt_kind,
        "receipt kind differs from request"
    );
    let encoded = bincode::serialize(&info.receipt)?;
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    let receipt_path = out.with_extension("receipt");
    fs::write(&receipt_path, &encoded)?;
    let report = serde_json::json!({
        "status": "verified_real_proof",
        "scope": "standalone_approval_verifier_guest",
        "full_lez_private_transaction": false,
        "lez_reference_commit": "411adc8fdb3f4c3af64354c4fc26d3e4a6a3d3f4",
        "risc0_zkvm": "3.0.5", "k256": "0.13.4", "rust_host": "1.98.1",
        "guest_crypto_backend": "official RISC Zero acceleration patches; exact commits in guest Cargo.lock",
        "m": m, "n": n, "verified_approvals": actual.signer_indices.len(),
        "dev_mode": false, "receipt_kind": actual_kind,
        "image_id_words_u32": APPROVAL_GUEST_ID,
        "guest_elf_bytes": APPROVAL_GUEST_ELF.len(),
        "serialized_guest_input_bytes": input_words.len() * 4,
        "receipt_bytes_bincode_1_3_3": encoded.len(),
        "journal_bytes": info.receipt.journal.bytes.len(),
        "proving_wall_ms": proving_ms, "verification_wall_ms": verification_ms,
        "segment_limit_po2": 18, "stats": info.stats,
        "configuration_digest": hex::encode(actual.configuration_digest),
        "action_digest": hex::encode(actual.action_digest),
        "signer_indices": actual.signer_indices,
        "receipt_file": receipt_path.file_name().unwrap().to_string_lossy(),
        "fixtures": "public deterministic keys; synthetic action; no network transaction",
        "not_measured": ["LEZ private-account wrapper", "LEZ transaction size", "devnet block limits", "current-member account binding", "expiry enforcement", "replay storage", "custody", "coordination", "selective disclosure"]
    });
    fs::write(&out, serde_json::to_string_pretty(&report)? + "\n")?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
