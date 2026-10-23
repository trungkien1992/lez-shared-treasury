//! Versioned creation/enrollment entry point, reusing the archived transfer core.
//! Every initial member signs the same private vault descriptor before creation.
#[path = "../../../integration/core/src/lib.rs"]
pub mod transfer;
use borsh::{BorshDeserialize, BorshSerialize};
use lee_core::{
    Identifier, NullifierPublicKey,
    account::{AccountId, ProgramShardSelector},
    encryption::ViewingPublicKey,
    native_token::{self, NATIVE_TOKEN_PROGRAM_ID},
    program::{ApplyInput, ApplyOutput, ChainedCall, PdaSeed, PlanInput, PlanOutput, ShardEffect},
};
use sha2::{Digest, Sha256};
use transfer as vault_core;

pub const ENROLLMENT_VERSION: u32 = 1;
pub const INITIAL_EPOCH: u64 = 1;

#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct Descriptor {
    pub version: u32,
    /// Application network domain, checked against deployment settings by clients.
    pub network_id: [u8; 32],
    /// Random per-vault salt. Public in the member invitation, never a secret key.
    pub salt: [u8; 32],
    pub threshold: u32,
    /// Ordered member list; changing its order changes the vault identity.
    pub member_keys: Vec<[u8; 32]>,
    pub npk: NullifierPublicKey,
    pub vpk: ViewingPublicKey,
    pub identifier: Identifier,
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct Funding {
    pub from: AccountId,
    pub amount: u128,
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct Create {
    pub descriptor: Descriptor,
    pub expires_at_ms: u64,
    /// Unanimous enrollment, even when the spending threshold is smaller.
    pub enrollments: Vec<vault_core::Approval>,
    /// Funding is separately authorized by its source account. Members approve
    /// the configuration; this optional inbound amount is not their signature.
    pub funding: Option<Funding>,
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub enum Instruction {
    Create(Create),
    Transfer(vault_core::Request),
}

pub fn descriptor_digest(descriptor: &Descriptor) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"LEZ-RFP005/vault-descriptor/v1\0");
    h.update(borsh::to_vec(descriptor).expect("descriptor serializes"));
    h.finalize().into()
}

pub fn seed(descriptor: &Descriptor) -> PdaSeed {
    PdaSeed::new(descriptor_digest(descriptor))
}

pub fn vault_id(descriptor: &Descriptor, program: AccountId) -> AccountId {
    AccountId::for_private_pda(
        &program,
        &seed(descriptor),
        &descriptor.npk,
        &descriptor.vpk,
        descriptor.identifier,
    )
}

pub fn initial_state(descriptor: &Descriptor) -> vault_core::VaultState {
    vault_core::VaultState {
        version: vault_core::VERSION,
        network_id: descriptor.network_id,
        epoch: INITIAL_EPOCH,
        threshold: descriptor.threshold,
        member_keys: descriptor.member_keys.clone(),
        seed: seed(descriptor),
        next_proposal: 0,
    }
}

pub fn enrollment_configuration(descriptor: &Descriptor) -> approval_core::Configuration {
    approval_core::Configuration {
        epoch: INITIAL_EPOCH,
        threshold: descriptor.member_keys.len() as u32,
        member_keys: descriptor.member_keys.clone(),
    }
}

pub fn enrollment_action(create: &Create, program: AccountId) -> approval_core::Action {
    let mut instruction = b"LEZ-RFP005/member-enrollment/v1\0".to_vec();
    instruction.extend(borsh::to_vec(&create.descriptor).expect("descriptor serializes"));
    approval_core::Action {
        network_id: create.descriptor.network_id,
        instance_id: *vault_id(&create.descriptor, program).value(),
        program_id: *program.value(),
        configuration_epoch: INITIAL_EPOCH,
        proposal_id: descriptor_digest(&create.descriptor),
        target_program: *program.value(),
        expires_at: create.expires_at_ms,
        instruction,
    }
}

pub fn enrollment_digest(create: &Create, program: AccountId) -> [u8; 32] {
    approval_core::action_digest(
        &enrollment_configuration(&create.descriptor),
        &enrollment_action(create, program),
    )
}

fn validate_create(create: &Create, vault: AccountId, program: AccountId) -> Result<(), String> {
    let d = &create.descriptor;
    let n = d.member_keys.len();
    if d.version != ENROLLMENT_VERSION
        || n == 0
        || n > approval_core::MAX_MEMBERS
        || d.threshold == 0
        || d.threshold as usize > n
        || create.expires_at_ms == 0
        || create.enrollments.len() > n
        || d.vpk.to_bytes().len() != ViewingPublicKey::LEN
    {
        return Err("invalid enrollment version, threshold or bounds".into());
    }
    if vault_id(d, program) != vault {
        return Err("vault address does not match enrolled descriptor".into());
    }
    Ok(())
}

pub fn selectors(create: &Create, program: AccountId) -> Vec<ProgramShardSelector> {
    let vault = vault_id(&create.descriptor, program);
    let mut selectors = vec![ProgramShardSelector::new(vault, program)];
    if let Some(funding) = &create.funding {
        selectors.push(ProgramShardSelector::native_balance(vault));
        selectors.push(ProgramShardSelector::native_balance(funding.from));
    }
    selectors
}

pub fn plan(input: PlanInput, instruction: Instruction) -> Result<PlanOutput, String> {
    match instruction {
        Instruction::Transfer(request) => {
            // Preserve the v1 transfer semantics, but tag its effect for the new dispatcher.
            let mut out = vault_core::plan(input, request.clone())?;
            out.effects[0].data = borsh::to_vec(&Instruction::Transfer(request)).unwrap();
            Ok(out)
        }
        Instruction::Create(create) => {
            let config = input
                .accounts
                .first()
                .ok_or("missing configuration handle")?;
            if config.program_account_id != input.self_account_id {
                return Err("configuration must belong to the executing program".into());
            }
            validate_create(&create, config.account_id, input.self_account_id)?;
            let calls = if let Some(funding) = &create.funding {
                let [_, balance, funder] = input.accounts.as_slice() else {
                    return Err("expected configuration, vault balance and funding balance".into());
                };
                if balance.account_id != config.account_id
                    || balance.program_account_id != NATIVE_TOKEN_PROGRAM_ID
                    || funder.account_id != funding.from
                    || funder.program_account_id != NATIVE_TOKEN_PROGRAM_ID
                    || !funder.is_authorized
                    || funding.from == config.account_id
                    || funding.amount == 0
                {
                    return Err("invalid or unauthorized funding source".into());
                }
                vec![ChainedCall::new(
                    NATIVE_TOKEN_PROGRAM_ID,
                    vec![funder.into(), balance.into()],
                    &native_token::Instruction::Transfer {
                        amount: funding.amount,
                    },
                )]
            } else {
                if input.accounts.len() != 1 {
                    return Err("unexpected creation handles".into());
                }
                vec![]
            };
            let expiry = create.expires_at_ms;
            let effect = ShardEffect::new(config, &Instruction::Create(create));
            Ok(PlanOutput::new(input)
                .with_effects(vec![effect])
                .with_chained_calls(calls)
                .with_timestamp_validity_window(..expiry))
        }
    }
}

pub fn apply(input: ApplyInput) -> Result<ApplyOutput, String> {
    if input.selector.program_account_id != input.self_account_id {
        return Err("configuration must belong to the executing program".into());
    }
    let instruction: Instruction =
        borsh::from_slice(&input.effect_data).map_err(|_| "malformed lifecycle effect")?;
    match instruction {
        Instruction::Transfer(request) => {
            let mut inner = input.clone();
            inner.effect_data = borsh::to_vec(&request).unwrap();
            let result = vault_core::apply(inner)?;
            // Journal binds the original tagged input expected by LEE.
            Ok(ApplyOutput::new(input, result.post_data))
        }
        Instruction::Create(create) => {
            if !input.pre_data.is_empty() {
                return Err("vault already initialized or configuration is malformed".into());
            }
            validate_create(&create, input.selector.account_id, input.self_account_id)?;
            let approvals = create
                .enrollments
                .iter()
                .map(|a| approval_core::Approval {
                    member_index: a.member_index,
                    signature: a.signature.to_vec(),
                })
                .collect();
            approval_core::verify(&approval_core::Input {
                configuration: enrollment_configuration(&create.descriptor),
                action: enrollment_action(&create, input.self_account_id),
                approvals,
            })
            .map_err(|e| format!("enrollment rejected: {e:?}"))?;
            let state = borsh::to_vec(&initial_state(&create.descriptor))
                .unwrap()
                .try_into()
                .map_err(|_| "vault state too large")?;
            Ok(ApplyOutput::new(input, Some(state)))
        }
    }
}
