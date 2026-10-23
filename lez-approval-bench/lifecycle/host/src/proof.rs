//! Two real private transactions, with a proof-verified LEE state handoff.
//! This local driver does not claim sequencer or public-network settlement.
use crate::{fixture, measured};
use anyhow::{Context, Result, ensure};
use lee::{
    PrivacyPreservingTransaction, V03State,
    privacy_preserving_transaction::{circuit::Proof, message::Message, witness_set::WitnessSet},
    program::Program,
};
use lee_core::PrivacyPreservingCircuitOutput;
use risc0_zkvm::{ExecutorEnv, Receipt};
use serde_json::{Value, json};
use std::{fs, path::Path, time::Instant};

pub fn run(mode: &str, program: &Program, f: &fixture::Fixture, dir: &Path) -> Result<Value> {
    let checking = matches!(mode, "check" | "check-succinct");
    let format = if matches!(mode, "succinct" | "check-succinct") {
        "succinct"
    } else {
        "composite"
    };
    let create_dir = dir.join("create");
    let spend_dir = dir.join("spend");
    fs::create_dir_all(&create_dir)?;
    fs::create_dir_all(&spend_dir)?;

    // A candidate account value alone must not be sufficient to make a spend witness.
    ensure!(
        f.spend_inputs(&f.state, f.vault_created.clone()).is_err(),
        "spend witness unexpectedly available before accepted creation"
    );
    let (created, create_receipt, create_stages) = if checking {
        let (output, receipt) = load_transaction(&create_dir, format)?;
        (output, receipt, vec![])
    } else {
        prove(f.create_root()?, f.witnesses.clone(), program, &create_dir)?
    };
    let vault_pre = f.check_created(&created)?;
    let create_receipt = select_receipt(&create_receipt, format, &create_dir, checking)?;
    let create_tx = transaction(&created, &create_receipt)?;
    let (accepted_creation, create_checks) = accept(&f.state, &create_tx, 10)?;
    let create_encoding = if checking {
        check_encoding(&create_tx, &create_dir, format)?
    } else {
        save_transaction(&create_tx, &create_dir, format)?
    };
    record_or_check_state(
        &dir.join("after-creation.state.borsh"),
        &accepted_creation,
        checking,
    )?;
    if !checking {
        measured::write_json(
            &dir.join("creation-progress.json"),
            &json!({"creation_real_proof_verified":true,"creation_transition_accepted":true,
                "spend_completed":false,"sequencer_submission":false,
                "checks":create_checks,"encoding":create_encoding,"stages":create_stages}),
        )?;
    }

    // This membership witness is read from the actual proof-accepted state.
    // There is no with_private_accounts insertion on this path.
    let (root, witnesses) = f.spend_inputs(&accepted_creation, vault_pre)?;
    let witness_bytes = borsh::to_vec(&witnesses)?;
    let witness_path = dir.join("accepted-spend-witnesses.borsh");
    if checking {
        ensure!(
            fs::read(&witness_path)? == witness_bytes,
            "saved spend witnesses changed"
        );
    } else {
        fs::write(&witness_path, &witness_bytes)?;
    }
    let (spent, spend_receipt, spend_stages) = if checking {
        let (output, receipt) = load_transaction(&spend_dir, format)?;
        (output, receipt, vec![])
    } else {
        prove(root, witnesses, program, &spend_dir)?
    };
    f.check_spent(&spent)?;
    let spend_receipt = select_receipt(&spend_receipt, format, &spend_dir, checking)?;
    let spend_tx = transaction(&spent, &spend_receipt)?;
    let (accepted_spend, spend_checks) = accept(&accepted_creation, &spend_tx, 11)?;
    let spend_encoding = if checking {
        check_encoding(&spend_tx, &spend_dir, format)?
    } else {
        save_transaction(&spend_tx, &spend_dir, format)?
    };
    record_or_check_state(
        &dir.join("after-spend.state.borsh"),
        &accepted_spend,
        checking,
    )?;
    // The same spend must not be accepted before the creation it depends on.
    let mut before_creation = f.state.clone();
    let before_bytes = borsh::to_vec(&before_creation)?;
    let premature = before_creation
        .transition_from_privacy_preserving_transaction(&spend_tx, 10, fixture::EXPIRES - 1)
        .expect_err("spend accepted before its required creation");
    ensure!(
        borsh::to_vec(&before_creation)? == before_bytes,
        "premature spend changed state"
    );

    Ok(json!({
        "mode":mode,"proof_format":format,"unanimous_enrollment":f.create.descriptor.member_keys.len(),
        "spending_threshold":f.create.descriptor.threshold,
        "creation":{"funder_before":1000,"funder_after":500,"vault_after":500,
            "checks":create_checks,"encoding":create_encoding,"stages":create_stages},
        "spend":{"vault_before":500,"vault_after":463,"recipient_after":37,"next_proposal":1,
            "checks":spend_checks,"encoding":spend_encoding,"stages":spend_stages},
        "proof_generated":!checking,"real_receipts_verified":2,
        "fresh_process_check":checking,"all_four_notes_decrypted_and_checked":true,
        "local_lee_transitions_verified":2,"state_carry":"proof-accepted LEE state and its membership witness",
        "spend_witness_absent_before_creation":true,"spend_before_creation_rejected":format!("{premature:?}"),
        "accepted_spend_witness_sha256":measured::sha(&witness_bytes),
        "after_creation_commitment_root":accepted_creation.commitment_root(),
        "after_spend_commitment_root":accepted_spend.commitment_root(),
        "created_vault_commitment":lee_core::Commitment::new(&f.vault, &f.vault_created),
        "vault_membership_after_creation":accepted_creation.get_proof_for_commitment(
            &lee_core::Commitment::new(&f.vault, &f.vault_created)),
        "creation_commitments":create_tx.message.commitments(),
        "spend_commitments":spend_tx.message.commitments(),
        "private_notes_per_transaction":2,"ciphertext_padding":1024,"dummy_notes":0,
        "vault_id":f.vault.to_string(),"funder_id":f.funder.to_string(),"recipient_id":f.recipient.to_string(),
        "sequencer_submission":false,"live_devnet_submission":false,
        "limitations":["local LEE transitions, not sequencer or network settlement",
            "test-only genesis funding commitment and public deterministic fixture keys",
            "two notes, no wallet padding benchmark","no member key sharing, rotation or production interfaces"]
    }))
}

fn prove(
    root: lee_core::execution_state::RootCall,
    witnesses: Vec<lee_core::PrivateWitness>,
    program: &Program,
    dir: &Path,
) -> Result<(PrivacyPreservingCircuitOutput, Receipt, Vec<Value>)> {
    let cached: Vec<_> = ["vault-plan", "vault-apply", "ppe-composite"].into_iter()
        .filter(|label| dir.join(format!("{label}.receipt")).exists() && dir.join(format!("{label}.json")).exists())
        .collect();
    let mut runner = measured::Runner {
        prove: true,
        dir: dir.to_path_buf(),
        stages: vec![],
        assumptions: ExecutorEnv::builder(),
    };
    let (output, receipt) = measured::transaction_output(root, witnesses, program, &mut runner)?;
    let receipt = receipt.context("real proving returned no receipt")?;
    measured::reject_fake(&receipt)?;
    receipt.verify(lee::PRIVACY_PRESERVING_CIRCUIT_ID)?;
    for metadata in &mut runner.stages {
        let reused = cached.iter().any(|label| metadata["stage"] == *label);
        metadata["checkpoint_reused"] = json!(reused);
    }
    Ok((output, receipt, runner.stages))
}

pub fn check_node_witness(f: &fixture::Fixture, dir: &Path) -> Result<Value> {
    let observation: Value = serde_json::from_slice(&fs::read(dir.join("node-creation-witness.json"))?)?;
    let (created, receipt) = load_transaction(&dir.join("create"), "succinct")?;
    let vault_pre = f.check_created(&created)?;
    let create_tx = transaction(&created, &receipt)?;
    let (accepted, _) = accept(&f.state, &create_tx, 10)?;
    let (_, mut witnesses) = f.spend_inputs(&accepted, vault_pre)?;
    let commitment = lee_core::Commitment::new(&f.vault, &f.vault_created);
    let observed_commitment: lee_core::Commitment = serde_json::from_value(observation["commitment"].clone())?;
    let membership: lee_core::MembershipProof = serde_json::from_value(observation["membership_proof"].clone())?;
    let root: lee_core::CommitmentSetDigest = serde_json::from_value(observation["commitment_root"].clone())?;
    ensure!(observed_commitment == commitment, "node witness identifies a different vault state");
    ensure!(lee_core::compute_digest_for_path(&commitment, &membership) == root,
        "node witness does not open the reported commitment root");
    ensure!(root == accepted.commitment_root(), "node root differs from the proved fixture state");
    let lee_core::NullifierWitness::Update {membership_proof, ..} = &mut witnesses[0].nullifier else {
        anyhow::bail!("expected an existing vault update witness");
    };
    *membership_proof = membership;
    let encoded = borsh::to_vec(&witnesses)?;
    ensure!(encoded == fs::read(dir.join("accepted-spend-witnesses.borsh"))?,
        "ingested node witness differs from the input used for the spend proof");
    Ok(json!({"mode":"check-node-witness","node_membership_ingested":true,
        "membership_opens_observed_root":true,"input_bytes_match_proved_spend":true,
        "witness_sha256":measured::sha(&encoded),"node_commitment_root":root,
        "observation":observation,"proof_generated":false,"live_devnet_submission":false}))
}

fn select_receipt(receipt: &Receipt, format: &str, dir: &Path, checking: bool) -> Result<Receipt> {
    if format == "succinct" && !checking {
        Ok(measured::compress(receipt, dir)?.0)
    } else {
        Ok(receipt.clone())
    }
}

fn transaction(
    output: &PrivacyPreservingCircuitOutput,
    receipt: &Receipt,
) -> Result<PrivacyPreservingTransaction> {
    measured::reject_fake(receipt)?;
    receipt.verify(lee::PRIVACY_PRESERVING_CIRCUIT_ID)?;
    let bound: PrivacyPreservingCircuitOutput = borsh::from_slice(
        lee_core::from_frame(&receipt.journal.bytes).context("invalid PPE journal frame")?,
    )?;
    ensure!(
        borsh::to_vec(&bound)? == borsh::to_vec(output)?,
        "proof/output mismatch"
    );
    let proof = Proof::from_inner(borsh::to_vec(&receipt.inner)?);
    let message = Message::from_circuit_output(vec![], bound);
    let witness = WitnessSet::for_message(&message, proof, &[]);
    Ok(PrivacyPreservingTransaction::new(message, witness))
}

fn load_transaction(dir: &Path, format: &str) -> Result<(PrivacyPreservingCircuitOutput, Receipt)> {
    let receipt: Receipt =
        bincode::deserialize(&fs::read(dir.join(format!("ppe-{format}.receipt")))?)?;
    measured::reject_fake(&receipt)?;
    receipt.verify(lee::PRIVACY_PRESERVING_CIRCUIT_ID)?;
    let output: PrivacyPreservingCircuitOutput = borsh::from_slice(
        lee_core::from_frame(&receipt.journal.bytes).context("invalid saved journal frame")?,
    )?;
    let encoded: PrivacyPreservingTransaction =
        borsh::from_slice(&fs::read(dir.join(format!("transaction-{format}.borsh")))?)?;
    ensure!(
        transaction(&output, &receipt)? == encoded,
        "saved transaction/receipt mismatch"
    );
    Ok((output, receipt))
}

fn accept(
    pre: &V03State,
    tx: &PrivacyPreservingTransaction,
    block: u64,
) -> Result<(V03State, Value)> {
    ensure!(
        tx.message.public_actions.is_empty(),
        "unexpected public effects"
    );
    ensure!(
        tx.witness_set.signatures_and_public_keys().is_empty(),
        "unexpected public signatures"
    );
    let before = borsh::to_vec(pre)?;
    let mut expired = pre.clone();
    let error = expired
        .transition_from_privacy_preserving_transaction(tx, block, fixture::EXPIRES)
        .expect_err("transaction accepted at its exclusive expiry");
    ensure!(
        matches!(error, lee::error::LeeError::OutOfValidityWindow),
        "unexpected expiry error: {error:?}"
    );
    ensure!(
        borsh::to_vec(&expired)? == before,
        "expiry failure changed state"
    );
    let mut modified = tx.clone();
    modified.message.timestamp_validity_window = (..fixture::EXPIRES + 1).into();
    let mut tampered_state = pre.clone();
    let tamper = tampered_state
        .transition_from_privacy_preserving_transaction(&modified, block, fixture::EXPIRES - 1)
        .expect_err("modified proof-bound message accepted");
    ensure!(
        matches!(tamper, lee::error::LeeError::InvalidPrivacyPreservingProof),
        "unexpected tamper error: {tamper:?}"
    );
    ensure!(
        borsh::to_vec(&tampered_state)? == before,
        "tamper failure changed state"
    );
    let mut accepted = pre.clone();
    let started = Instant::now();
    accepted.transition_from_privacy_preserving_transaction(tx, block, fixture::EXPIRES - 1)?;
    let seconds = started.elapsed().as_secs_f64();
    let after = borsh::to_vec(&accepted)?;
    ensure!(after != before, "accepted transition did not change state");
    for commitment in tx.message.commitments() {
        ensure!(
            accepted.get_proof_for_commitment(&commitment).is_some(),
            "accepted commitment missing"
        );
    }
    let mut replay_state = accepted.clone();
    let replay = replay_state
        .transition_from_privacy_preserving_transaction(tx, block + 1, fixture::EXPIRES - 1)
        .expect_err("replay accepted");
    ensure!(
        borsh::to_vec(&replay_state)? == after,
        "replay failure changed state"
    );
    Ok((
        accepted,
        json!({"valid_transition":true,"validation_seconds":seconds,
        "expiry_rejected":format!("{error:?}"),"modified_message_rejected":format!("{tamper:?}"),
        "replay_rejected":format!("{replay:?}"),"failed_checks_preserved_state":true,
        "pre_state_sha256":measured::sha(&before),"post_state_sha256":measured::sha(&after)}),
    ))
}

fn record_or_check_state(path: &Path, state: &V03State, checking: bool) -> Result<()> {
    let bytes = borsh::to_vec(state)?;
    if checking {
        ensure!(
            fs::read(path)? == bytes,
            "saved accepted state changed: {}",
            path.display()
        );
    } else {
        fs::write(path, bytes)?;
    }
    Ok(())
}

fn encodings(tx: &PrivacyPreservingTransaction) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>)> {
    let borsh_bytes = borsh::to_vec(tx)?;
    let core_bytes = tx.to_bytes();
    ensure!(
        borsh::from_slice::<PrivacyPreservingTransaction>(&borsh_bytes)? == *tx,
        "Borsh round trip failed"
    );
    ensure!(
        PrivacyPreservingTransaction::from_bytes(&core_bytes)? == *tx,
        "core round trip failed"
    );
    let mut envelope = vec![1u8];
    envelope.extend_from_slice(&borsh_bytes);
    Ok((borsh_bytes, core_bytes, envelope))
}

pub(crate) fn save_transaction(
    tx: &PrivacyPreservingTransaction,
    dir: &Path,
    format: &str,
) -> Result<Value> {
    let (borsh_bytes, core_bytes, envelope) = encodings(tx)?;
    fs::write(
        dir.join(format!("transaction-{format}.borsh")),
        &borsh_bytes,
    )?;
    fs::write(dir.join(format!("transaction-{format}.core")), &core_bytes)?;
    fs::write(
        dir.join(format!("transaction-{format}.lee-envelope")),
        &envelope,
    )?;
    Ok(encoding_report(&borsh_bytes, &core_bytes, &envelope))
}

fn check_encoding(tx: &PrivacyPreservingTransaction, dir: &Path, format: &str) -> Result<Value> {
    let (borsh_bytes, core_bytes, envelope) = encodings(tx)?;
    for (suffix, bytes) in [
        ("borsh", &borsh_bytes),
        ("core", &core_bytes),
        ("lee-envelope", &envelope),
    ] {
        ensure!(
            fs::read(dir.join(format!("transaction-{format}.{suffix}")))? == *bytes,
            "saved {suffix} encoding changed"
        );
    }
    Ok(encoding_report(&borsh_bytes, &core_bytes, &envelope))
}

fn encoding_report(borsh_bytes: &[u8], core_bytes: &[u8], envelope: &[u8]) -> Value {
    json!({"borsh_transaction_bytes":borsh_bytes.len(),"core_transaction_bytes":core_bytes.len(),
        "lee_envelope_bytes":envelope.len(),"borsh_sha256":measured::sha(borsh_bytes),
        "envelope_sha256":measured::sha(envelope),
        "envelope_encoding":"pinned common::LeeTransaction layout; RPC verification is a separate gate",
        "fits_repository_1mib_minus_2048":envelope.len() <= 1024 * 1024 - 2048})
}
