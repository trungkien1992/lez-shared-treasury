//! Bounded private-vault transfer prototype. Configuration comes only from the
//! account's committed pre-state; callers supply a transfer and approvals.
use borsh::{BorshDeserialize, BorshSerialize};
use lee_core::{
    account::{AccountId, ProgramShardSelector},
    native_token::{NATIVE_TOKEN_PROGRAM_ID, custody_transfer},
    program::{ApplyInput, ApplyOutput, PdaSeed, PlanInput, PlanOutput, ShardEffect},
};

pub const VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct VaultState {
    pub version: u32,
    pub network_id: [u8; 32],
    pub epoch: u64,
    pub threshold: u32,
    pub member_keys: Vec<[u8; 32]>,
    pub seed: PdaSeed,
    pub next_proposal: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct Transfer {
    pub epoch: u64,
    pub proposal: u64,
    /// LEZ timestamps are Unix milliseconds; the upper bound is exclusive.
    pub expires_at_ms: u64,
    pub seed: PdaSeed,
    pub recipient: AccountId,
    pub amount: u128,
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct Approval {
    pub member_index: u32,
    pub signature: [u8; 64],
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct Request {
    pub transfer: Transfer,
    pub approvals: Vec<Approval>,
}

pub fn configuration(state: &VaultState) -> approval_core::Configuration {
    approval_core::Configuration {
        epoch: state.epoch,
        threshold: state.threshold,
        member_keys: state.member_keys.clone(),
    }
}

pub fn action(
    state: &VaultState,
    vault: AccountId,
    program: AccountId,
    transfer: &Transfer,
) -> approval_core::Action {
    let mut proposal_id = [0u8; 32];
    proposal_id[..8].copy_from_slice(&transfer.proposal.to_le_bytes());
    approval_core::Action {
        network_id: state.network_id,
        instance_id: *vault.value(),
        program_id: *program.value(),
        configuration_epoch: transfer.epoch,
        proposal_id,
        target_program: *NATIVE_TOKEN_PROGRAM_ID.value(),
        expires_at: transfer.expires_at_ms,
        instruction: borsh::to_vec(transfer).expect("fixed transfer serializes"),
    }
}

pub fn plan(input: PlanInput, request: Request) -> Result<PlanOutput, String> {
    let [config, balance, recipient] = input.accounts.as_slice() else {
        return Err("expected vault configuration, vault balance, recipient balance".into());
    };
    let transfer = &request.transfer;
    if config.program_account_id != input.self_account_id
        || balance.program_account_id != NATIVE_TOKEN_PROGRAM_ID
        || recipient.program_account_id != NATIVE_TOKEN_PROGRAM_ID
        || config.account_id != balance.account_id
        || recipient.account_id != transfer.recipient
        || config.account_id == recipient.account_id
        || transfer.amount == 0
        || transfer.expires_at_ms == 0
        || request.approvals.len() > approval_core::MAX_MEMBERS
    {
        return Err("invalid transfer handles or bounds".into());
    }
    let effect = ShardEffect::new(config, &request);
    let call = custody_transfer(
        config.account_id,
        transfer.seed,
        transfer.recipient,
        transfer.amount,
    );
    // LEZ executes and validates every effect before dispatching chained calls.
    // A failing approval apply therefore aborts the entire transaction.
    Ok(PlanOutput::new(input)
        .with_effects(vec![effect])
        .with_chained_calls(vec![call])
        .with_timestamp_validity_window(..transfer.expires_at_ms))
}

pub fn apply(input: ApplyInput) -> Result<ApplyOutput, String> {
    if input.selector.program_account_id != input.self_account_id {
        return Err("configuration must belong to the executing program".into());
    }
    let mut state: VaultState = borsh::from_slice(&input.pre_data)
        .map_err(|_| "missing or malformed committed vault configuration")?;
    let request: Request =
        borsh::from_slice(&input.effect_data).map_err(|_| "malformed approval effect")?;
    let transfer = &request.transfer;
    if state.version != VERSION
        || state.epoch != transfer.epoch
        || state.next_proposal != transfer.proposal
        || state.seed != transfer.seed
        || transfer.amount == 0
        || transfer.expires_at_ms == 0
        || transfer.recipient == input.selector.account_id
    {
        return Err("state version, epoch, sequence, seed or transfer mismatch".into());
    }
    let approval_input = approval_core::Input {
        configuration: configuration(&state),
        action: action(
            &state,
            input.selector.account_id,
            input.self_account_id,
            transfer,
        ),
        approvals: request
            .approvals
            .iter()
            .map(|a| approval_core::Approval {
                member_index: a.member_index,
                signature: a.signature.to_vec(),
            })
            .collect(),
    };
    approval_core::verify(&approval_input).map_err(|e| format!("{e:?}"))?;
    state.next_proposal = state
        .next_proposal
        .checked_add(1)
        .ok_or("proposal counter exhausted")?;
    let data = borsh::to_vec(&state)
        .map_err(|e| e.to_string())?
        .try_into()
        .map_err(|_| "vault state too large")?;
    Ok(ApplyOutput::new(input, Some(data)))
}

pub fn selectors(
    vault: AccountId,
    program: AccountId,
    recipient: AccountId,
) -> Vec<ProgramShardSelector> {
    vec![
        ProgramShardSelector::new(vault, program),
        ProgramShardSelector::native_balance(vault),
        ProgramShardSelector::native_balance(recipient),
    ]
}
