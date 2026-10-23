use crate::{fixture::Fixture, measured};
use anyhow::{Context, Result, ensure};
use lee::program::Program;
use lee_core::{
    account::{AccountId, ShardData},
    program::{AccountMeta, ApplyInput, GuestOutput, PlanInput},
};
use lifecycle_core::transfer as vault_core;
use lifecycle_core::{Create, Instruction};
use risc0_zkvm::{ExecutorEnv, ExitCode, default_executor};
use serde_json::{Value, json};
use std::path::Path;

fn guest(program: &Program, input: &ApplyInput) -> Result<(GuestOutput, u64)> {
    let mut env = ExecutorEnv::builder();
    env.segment_limit_po2(measured::SEGMENT_PO2)
        .session_limit(Some(measured::SESSION_LIMIT));
    Program::write_apply_inputs(input, &mut env)?;
    let session = default_executor().execute(env.build()?, program.elf())?;
    ensure!(
        session.exit_code == ExitCode::Halted(0),
        "guest did not halt successfully"
    );
    Ok((
        borsh::from_slice(lee_core::from_frame(&session.journal.bytes).context("bad journal")?)?,
        session.cycles(),
    ))
}

fn negative_guest(program: &Program, input: &ApplyInput) -> Result<String> {
    let error = guest(program, input).expect_err("invalid lifecycle apply accepted");
    let detail = format!("{error:#}");
    ensure!(
        detail.contains("lifecycle apply rejected"),
        "unexpected executor failure: {detail}"
    );
    Ok(detail)
}

fn apply_for(create: Create, program: AccountId) -> ApplyInput {
    ApplyInput {
        self_account_id: program,
        selector: lee_core::account::ProgramShardSelector::new(
            lifecycle_core::vault_id(&create.descriptor, program),
            program,
        ),
        pre_data: ShardData::empty(),
        effect_data: borsh::to_vec(&Instruction::Create(create)).unwrap(),
    }
}

fn plan_for(create: &Create, program: AccountId) -> PlanInput {
    PlanInput {
        self_account_id: program,
        caller_account_id: None,
        accounts: lifecycle_core::selectors(create, program)
            .iter()
            .map(|s| {
                AccountMeta::new(
                    s.account_id,
                    create
                        .funding
                        .as_ref()
                        .is_some_and(|f| f.from == s.account_id),
                    s.program_account_id,
                )
            })
            .collect(),
        instruction_data: borsh::to_vec(&Instruction::Create(create.clone())).unwrap(),
    }
}

pub fn run(program: &Program, f: &Fixture, dir: &Path) -> Result<Value> {
    let valid = f.create_apply()?;
    let expected = lifecycle_core::apply(valid.clone()).map_err(anyhow::Error::msg)?;
    let (actual, create_cycles) = guest(program, &valid)?;
    ensure!(
        actual == GuestOutput::Apply(expected.clone()),
        "native and guest creation differ"
    );
    let transfer_valid = f.transfer_apply()?;
    let transfer_expected =
        lifecycle_core::apply(transfer_valid.clone()).map_err(anyhow::Error::msg)?;
    let (actual, transfer_cycles) = guest(program, &transfer_valid)?;
    ensure!(
        actual == GuestOutput::Apply(transfer_expected.clone()),
        "native and guest spending differ"
    );
    let mut unfunded = f.create.clone();
    unfunded.funding = None;
    let unfunded_input = apply_for(unfunded.clone(), f.program_id);
    let (unfunded_out, _) = guest(program, &unfunded_input)?;
    let GuestOutput::Apply(unfunded_out) = unfunded_out else {
        anyhow::bail!("expected apply")
    };
    ensure!(
        unfunded_out.post_data == expected.post_data,
        "funding changed enrolled state"
    );
    let unfunded_plan = lifecycle_core::plan(
        plan_for(&unfunded, f.program_id),
        Instruction::Create(unfunded.clone()),
    )
    .map_err(anyhow::Error::msg)?;
    ensure!(
        unfunded_plan.chained_calls.is_empty(),
        "unfunded creation must not transfer"
    );
    let mut cases: Vec<(&str, ApplyInput)> = vec![];
    macro_rules! create_case {
        ($name:expr, $mutate:expr) => {{
            let mut c = f.create.clone();
            ($mutate)(&mut c);
            // Rebind the address after descriptor mutations: old signatures must still fail.
            cases.push(($name, apply_for(c, f.program_id)));
        }};
    }
    create_case!("missing_one_enrollment", |c: &mut Create| {
        c.enrollments.pop();
    });
    create_case!("spending_quorum_is_not_enrollment", |c: &mut Create| c
        .enrollments
        .truncate(5));
    create_case!("no_member_consent", |c: &mut Create| c.enrollments.clear());
    create_case!("duplicate_signer", |c: &mut Create| c.enrollments[1] =
        c.enrollments[0].clone());
    create_case!("extra_enrollment", |c: &mut Create| c
        .enrollments
        .push(c.enrollments[0].clone()));
    create_case!("nonmember_index", |c: &mut Create| c.enrollments[0]
        .member_index = 7);
    create_case!("invalid_signature", |c: &mut Create| c.enrollments[0]
        .signature[0] ^= 1);
    create_case!("changed_network_and_rebound_address", |c: &mut Create| c
        .descriptor
        .network_id[0] ^=
        1);
    create_case!("changed_threshold_and_rebound_address", |c: &mut Create| {
        c.descriptor.threshold = 4
    });
    create_case!("changed_salt_and_rebound_address", |c: &mut Create| c
        .descriptor
        .salt[0] ^=
        1);
    create_case!("reordered_members_and_rebound_address", |c: &mut Create| c
        .descriptor
        .member_keys
        .swap(0, 6));
    create_case!("duplicate_member_keys", |c: &mut Create| c
        .descriptor
        .member_keys[1] =
        c.descriptor.member_keys[0]);
    create_case!("invalid_member_key", |c: &mut Create| c
        .descriptor
        .member_keys[0] =
        [255; 32]);
    create_case!("changed_nullifier_key", |c: &mut Create| c.descriptor.npk =
        (&crate::fixture::nsk(14)).into());
    create_case!("changed_viewing_key", |c: &mut Create| c.descriptor.vpk =
        lee_core::encryption::ViewingPublicKey::from_seed(
            &[33; 32], &[34; 32]
        ));
    create_case!("changed_identifier", |c: &mut Create| c
        .descriptor
        .identifier =
        lee_core::Identifier::new([1; 32]));
    create_case!("changed_expiry", |c: &mut Create| c.expires_at_ms += 1);
    create_case!("zero_expiry", |c: &mut Create| c.expires_at_ms = 0);
    create_case!("zero_threshold", |c: &mut Create| c.descriptor.threshold =
        0);
    create_case!("threshold_exceeds_members", |c: &mut Create| c
        .descriptor
        .threshold =
        8);
    create_case!("unknown_version", |c: &mut Create| c.descriptor.version +=
        1);
    create_case!("empty_member_list", |c: &mut Create| c
        .descriptor
        .member_keys
        .clear());
    create_case!("too_many_members", |c: &mut Create| c
        .descriptor
        .member_keys
        .resize(65, [255; 32]));
    let mut other_vault = valid.clone();
    other_vault.selector.account_id = AccountId::new([60; 32]);
    cases.push(("unrelated_or_prefunded_vault_address", other_vault));
    cases.push((
        "other_program_and_rebound_address",
        apply_for(f.create.clone(), AccountId::new([61; 32])),
    ));
    let mut foreign = valid.clone();
    foreign.selector.program_account_id = AccountId::new([62; 32]);
    cases.push(("foreign_configuration_shard", foreign));
    let mut replay = valid.clone();
    replay.pre_data = expected.post_data.clone().unwrap();
    cases.push(("reinitialize_existing_vault", replay));
    let mut malformed = valid.clone();
    malformed.pre_data = vec![1, 2, 3].try_into()?;
    cases.push(("overwrite_malformed_existing_configuration", malformed));
    let mut bad_effect = valid.clone();
    bad_effect.effect_data.push(0);
    cases.push(("trailing_effect_bytes", bad_effect));
    let mut bad_viewing_key = f.create.clone();
    bad_viewing_key.descriptor.vpk = borsh::from_slice(&borsh::to_vec(&vec![1u8])?)?;
    let mut bad_viewing_input = valid.clone();
    bad_viewing_input.effect_data = borsh::to_vec(&Instruction::Create(bad_viewing_key))?;
    cases.push(("malformed_viewing_key_length", bad_viewing_input));
    macro_rules! transfer_case {
        ($name:expr, $mutate:expr) => {{
            let mut request = f.transfer();
            ($mutate)(&mut request);
            let mut input = transfer_valid.clone();
            input.effect_data = borsh::to_vec(&Instruction::Transfer(request))?;
            cases.push(($name, input));
        }};
    }
    transfer_case!(
        "transfer_insufficient_quorum",
        |r: &mut vault_core::Request| {
            r.approvals.pop();
        }
    );
    transfer_case!("transfer_no_approvals", |r: &mut vault_core::Request| r
        .approvals
        .clear());
    transfer_case!(
        "transfer_duplicate_approval",
        |r: &mut vault_core::Request| r.approvals[1] = r.approvals[0].clone()
    );
    transfer_case!("transfer_changed_amount", |r: &mut vault_core::Request| {
        r.transfer.amount += 1
    });
    transfer_case!(
        "transfer_changed_recipient",
        |r: &mut vault_core::Request| r.transfer.recipient = f.funder
    );
    transfer_case!("transfer_changed_expiry", |r: &mut vault_core::Request| {
        r.transfer.expires_at_ms += 1
    });
    transfer_case!("transfer_stale_epoch", |r: &mut vault_core::Request| r
        .transfer
        .epoch =
        0);
    transfer_case!(
        "enrollment_signatures_cannot_authorize_spending",
        |r: &mut vault_core::Request| r.approvals = f.create.enrollments[..5].to_vec()
    );
    let mut spend_replay = transfer_valid.clone();
    spend_replay.pre_data = transfer_expected.post_data.unwrap();
    cases.push(("replay_consumed_transfer", spend_replay));
    let mut before_creation = transfer_valid;
    before_creation.pre_data = ShardData::empty();
    cases.push(("spending_before_creation", before_creation));
    let mut result = vec![];
    for (name, input) in cases {
        let native =
            lifecycle_core::apply(input.clone()).expect_err("native accepted invalid input");
        let guest = negative_guest(program, &input)?;
        result.push(json!({"case":name, "native_rejected":native, "guest_rejected":guest}));
    }

    let good_plan = plan_for(&f.create, f.program_id);
    let plan = lifecycle_core::plan(good_plan.clone(), Instruction::Create(f.create.clone()))
        .map_err(anyhow::Error::msg)?;
    ensure!(
        plan.timestamp_validity_window == (..crate::fixture::EXPIRES).into(),
        "expiry not propagated"
    );
    let mut plans = vec![];
    let mut unauthorized = good_plan.clone();
    unauthorized.accounts[2].is_authorized = false;
    plans.push((
        "unauthorized_funding_handle",
        unauthorized,
        f.create.clone(),
    ));
    let mut wrong_balance = good_plan.clone();
    wrong_balance.accounts[1].account_id = f.funder;
    plans.push((
        "wrong_vault_balance_handle",
        wrong_balance,
        f.create.clone(),
    ));
    let mut wrong_funder = good_plan.clone();
    wrong_funder.accounts[2].account_id = f.recipient;
    plans.push(("wrong_funding_handle", wrong_funder, f.create.clone()));
    let mut zero = f.create.clone();
    zero.funding.as_mut().unwrap().amount = 0;
    plans.push(("zero_funding_amount", plan_for(&zero, f.program_id), zero));
    let mut extra = plan_for(&unfunded, f.program_id);
    extra.accounts.push(extra.accounts[0].clone());
    plans.push(("extra_unfunded_handle", extra, unfunded));
    let mut plan_results = vec![];
    for (name, mut input, create) in plans {
        input.instruction_data = borsh::to_vec(&Instruction::Create(create.clone()))?;
        let error = lifecycle_core::plan(input.clone(), Instruction::Create(create))
            .expect_err("invalid native plan accepted");
        let mut env = ExecutorEnv::builder();
        env.segment_limit_po2(measured::SEGMENT_PO2)
            .session_limit(Some(measured::SESSION_LIMIT));
        Program::write_plan_inputs(&input, &mut env)?;
        let failure = default_executor()
            .execute(env.build()?, program.elf())
            .expect_err("invalid guest plan accepted");
        let detail = format!("{failure:#}");
        ensure!(
            detail.contains("lifecycle plan rejected"),
            "unexpected executor failure: {detail}"
        );
        plan_results.push(json!({"case":name,"native_rejected":error,"guest_rejected":detail}));
    }

    let mut flow_results = vec![];
    for (name, bad_ask) in [
        ("funding_key_required", true),
        ("funding_balance_required", false),
    ] {
        let mut root = f.create_root()?;
        let mut witnesses = f.witnesses.clone();
        if bad_ask {
            witnesses[1].kind = lee_core::WitnessKind::Regular { ask: None };
        } else {
            let mut create = f.create.clone();
            create.funding.as_mut().unwrap().amount = crate::fixture::INITIAL_FUNDER + 1;
            root.instruction_data = borsh::to_vec(&Instruction::Create(create))?;
        }
        let mut runner = measured::Runner {
            prove: false,
            dir: dir.join(name),
            stages: vec![],
            assumptions: ExecutorEnv::builder(),
        };
        let error = match measured::transaction_output(root, witnesses, program, &mut runner) {
            Ok(_) => anyhow::bail!("invalid funding flow accepted"),
            Err(e) => e,
        };
        let detail = format!("{error:#}");
        ensure!(
            if bad_ask {
                detail.contains("invalid or unauthorized funding source")
            } else {
                matches!(
                    error.downcast_ref::<lee_core::native_token::TransferError>(),
                    Some(lee_core::native_token::TransferError::InsufficientBalance { .. })
                )
            },
            "unexpected flow error: {detail}"
        );
        flow_results.push(json!({"case":name,"rejected":detail,"returned_ppe_output":false}));
    }
    Ok(
        json!({"mode":"execution-only; native and real guest binary",
        "positive_creation_native_guest_match":true,"positive_transfer_native_guest_match":true,
        "unfunded_creation_same_configuration":true,"exclusive_expiry_window_propagated":true,
        "create_apply_user_cycles":create_cycles,"transfer_apply_user_cycles":transfer_cycles,
        "negative_apply_cases":result,"negative_plan_cases":plan_results,"negative_funding_flows":flow_results,
        "proof_generated":false,"settlement_verified":false}),
    )
}
