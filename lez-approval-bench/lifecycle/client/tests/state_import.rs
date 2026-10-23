mod support;
use k256::schnorr::SigningKey;
use lee::{PrivacyPreservingTransaction, V03State};
use lee_core::{Commitment, account::AccountId};
use treasury_client::state_import::{self, MAX_CHECKPOINT_AGE_MS};
use treasury_client::{Decision, Request, decode, unsigned_transfer};

const NOW: u64 = 10_001;
fn verify(f: &support::Fixture) -> anyhow::Result<state_import::VerifiedState> {
    state_import::verify(
        &borsh::to_vec(&f.evidence)?,
        &f.checkpoint,
        &f.nullifier_key,
        NOW,
    )
}
fn request(s: &state_import::VerifiedState) -> Request {
    unsigned_transfer(s.context(), AccountId::new([18; 32]), 37, 2_000_000_000_000).unwrap()
}
fn key() -> SigningKey {
    let mut bytes = [0; 32];
    bytes[31] = 1;
    SigningKey::from_bytes(&bytes).unwrap()
}

#[test]
fn real_receipt_replay_imports_unspent_creation_and_signs_guest_compatible_approval() {
    let f = support::fixture(false, false, false);
    let s = verify(&f).unwrap();
    assert_eq!(s.report().unwrap()["balance_base_units"], "500");
    assert_eq!(s.report().unwrap()["proposal_counter"], 0);
    let r = request(&s);
    let review = s.review(&r, NOW).unwrap();
    let digest = treasury_client::unhex::<32>(review["review_digest"].as_str().unwrap()).unwrap();
    let response = s
        .respond(&r, &key(), Decision::Approve, &digest, NOW)
        .unwrap();
    treasury_client::verify_response(s.context(), &decode(s.context(), &r).unwrap(), &response)
        .unwrap();
}

#[test]
fn latest_member_workflow_imports_the_current_post_payment_note() {
    let f = support::fixture(true, true, true);
    let s = verify(&f).unwrap();
    assert_eq!(s.report().unwrap()["balance_base_units"], "463");
    assert_eq!(s.report().unwrap()["proposal_counter"], 1);
    assert_eq!(s.report().unwrap()["replayed_transactions"], 3);
}

#[test]
fn spent_note_is_rejected_even_though_its_membership_proof_still_exists() {
    let f = support::fixture(true, true, false);
    let report =
        support::root().join("reports/2026-10-13-client-real-lifecycle/after-spend.state.borsh");
    let head: V03State = borsh::from_slice(&std::fs::read(report).unwrap()).unwrap();
    let vault = lifecycle_core::vault_id(
        &f.evidence.descriptor,
        treasury_client::program_id().unwrap(),
    );
    assert!(
        head.get_proof_for_commitment(&Commitment::new(&vault, &f.evidence.vault_account))
            .is_some()
    );
    assert!(
        verify(&f)
            .err()
            .unwrap()
            .to_string()
            .contains("has been spent")
    );
}

#[test]
fn modified_note_key_or_descriptor_cannot_create_a_verified_context() {
    let mut f = support::fixture(false, false, false);
    let original = f.evidence.vault_account.clone();
    f.evidence.vault_account.nonce.0 ^= 1;
    assert!(verify(&f).is_err());
    f.evidence.vault_account = original;
    f.nullifier_key[0] ^= 1;
    assert!(
        verify(&f)
            .err()
            .unwrap()
            .to_string()
            .contains("nullifier key")
    );
    f.nullifier_key[0] ^= 1;
    f.evidence.descriptor.salt[0] ^= 1;
    assert!(verify(&f).is_err());
}

#[test]
fn wrong_snapshot_network_vault_guest_or_head_hash_is_rejected() {
    let original = support::fixture(false, false, false);
    for which in 0..5 {
        let mut f = support::fixture(false, false, false);
        match which {
            0 => f.evidence.base_state[100] ^= 1,
            1 => f.checkpoint.network_domain_hex = "00".repeat(32),
            2 => f.checkpoint.vault_hex = "00".repeat(32),
            3 => f.checkpoint.guest_sha256 = "00".repeat(32),
            _ => f.checkpoint.head_state_sha256 = "00".repeat(32),
        }
        assert!(verify(&f).is_err(), "case {which}");
    }
    assert!(verify(&original).is_ok());
}

#[test]
fn changed_proof_bound_message_and_omitted_or_reordered_replay_fail() {
    let mut f = support::fixture(false, false, false);
    let mut tx: PrivacyPreservingTransaction =
        borsh::from_slice(&f.evidence.steps[1].transaction).unwrap();
    tx.message.timestamp_validity_window = (..1_999_999_999_999).into();
    f.evidence.steps[1].transaction = borsh::to_vec(&tx).unwrap();
    assert!(verify(&f).is_err());
    let mut f = support::fixture(false, false, false);
    f.evidence.steps.remove(0);
    assert!(verify(&f).is_err());
    let mut f = support::fixture(false, false, false);
    f.evidence.steps.reverse();
    assert!(verify(&f).is_err());
}

#[test]
fn development_receipts_are_explicitly_forbidden() {
    use lee::privacy_preserving_transaction::{circuit::Proof, witness_set::WitnessSet};
    use risc0_zkvm::{FakeReceipt, InnerReceipt, ReceiptClaim};
    let mut f = support::fixture(false, false, false);
    let mut tx: PrivacyPreservingTransaction =
        borsh::from_slice(&f.evidence.steps[1].transaction).unwrap();
    let inner = InnerReceipt::Fake(FakeReceipt::new(ReceiptClaim::ok(
        lee::PRIVACY_PRESERVING_CIRCUIT_ID,
        Vec::<u8>::new(),
    )));
    tx.witness_set = WitnessSet::for_message(
        &tx.message,
        Proof::from_inner(borsh::to_vec(&inner).unwrap()),
        &[],
    );
    f.evidence.steps[1].transaction = borsh::to_vec(&tx).unwrap();
    assert!(
        verify(&f)
            .err()
            .unwrap()
            .to_string()
            .contains("fake receipts forbidden")
    );
}

#[test]
fn checkpoint_expiry_age_future_observation_and_counter_rollback_fail() {
    let mut f = support::fixture(false, false, false);
    let bytes = borsh::to_vec(&f.evidence).unwrap();
    assert!(state_import::verify(&bytes, &f.checkpoint, &f.nullifier_key, 20_000).is_err());
    assert!(state_import::verify(&bytes, &f.checkpoint, &f.nullifier_key, 9_999).is_err());
    f.checkpoint.valid_until_ms = u64::MAX;
    assert!(
        state_import::verify(
            &bytes,
            &f.checkpoint,
            &f.nullifier_key,
            10_000 + MAX_CHECKPOINT_AGE_MS + 1
        )
        .is_err()
    );
    f.checkpoint.minimum_proposal_counter = 1;
    assert!(
        verify(&f)
            .err()
            .unwrap()
            .to_string()
            .contains("rolled back")
    );
}

#[test]
fn signing_binds_the_trusted_checkpoint_and_rechecks_freshness() {
    let f = support::fixture(false, false, false);
    let s = verify(&f).unwrap();
    let r = request(&s);
    let plain = decode(s.context(), &r).unwrap().review_digest;
    assert!(
        s.respond(&r, &key(), Decision::Approve, &plain, NOW)
            .is_err()
    );
    let review = s.review(&r, NOW).unwrap();
    let digest = treasury_client::unhex::<32>(review["review_digest"].as_str().unwrap()).unwrap();
    assert!(
        s.respond(&r, &key(), Decision::Approve, &digest, 20_000)
            .is_err()
    );
    let mut f = support::fixture(false, false, false);
    f.checkpoint.valid_until_ms += 1;
    let changed = verify(&f).unwrap();
    assert!(
        changed
            .respond(&r, &key(), Decision::Approve, &digest, NOW)
            .is_err()
    );
}

#[test]
fn wrong_replay_bounds_trailing_bytes_and_version_fail_closed() {
    let mut f = support::fixture(false, false, false);
    f.evidence.steps[1].timestamp_ms = 999;
    assert!(verify(&f).is_err());
    let mut f = support::fixture(false, false, false);
    f.evidence.steps[1].block_id = 99;
    assert!(verify(&f).is_err());
    let f = support::fixture(false, false, false);
    let mut bytes = borsh::to_vec(&f.evidence).unwrap();
    bytes.push(0);
    assert!(state_import::verify(&bytes, &f.checkpoint, &f.nullifier_key, NOW).is_err());
    let mut f = support::fixture(false, false, false);
    f.evidence.version += 1;
    assert!(verify(&f).is_err());
}

#[test]
fn ambiguous_legacy_cli_signing_requires_an_explicit_mode() {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_treasury-client"))
        .arg("respond")
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("respond-verified"));
}

#[test]
#[cfg(unix)]
fn verified_cli_import_review_and_sign_roundtrip_with_real_archived_proofs() {
    use std::{
        fs,
        io::Write,
        process::Command,
        time::{SystemTime, UNIX_EPOCH},
    };
    struct Scratch(std::path::PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let directory =
        Scratch(std::env::temp_dir().join(format!("lez-state-cli-{}-{now}", std::process::id())));
    fs::create_dir(&directory.0).unwrap();
    let write = |name: &str, bytes: &[u8]| {
        use std::os::unix::fs::OpenOptionsExt;
        let path = directory.0.join(name);
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .unwrap();
        file.write_all(bytes).unwrap();
        path.to_str().unwrap().to_string()
    };
    let mut f = support::fixture(false, false, false);
    f.checkpoint.observed_at_ms = now;
    f.checkpoint.valid_until_ms = now + 120_000;
    let evidence = write("evidence.borsh", &borsh::to_vec(&f.evidence).unwrap());
    let checkpoint = write(
        "checkpoint.json",
        &serde_json::to_vec(&f.checkpoint).unwrap(),
    );
    let nullifier = write(
        "public-test-nullifier.key",
        hex::encode(f.nullifier_key).as_bytes(),
    );
    let signing = write(
        "public-test-member.key",
        hex::encode(key().to_bytes()).as_bytes(),
    );
    let context = directory
        .0
        .join("context.json")
        .to_str()
        .unwrap()
        .to_string();
    let binary = env!("CARGO_BIN_EXE_treasury-client");
    let run = |args: &[&str]| {
        let output = Command::new(binary).args(args).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()
    };
    let imported = run(&["import-state", &evidence, &checkpoint, &nullifier, &context]);
    assert_eq!(imported["candidate_unspent_at_checkpoint"], true);
    assert_eq!(imported["balance_base_units"], "500");
    let context: treasury_client::Context =
        serde_json::from_slice(&fs::read(context).unwrap()).unwrap();
    let request = unsigned_transfer(&context, AccountId::new([18; 32]), 37, now + 120_000).unwrap();
    let request_path = write("request.json", &serde_json::to_vec(&request).unwrap());
    let reviewed = run(&[
        "review-verified",
        &evidence,
        &checkpoint,
        &nullifier,
        &request_path,
    ]);
    let response_path = directory
        .0
        .join("response.json")
        .to_str()
        .unwrap()
        .to_string();
    run(&[
        "respond-verified",
        &evidence,
        &checkpoint,
        &nullifier,
        &request_path,
        &signing,
        "approve",
        reviewed["review_digest"].as_str().unwrap(),
        &response_path,
    ]);
    let response = serde_json::from_slice(&fs::read(response_path).unwrap()).unwrap();
    treasury_client::verify_response(&context, &decode(&context, &request).unwrap(), &response)
        .unwrap();
    println!(
        "verified CLI roundtrip: imported accepted state, reviewed checkpoint-bound bytes, signed and independently checked approval"
    );
}
