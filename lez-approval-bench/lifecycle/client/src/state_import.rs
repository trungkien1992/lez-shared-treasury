//! Import relative to an independently trusted checkpoint, not an RPC-supplied root.
//! The base snapshot must precede the candidate note. Replaying the complete suffix
//! verifies receipts and tracks its spend nullifier, including spent notes that
//! remain members of the append-only commitment tree. No networking or key storage.
use anyhow::{Context as _, Result, ensure};
use borsh::{BorshDeserialize, BorshSerialize};
use k256::schnorr::SigningKey;
use lee::{PrivacyPreservingTransaction, V03State};
use lee_core::{Commitment, Nullifier, NullifierPublicKey, NullifierSecretKey, account::Account};
use lifecycle_core::{Descriptor, Instruction, transfer::VaultState};
use risc0_zkvm::InnerReceipt;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{Context, Decision, GUEST_SHA256, Request, Response, decode, hash, unhex};

pub const EVIDENCE_VERSION: u32 = 1;
pub const CHECKPOINT_FORMAT: &str = "lez-treasury-trusted-checkpoint/v1";
pub const MAX_EVIDENCE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_CHECKPOINT_AGE_MS: u64 = 5 * 60 * 1000;

/// Obtained and authenticated independently of the request/evidence provider.
/// The state hashes commit to public state, commitments AND spent nullifiers.
/// This file is a caller trust anchor, not itself a consensus or finality proof.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrustedCheckpoint {
    pub format: String,
    pub guest_sha256: String,
    pub network_domain_hex: String,
    pub vault_hex: String,
    pub base_state_sha256: String,
    pub base_block_id: u64,
    pub head_state_sha256: String,
    pub head_block_id: u64,
    pub observed_at_ms: u64,
    pub valid_until_ms: u64,
    pub minimum_proposal_counter: u64,
}

#[derive(Clone, BorshSerialize, BorshDeserialize)]
pub struct AcceptedStep {
    pub block_id: u64,
    pub timestamp_ms: u64,
    /// Exact pinned LEE PrivacyPreservingTransaction Borsh bytes, not an envelope.
    pub transaction: Vec<u8>,
}

#[derive(Clone, BorshSerialize, BorshDeserialize)]
pub struct StateEvidence {
    pub version: u32,
    pub descriptor: Descriptor,
    pub vault_account: Account,
    pub base_state: Vec<u8>,
    pub steps: Vec<AcceptedStep>,
}

/// No public constructor or deserialization: a JSON flag cannot create this type.
pub struct VerifiedState {
    context: Context,
    checkpoint: TrustedCheckpoint,
    checkpoint_id: [u8; 32],
    commitment: Commitment,
    root: [u8; 32],
    balance: u128,
    replayed_transactions: usize,
}

impl TrustedCheckpoint {
    fn fresh(&self, now_ms: u64) -> Result<()> {
        ensure!(
            self.format == CHECKPOINT_FORMAT,
            "unsupported checkpoint format"
        );
        ensure!(
            self.guest_sha256 == GUEST_SHA256,
            "checkpoint uses a different guest"
        );
        ensure!(
            self.head_block_id > self.base_block_id,
            "checkpoint has no replay interval"
        );
        ensure!(
            self.valid_until_ms > self.observed_at_ms,
            "invalid checkpoint validity window"
        );
        ensure!(
            now_ms >= self.observed_at_ms,
            "checkpoint observation is in the future"
        );
        ensure!(now_ms < self.valid_until_ms, "checkpoint expired");
        ensure!(
            now_ms - self.observed_at_ms <= MAX_CHECKPOINT_AGE_MS,
            "checkpoint too old"
        );
        Ok(())
    }
}

pub fn verify(
    encoded: &[u8],
    checkpoint: &TrustedCheckpoint,
    nullifier_secret: &NullifierSecretKey,
    now_ms: u64,
) -> Result<VerifiedState> {
    checkpoint.fresh(now_ms)?;
    ensure!(
        encoded.len() <= MAX_EVIDENCE_BYTES,
        "state evidence too large"
    );
    let evidence: StateEvidence = borsh::from_slice(encoded).context("invalid state evidence")?;
    ensure!(
        evidence.version == EVIDENCE_VERSION,
        "unsupported state evidence version"
    );
    ensure!(
        !evidence.steps.is_empty() && evidence.steps.len() <= 128,
        "invalid replay length"
    );
    ensure!(
        hash(&evidence.base_state) == unhex::<32>(&checkpoint.base_state_sha256)?,
        "base snapshot differs from trusted checkpoint"
    );
    ensure!(
        evidence.descriptor.network_id == unhex::<32>(&checkpoint.network_domain_hex)?,
        "checkpoint network mismatch"
    );
    ensure!(
        NullifierPublicKey::from(nullifier_secret) == evidence.descriptor.npk,
        "nullifier key does not match treasury descriptor"
    );

    let mut context = Context::from_descriptor(&evidence.descriptor)?;
    let (program, vault, _, _) = context.parts()?;
    ensure!(
        *vault.value() == unhex::<32>(&checkpoint.vault_hex)?,
        "checkpoint vault mismatch"
    );
    let shard = evidence.vault_account.data.shard(program);
    let vault_state: VaultState =
        borsh::from_slice(shard).context("note lacks valid treasury state")?;
    context.state_hex = hex::encode(borsh::to_vec(&vault_state)?);
    context.parts()?;
    ensure!(
        vault_state.next_proposal >= checkpoint.minimum_proposal_counter,
        "vault proposal counter rolled back"
    );
    let commitment = Commitment::new(&vault, &evidence.vault_account);
    let spend_nullifier = Nullifier::for_account_update(&commitment, nullifier_secret);
    let mut state: V03State =
        borsh::from_slice(&evidence.base_state).context("invalid base state")?;
    ensure!(
        state.get_proof_for_commitment(&commitment).is_none(),
        "base snapshot must precede candidate note creation"
    );
    let mut created = false;
    let mut previous_block = checkpoint.base_block_id;
    let mut previous_timestamp = 0;
    for step in &evidence.steps {
        ensure!(
            step.block_id > checkpoint.base_block_id
                && step.block_id >= previous_block
                && step.block_id <= checkpoint.head_block_id,
            "replay block order or checkpoint bound violated"
        );
        ensure!(
            step.timestamp_ms >= previous_timestamp
                && step.timestamp_ms <= checkpoint.observed_at_ms,
            "replay timestamp order or checkpoint bound violated"
        );
        ensure!(
            step.transaction.len() <= 1024 * 1024,
            "replay transaction too large"
        );
        let tx: PrivacyPreservingTransaction =
            borsh::from_slice(&step.transaction).context("invalid replay transaction")?;
        let inner: InnerReceipt = borsh::from_slice(&tx.witness_set.proof().clone().into_inner())
            .context("invalid receipt encoding")?;
        ensure!(
            !matches!(inner, InnerReceipt::Fake(_)),
            "fake receipts forbidden"
        );
        ensure!(
            !tx.message
                .nullifiers()
                .iter()
                .any(|(n, _)| n == &spend_nullifier),
            "candidate vault note has been spent"
        );
        state
            .transition_from_privacy_preserving_transaction(&tx, step.block_id, step.timestamp_ms)
            .context("replayed transaction failed proof or state validation")?;
        created |= tx.message.commitments().contains(&commitment);
        previous_block = step.block_id;
        previous_timestamp = step.timestamp_ms;
    }
    ensure!(
        created,
        "candidate note was not created in the verified replay"
    );
    ensure!(
        hash(&borsh::to_vec(&state)?) == unhex::<32>(&checkpoint.head_state_sha256)?,
        "replayed head differs from trusted checkpoint"
    );
    let membership = state
        .get_proof_for_commitment(&commitment)
        .context("candidate commitment absent from verified head")?;
    let root = state.commitment_root();
    ensure!(
        lee_core::compute_digest_for_path(&commitment, &membership) == root,
        "candidate membership does not open the verified root"
    );
    let balance = lee_core::native_token::decode_balance(
        evidence
            .vault_account
            .data
            .shard(lee_core::native_token::NATIVE_TOKEN_PROGRAM_ID),
    )?;
    let checkpoint_id = hash(&serde_json::to_vec(checkpoint)?);
    Ok(VerifiedState {
        context,
        checkpoint: checkpoint.clone(),
        checkpoint_id,
        commitment,
        root,
        balance,
        replayed_transactions: evidence.steps.len(),
    })
}

impl VerifiedState {
    pub fn context(&self) -> &Context {
        &self.context
    }

    pub fn report(&self) -> Result<Value> {
        let (_, _, _, state) = self.context.parts()?;
        Ok(
            json!({"state_authentication":"receipt-verified replay relative to an independently trusted checkpoint",
            "checkpoint_id":hex::encode(self.checkpoint_id),"head_block_id":self.checkpoint.head_block_id,
            "commitment_hex":hex::encode(self.commitment.to_byte_array()),"commitment_root_hex":hex::encode(self.root),
            "balance_base_units":self.balance.to_string(),"proposal_counter":state.next_proposal,
            "replayed_transactions":self.replayed_transactions,"candidate_unspent_at_checkpoint":true,
            "public_network_finality_verified":false}),
        )
    }

    fn reviewed_digest(&self, request: &Request) -> Result<[u8; 32]> {
        let decoded = decode(&self.context, request)?;
        ensure!(
            matches!(decoded.instruction, Instruction::Transfer(_)),
            "verified-state signing currently supports transfers only"
        );
        let mut bytes = b"LEZ-RFP005/verified-state-review/v1\0".to_vec();
        bytes.extend(decoded.review_digest);
        bytes.extend(self.checkpoint_id);
        bytes.extend(self.commitment.to_byte_array());
        Ok(hash(&bytes))
    }

    pub fn review(&self, request: &Request, now_ms: u64) -> Result<Value> {
        self.checkpoint.fresh(now_ms)?;
        crate::ensure_unexpired(&decode(&self.context, request)?, now_ms)?;
        let mut result = crate::review(&self.context, request)?;
        result["review_digest"] = json!(hex::encode(self.reviewed_digest(request)?));
        result["context_authentication"] = self.report()?;
        Ok(result)
    }

    pub fn respond(
        &self,
        request: &Request,
        key: &SigningKey,
        decision: Decision,
        expected_review: &[u8; 32],
        now_ms: u64,
    ) -> Result<Response> {
        self.checkpoint.fresh(now_ms)?;
        ensure!(
            &self.reviewed_digest(request)? == expected_review,
            "verified state or reviewed request changed"
        );
        let decoded = decode(&self.context, request)?;
        crate::respond(
            &self.context,
            request,
            key,
            decision,
            &decoded.review_digest,
            now_ms,
        )
    }
}
