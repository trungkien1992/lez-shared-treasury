use crate::{fixture::Fixture, measured};
use anyhow::{Result, ensure};
use lee::program::Program;
use lee_core::{
    account::AccountId,
    program::{ApplyInput, GuestOutput},
};
use risc0_zkvm::{ExecutorEnv, ExitCode, default_executor};
use serde_json::{Value, json};
use vault_core::{Request, VaultState};

pub fn run(program: &Program, fixture: &Fixture) -> Result<Value> {
    ensure!((2..=6).contains(&fixture.configuration.threshold),
        "this validation matrix requires a threshold from 2 through 6");
    let valid = fixture.apply_input()?;
    let expected = vault_core::apply(valid.clone()).map_err(anyhow::Error::msg)?;
    let mut env = ExecutorEnv::builder();
    env.segment_limit_po2(measured::SEGMENT_PO2)
        .session_limit(Some(measured::SESSION_LIMIT));
    Program::write_apply_inputs(&valid, &mut env)?;
    let positive = default_executor().execute(env.build()?, program.elf())?;
    ensure!(
        positive.exit_code == ExitCode::Halted(0),
        "valid guest apply failed"
    );
    let decoded: GuestOutput =
        borsh::from_slice(lee_core::from_frame(&positive.journal.bytes).unwrap())?;
    ensure!(
        decoded == GuestOutput::Apply(expected.clone()),
        "native/guest apply differs"
    );
    let mut cases = vec![];
    macro_rules! request_case {
        ($name:expr, $change:expr) => {{
            let mut input = valid.clone();
            let mut request: Request = borsh::from_slice(&input.effect_data)?;
            ($change)(&mut request);
            input.effect_data = borsh::to_vec(&request)?;
            cases.push(($name, input));
        }};
    }
    macro_rules! state_case {
        ($name:expr, $change:expr) => {{
            let mut input = valid.clone();
            let mut state: VaultState = borsh::from_slice(&input.pre_data)?;
            ($change)(&mut state);
            input.pre_data = borsh::to_vec(&state)?.try_into()?;
            cases.push(($name, input));
        }};
    }
    request_case!("insufficient_quorum", |r: &mut Request| {
        r.approvals.pop();
    });
    request_case!("group_secrets_without_approvals", |r: &mut Request| r
        .approvals
        .clear());
    request_case!("duplicate_member", |r: &mut Request| r.approvals[1] =
        r.approvals[0].clone());
    request_case!("nonmember_index", |r: &mut Request| r.approvals[0]
        .member_index = 7);
    request_case!("corrupted_signature", |r: &mut Request| r.approvals[0]
        .signature[0] ^=
        1);
    request_case!("changed_amount", |r: &mut Request| r.transfer.amount += 1);
    request_case!("changed_recipient", |r: &mut Request| r
        .transfer
        .recipient =
        AccountId::new([61; 32]));
    request_case!("changed_expiry", |r: &mut Request| r
        .transfer
        .expires_at_ms += 1);
    request_case!("changed_proposal", |r: &mut Request| r.transfer.proposal +=
        1);
    request_case!("stale_epoch", |r: &mut Request| r.transfer.epoch -= 1);
    request_case!("changed_custody_seed", |r: &mut Request| r.transfer.seed =
        lee_core::program::PdaSeed::new([62; 32]));
    state_case!("other_network", |s: &mut VaultState| s.network_id[0] ^= 1);
    state_case!("current_threshold_increased", |s: &mut VaultState| s
        .threshold +=
        1);
    state_case!("current_members_changed", |s: &mut VaultState| s
        .member_keys
        .swap(0, 6));
    let mut other_vault = valid.clone();
    other_vault.selector.account_id = AccountId::new([63; 32]);
    cases.push(("other_vault", other_vault));
    let mut other_program = valid.clone();
    other_program.self_account_id = AccountId::new([64; 32]);
    other_program.selector.program_account_id = other_program.self_account_id;
    cases.push(("other_program", other_program));
    let mut replay = valid.clone();
    replay.pre_data = expected.post_data.unwrap();
    cases.push(("proposal_already_consumed", replay));
    let mut no_state = valid.clone();
    no_state.pre_data = vec![].try_into()?;
    cases.push(("missing_authoritative_state", no_state));
    let mut foreign = valid;
    foreign.selector.program_account_id = AccountId::new([65; 32]);
    cases.push(("foreign_configuration_shard", foreign));
    let mut results = vec![];
    for (name, input) in cases {
        let native_error =
            vault_core::apply(input.clone()).expect_err("negative native case accepted");
        let guest_error = rejects_guest(program, &input)?;
        results
            .push(json!({"case":name,"native_rejected":native_error,"guest_rejected":guest_error}));
    }
    Ok(
        json!({"threshold":fixture.configuration.threshold,"positive_native_and_guest_match":true,
        "positive_apply_user_cycles":positive.cycles(),"negative_cases":results,
        "mode":"execution-only; real guest binary; no mock receipts"}),
    )
}

fn rejects_guest(program: &Program, input: &ApplyInput) -> Result<String> {
    let mut env = ExecutorEnv::builder();
    env.segment_limit_po2(measured::SEGMENT_PO2)
        .session_limit(Some(measured::SESSION_LIMIT));
    Program::write_apply_inputs(input, &mut env)?;
    match default_executor().execute(env.build()?, program.elf()) {
        Ok(session) => {
            ensure!(
                session.exit_code != ExitCode::Halted(0),
                "invalid approval accepted by guest"
            );
            Ok(format!("{:?}", session.exit_code))
        }
        Err(error) => {
            let detail = format!("{error:#}");
            // An infrastructure failure is not a successful negative test.
            ensure!(
                detail.contains("vault approval rejected"),
                "unexpected executor failure: {detail}"
            );
            Ok(detail)
        }
    }
}
