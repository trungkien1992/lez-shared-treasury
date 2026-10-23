//! Offline coordination messages. The LEZ guest remains the execution authority.
use anyhow::{Context as _, Result, bail, ensure};
use k256::schnorr::{Signature, SigningKey, VerifyingKey, signature::hazmat::PrehashVerifier};
use lee::program::Program;
use lee_core::{account::AccountId, program::ApplyInput};
use lifecycle_core::{Create, Descriptor, Instruction, transfer as vault};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{borrow::Cow, collections::BTreeMap};

pub mod state_import;

pub const GUEST_SHA256: &str = "fff9878b2215978e6be4bd62ac1f89a4c3b1b351bf4f0d61b308eaa0c30e321c";
pub const CONTEXT_FORMAT: &str = "lez-treasury-context/v1";
pub const REQUEST_FORMAT: &str = "lez-treasury-request/v1";
pub const RESPONSE_FORMAT: &str = "lez-treasury-response/v1";
pub const BUNDLE_FORMAT: &str = "lez-treasury-responses/v1";
pub const MAX_FILE_BYTES: usize = 131_072;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Context {
    pub format: String,
    pub guest_sha256: String,
    pub descriptor_hex: String,
    /// Must come from an independently accepted private note for a live spend.
    /// `from_descriptor` creates only a local expected initial configuration.
    pub state_hex: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub format: String,
    pub context_id: String,
    /// Borsh Instruction with an empty approval list; review decodes these bytes.
    pub instruction_hex: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Decision {
    Approve,
    Reject,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub format: String,
    pub context_id: String,
    pub action_digest: String,
    pub member_index: u32,
    pub decision: Decision,
    pub signature: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    pub format: String,
    pub responses: Vec<Response>,
}

pub struct Decoded {
    pub program: AccountId,
    pub vault: AccountId,
    pub state: vault::VaultState,
    pub instruction: Instruction,
    pub configuration: approval_core::Configuration,
    pub action: approval_core::Action,
    pub digest: [u8; 32],
    pub review_digest: [u8; 32],
}

pub fn hash(data: &[u8]) -> [u8; 32] {
    Sha256::digest(data).into()
}

pub fn program_id() -> Result<AccountId> {
    let elf = lifecycle_methods::LIFECYCLE_GUEST_ELF;
    ensure!(
        hex::encode(hash(elf)) == GUEST_SHA256,
        "guest image differs from the reviewed program"
    );
    Ok(AccountId::for_shadow_program(
        &Program::new(Cow::Borrowed(elf))?.id(),
    ))
}

pub fn unhex<const N: usize>(s: &str) -> Result<[u8; N]> {
    ensure!(s.len() == N * 2, "expected {N} bytes of hexadecimal");
    Ok(hex::decode(s)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("invalid hex length"))?)
}

pub fn decode_borsh<T: borsh::BorshDeserialize>(s: &str) -> Result<T> {
    ensure!(s.len() <= MAX_FILE_BYTES, "encoded input is too large");
    Ok(borsh::from_slice(&hex::decode(s)?)?)
}

pub fn validate_configuration(c: &approval_core::Configuration) -> Result<()> {
    let input = approval_core::Input {
        configuration: c.clone(),
        action: approval_core::Action {
            network_id: [0; 32],
            instance_id: [0; 32],
            program_id: [0; 32],
            configuration_epoch: c.epoch,
            proposal_id: [0; 32],
            target_program: [0; 32],
            expires_at: 1,
            instruction: vec![],
        },
        approvals: vec![],
    };
    ensure!(
        approval_core::verify(&input) == Err(approval_core::Error::InsufficientApprovals),
        "invalid member keys, duplicates, or threshold"
    );
    Ok(())
}

impl Context {
    pub fn from_descriptor(d: &Descriptor) -> Result<Self> {
        let c = Self {
            format: CONTEXT_FORMAT.into(),
            guest_sha256: GUEST_SHA256.into(),
            descriptor_hex: hex::encode(borsh::to_vec(d)?),
            state_hex: hex::encode(borsh::to_vec(&lifecycle_core::initial_state(d))?),
        };
        c.parts()?;
        Ok(c)
    }

    pub fn parts(&self) -> Result<(AccountId, AccountId, Descriptor, vault::VaultState)> {
        ensure!(
            self.format == CONTEXT_FORMAT && self.guest_sha256 == GUEST_SHA256,
            "unsupported context or guest"
        );
        let program = program_id()?;
        let descriptor: Descriptor = decode_borsh(&self.descriptor_hex)?;
        ensure!(
            descriptor.version == lifecycle_core::ENROLLMENT_VERSION,
            "unsupported descriptor version"
        );
        ensure!(
            descriptor.vpk.to_bytes().len() == lee_core::encryption::ViewingPublicKey::LEN,
            "invalid viewing public key"
        );
        validate_configuration(&vault::configuration(&lifecycle_core::initial_state(
            &descriptor,
        )))?;
        let state: vault::VaultState = decode_borsh(&self.state_hex)?;
        validate_configuration(&vault::configuration(&state))?;
        // The archived guest has no configuration-change instruction yet.
        ensure!(
            state.version == vault::VERSION
                && state.epoch == lifecycle_core::INITIAL_EPOCH
                && state.network_id == descriptor.network_id
                && state.seed == lifecycle_core::seed(&descriptor)
                && state.member_keys == descriptor.member_keys
                && state.threshold == descriptor.threshold,
            "context configuration does not match this descriptor and program version"
        );
        let vault = lifecycle_core::vault_id(&descriptor, program);
        Ok((program, vault, descriptor, state))
    }

    pub fn id(&self) -> Result<[u8; 32]> {
        let (program, vault, descriptor, state) = self.parts()?;
        let mut bytes = b"LEZ-RFP005/client-context/v1\0".to_vec();
        bytes.extend(program.value());
        bytes.extend(vault.value());
        bytes.extend(borsh::to_vec(&descriptor)?);
        bytes.extend(borsh::to_vec(&state)?);
        Ok(hash(&bytes))
    }
}

pub fn request(context: &Context, instruction: Instruction) -> Result<Request> {
    let r = Request {
        format: REQUEST_FORMAT.into(),
        context_id: hex::encode(context.id()?),
        instruction_hex: hex::encode(borsh::to_vec(&instruction)?),
    };
    decode(context, &r)?;
    Ok(r)
}

pub fn decode(context: &Context, r: &Request) -> Result<Decoded> {
    ensure!(r.format == REQUEST_FORMAT, "unsupported request format");
    ensure!(
        unhex::<32>(&r.context_id)? == context.id()?,
        "request uses a different configuration or proposal counter"
    );
    let (program, vault_id, descriptor, state) = context.parts()?;
    let instruction: Instruction = decode_borsh(&r.instruction_hex)?;
    let (configuration, action) = match &instruction {
        Instruction::Create(c) => {
            ensure!(
                c.enrollments.is_empty(),
                "request must contain no embedded responses"
            );
            ensure!(
                c.descriptor == descriptor && state.next_proposal == 0,
                "creation descriptor or counter mismatch"
            );
            ensure!(c.expires_at_ms > 0, "expiry must be positive");
            if let Some(f) = &c.funding {
                ensure!(
                    f.amount > 0 && f.from != vault_id,
                    "invalid inbound funding"
                );
            }
            (
                lifecycle_core::enrollment_configuration(&descriptor),
                lifecycle_core::enrollment_action(c, program),
            )
        }
        Instruction::Transfer(r) => {
            ensure!(
                r.approvals.is_empty(),
                "request must contain no embedded responses"
            );
            let t = &r.transfer;
            ensure!(
                t.epoch == state.epoch && t.proposal == state.next_proposal && t.seed == state.seed,
                "transfer epoch, counter, or custody seed is stale"
            );
            ensure!(
                t.amount > 0 && t.expires_at_ms > 0 && t.recipient != vault_id,
                "invalid transfer amount, expiry, or recipient"
            );
            ensure!(state.next_proposal < u64::MAX, "proposal counter exhausted");
            (
                vault::configuration(&state),
                vault::action(&state, vault_id, program, t),
            )
        }
    };
    let digest = approval_core::action_digest(&configuration, &action);
    let mut review_bytes = b"LEZ-RFP005/client-review/v1\0".to_vec();
    review_bytes.extend(context.id()?);
    review_bytes.extend(hex::decode(&r.instruction_hex)?);
    Ok(Decoded {
        program,
        vault: vault_id,
        state,
        instruction,
        configuration,
        action,
        digest,
        review_digest: hash(&review_bytes),
    })
}

pub fn review(context: &Context, r: &Request) -> Result<Value> {
    let d = decode(context, r)?;
    let details = match &d.instruction {
        Instruction::Create(c) => {
            json!({"kind":"enroll","signed_scope":"complete treasury descriptor and expiry; inbound funding is separately authorized",
            "spending_threshold":c.descriptor.threshold,"required_enrollments":c.descriptor.member_keys.len(),
            "descriptor_digest":hex::encode(lifecycle_core::descriptor_digest(&c.descriptor)),
            "viewing_key_sha256":hex::encode(hash(c.descriptor.vpk.to_bytes())),
            "nullifier_public_key":hex::encode(c.descriptor.npk.0),
            "identifier":hex::encode(c.descriptor.identifier.value()),
            "funding":c.funding.as_ref().map(|f| json!({"from_hex":hex::encode(f.from.value()),
                "amount_base_units":f.amount.to_string(),"approved_by_this_enrollment":false}))})
        }
        Instruction::Transfer(r) => {
            json!({"kind":"native-transfer","target_program_hex":hex::encode(d.action.target_program),
            "recipient_hex":hex::encode(r.transfer.recipient.value()),"recipient":r.transfer.recipient.to_string(),
            "amount_base_units":r.transfer.amount.to_string(),"epoch":r.transfer.epoch,
            "proposal_counter":r.transfer.proposal,"custody_seed_hex":hex::encode(borsh::to_vec(&r.transfer.seed)?)})
        }
    };
    Ok(
        json!({"decoded_from":"exact Borsh instruction bytes","instruction_sha256":hex::encode(hash(&hex::decode(&r.instruction_hex)?)),
        "review_digest":hex::encode(d.review_digest),"action_digest":hex::encode(d.digest),
        "context_id":hex::encode(context.id()?),"guest_sha256":GUEST_SHA256,
        "program_hex":hex::encode(d.program.value()),"vault_hex":hex::encode(d.vault.value()),"vault":d.vault.to_string(),
        "network_domain_hex":hex::encode(d.action.network_id),"expires_at_ms":d.action.expires_at,
        "required_approvals":d.configuration.threshold,"member_public_keys":d.configuration.member_keys.iter().map(hex::encode).collect::<Vec<_>>(),
        "action":details,"context_authentication":"external; this file alone is not evidence of accepted chain state",
        "transport":"offline files; Logos Chat adapter pending"}),
    )
}

pub fn ensure_unexpired(d: &Decoded, now_ms: u64) -> Result<()> {
    ensure!(
        now_ms < d.action.expires_at,
        "request expired; create a new request and collect new signatures"
    );
    Ok(())
}

fn decision_digest(digest: &[u8; 32], decision: Decision) -> [u8; 32] {
    if decision == Decision::Approve {
        return *digest;
    }
    let mut b = b"LEZ-RFP005/rejection/v1\0".to_vec();
    b.extend(digest);
    hash(&b)
}

/// Offline signing against a caller-supplied context. For checkpoint-backed
/// transfer signing, use `state_import::VerifiedState::respond` instead.
pub fn respond(
    context: &Context,
    r: &Request,
    key: &SigningKey,
    decision: Decision,
    expected_review: &[u8; 32],
    now_ms: u64,
) -> Result<Response> {
    let d = decode(context, r)?;
    ensure_unexpired(&d, now_ms)?;
    ensure!(
        &d.review_digest == expected_review,
        "review digest changed; review the exact request again before signing"
    );
    let public: [u8; 32] = key.verifying_key().to_bytes().into();
    let index = d
        .configuration
        .member_keys
        .iter()
        .position(|k| k == &public)
        .context("key is not a member of this treasury")?;
    let mut aux = [0; 32];
    OsRng
        .try_fill_bytes(&mut aux)
        .context("operating-system randomness unavailable")?;
    let signature = key.sign_prehash_with_aux_rand(&decision_digest(&d.digest, decision), &aux)?;
    Ok(Response {
        format: RESPONSE_FORMAT.into(),
        context_id: hex::encode(context.id()?),
        action_digest: hex::encode(d.digest),
        member_index: index as u32,
        decision,
        signature: hex::encode(signature.to_bytes()),
    })
}

pub fn verify_response(context: &Context, d: &Decoded, r: &Response) -> Result<()> {
    ensure!(r.format == RESPONSE_FORMAT, "unsupported response format");
    ensure!(
        unhex::<32>(&r.context_id)? == context.id()? && unhex::<32>(&r.action_digest)? == d.digest,
        "response belongs to a different request or configuration"
    );
    let key = d
        .configuration
        .member_keys
        .get(r.member_index as usize)
        .context("response is from a nonmember index")?;
    let sig = Signature::try_from(unhex::<64>(&r.signature)?.as_slice())?;
    VerifyingKey::from_bytes(key)?
        .verify_prehash(&decision_digest(&d.digest, r.decision), &sig)
        .context("invalid approval or rejection signature")?;
    Ok(())
}

pub fn collect(
    context: &Context,
    r: &Request,
    responses: impl IntoIterator<Item = Response>,
    now_ms: u64,
) -> Result<Bundle> {
    let d = decode(context, r)?;
    ensure_unexpired(&d, now_ms)?;
    let mut map = BTreeMap::<u32, Response>::new();
    let mut count = 0;
    for response in responses {
        count += 1;
        ensure!(
            count <= approval_core::MAX_MEMBERS * 8,
            "too many response messages in one collection"
        );
        verify_response(context, &d, &response)?;
        if let Some(old) = map.get(&response.member_index) {
            ensure!(
                old.decision == response.decision,
                "member {} issued conflicting signed decisions; use a new proposal",
                response.member_index
            );
            // Duplicate delivery and independently randomized same-decision signatures count once.
            if response.signature < old.signature {
                map.insert(response.member_index, response);
            }
        } else {
            map.insert(response.member_index, response);
        }
    }
    Ok(Bundle {
        format: BUNDLE_FORMAT.into(),
        responses: map.into_values().collect(),
    })
}

pub fn status(context: &Context, r: &Request, bundle: &Bundle, now_ms: u64) -> Result<Value> {
    ensure!(bundle.format == BUNDLE_FORMAT, "unsupported bundle format");
    let canonical = collect(context, r, bundle.responses.clone(), now_ms)?;
    let d = decode(context, r)?;
    let approved: Vec<_> = canonical
        .responses
        .iter()
        .filter(|r| r.decision == Decision::Approve)
        .map(|r| r.member_index)
        .collect();
    let rejected: Vec<_> = canonical
        .responses
        .iter()
        .filter(|r| r.decision == Decision::Reject)
        .map(|r| r.member_index)
        .collect();
    Ok(
        json!({"action_digest":hex::encode(d.digest),"approved_indices":approved,"rejected_indices":rejected,
        "required_approvals":d.configuration.threshold,"quorum":approved.len() >= d.configuration.threshold as usize,
        "rejection_policy":"a rejection is a signed opinion, not a veto; a member's conflicting decisions halt export",
        "execution_status":"tracked externally by prover and sequencer evidence"}),
    )
}

/// Produces exactly the Borsh Instruction accepted by the unchanged guest.
pub fn export(
    context: &Context,
    r: &Request,
    bundle: &Bundle,
    now_ms: u64,
) -> Result<(Vec<u8>, Vec<u8>)> {
    ensure!(bundle.format == BUNDLE_FORMAT, "unsupported bundle format");
    let b = collect(context, r, bundle.responses.clone(), now_ms)?;
    let mut d = decode(context, r)?;
    let approvals: Vec<_> = b
        .responses
        .iter()
        .filter(|r| r.decision == Decision::Approve)
        .map(|r| {
            Ok(vault::Approval {
                member_index: r.member_index,
                signature: unhex::<64>(&r.signature)?,
            })
        })
        .collect::<Result<_>>()?;
    approval_core::verify(&approval_core::Input {
        configuration: d.configuration,
        action: d.action,
        approvals: approvals
            .iter()
            .map(|a| approval_core::Approval {
                member_index: a.member_index,
                signature: a.signature.to_vec(),
            })
            .collect(),
    })
    .map_err(|e| anyhow::anyhow!("cannot export approval package: {e:?}"))?;
    let pre_data = match &mut d.instruction {
        Instruction::Create(c) => {
            c.enrollments = approvals;
            lee_core::account::ShardData::empty()
        }
        Instruction::Transfer(r) => {
            r.approvals = approvals;
            borsh::to_vec(&d.state)?
                .try_into()
                .map_err(|_| anyhow::anyhow!("configuration too large"))?
        }
    };
    let bytes = borsh::to_vec(&d.instruction)?;
    let native = lifecycle_core::apply(ApplyInput {
        self_account_id: d.program,
        selector: lee_core::account::ProgramShardSelector::new(d.vault, d.program),
        pre_data,
        effect_data: bytes.clone(),
    })
    .map_err(|e| anyhow::anyhow!("guest core rejected exported instruction: {e}"))?;
    Ok((bytes, borsh::to_vec(&native)?))
}

pub fn decimal_amount(s: &str) -> Result<u128> {
    ensure!(
        !s.is_empty()
            && s.bytes().all(|b| b.is_ascii_digit())
            && (s.len() == 1 || !s.starts_with('0')),
        "amount must be a canonical integer in base units"
    );
    let amount: u128 = s.parse().context("amount exceeds u128")?;
    ensure!(amount > 0, "amount must be positive");
    Ok(amount)
}

pub fn unsigned_create(
    context: &Context,
    expires_at_ms: u64,
    funding: Option<lifecycle_core::Funding>,
) -> Result<Request> {
    let (_, _, descriptor, _) = context.parts()?;
    request(
        context,
        Instruction::Create(Create {
            descriptor,
            expires_at_ms,
            enrollments: vec![],
            funding,
        }),
    )
}

pub fn unsigned_transfer(
    context: &Context,
    recipient: AccountId,
    amount: u128,
    expires_at_ms: u64,
) -> Result<Request> {
    let (_, _, _, state) = context.parts()?;
    request(
        context,
        Instruction::Transfer(vault::Request {
            transfer: vault::Transfer {
                epoch: state.epoch,
                proposal: state.next_proposal,
                expires_at_ms,
                seed: state.seed,
                recipient,
                amount,
            },
            approvals: vec![],
        }),
    )
}

pub fn advance_expected_context(context: &Context, exported_instruction: &[u8]) -> Result<Context> {
    let mut result = context.clone();
    let (_, _, _, mut state) = context.parts()?;
    match borsh::from_slice::<Instruction>(exported_instruction)? {
        Instruction::Transfer(r) => {
            ensure!(
                r.transfer.proposal == state.next_proposal,
                "transfer is not for the current counter"
            );
            state.next_proposal = state
                .next_proposal
                .checked_add(1)
                .context("counter exhausted")?;
            result.state_hex = hex::encode(borsh::to_vec(&state)?);
        }
        Instruction::Create(_) => {}
    }
    // This is a predicted value only, never a claim of network acceptance.
    Ok(result)
}

pub fn parse_decision(s: &str) -> Result<Decision> {
    match s {
        "approve" => Ok(Decision::Approve),
        "reject" => Ok(Decision::Reject),
        _ => bail!("decision must be approve or reject"),
    }
}
