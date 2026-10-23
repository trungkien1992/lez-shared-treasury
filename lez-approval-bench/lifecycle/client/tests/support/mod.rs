use lee_core::{NullifierWitness, PrivateWitness, account::Account};
use lifecycle_core::{Descriptor, Instruction, transfer::VaultState};
use std::{
    fs,
    path::{Path, PathBuf},
};
use treasury_client::state_import::{
    AcceptedStep, CHECKPOINT_FORMAT, EVIDENCE_VERSION, StateEvidence, TrustedCheckpoint,
};
use treasury_client::{Context, GUEST_SHA256, hash, program_id};

pub struct Fixture {
    pub evidence: StateEvidence,
    pub checkpoint: TrustedCheckpoint,
    pub nullifier_key: [u8; 32],
}

pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Historical local-node fixtures with already public test-only LEZ key material.
pub fn fixture(member_client: bool, include_payment: bool, current_note: bool) -> Fixture {
    let base = root();
    let report = base.join(if member_client {
        "reports/2026-10-13-client-real-lifecycle"
    } else {
        "reports/2026-10-12-standalone-lifecycle"
    });
    let descriptor: Descriptor = if member_client {
        let c: Context = serde_json::from_slice(
            &fs::read(base.join("client/reports/2026-10-13-member-workflow/context.json")).unwrap(),
        )
        .unwrap();
        borsh::from_slice(&hex::decode(c.descriptor_hex).unwrap()).unwrap()
    } else {
        let bytes =
            fs::read(base.join("reports/2026-10-12-standalone-execution/create-instruction.borsh"))
                .unwrap();
        let Instruction::Create(c) = borsh::from_slice(&bytes).unwrap() else {
            panic!("creation fixture")
        };
        c.descriptor
    };
    let witnesses: Vec<PrivateWitness> =
        borsh::from_slice(&fs::read(report.join("accepted-spend-witnesses.borsh")).unwrap())
            .unwrap();
    let NullifierWitness::Update { account, nsk, .. } = &witnesses[0].nullifier else {
        panic!("vault update witness")
    };
    let mut account: Account = account.clone();
    let program = program_id().unwrap();
    if current_note {
        assert!(include_payment);
        let mut state: VaultState = borsh::from_slice(account.data.shard(program)).unwrap();
        state.next_proposal = 1;
        account
            .data
            .set_shard(program, borsh::to_vec(&state).unwrap().try_into().unwrap());
        account.data.set_shard(
            lee_core::native_token::NATIVE_TOKEN_PROGRAM_ID,
            lee_core::native_token::encode_balance(463),
        );
        account.nonce = account.nonce.private_account_nonce_increment(nsk);
    }
    // The archive snapshot contains builtins, while the actual standalone node
    // additionally receives the genesis supply in its explicit configuration.
    // The old private-workflow head snapshots omit that unrelated public shard.
    // Reconstruct both sides of the SAME full node state, including its debit.
    let bootstrap: serde_json::Value = serde_json::from_slice(
        &fs::read(base.join("reports/2026-10-12-standalone-funding/bootstrap.json")).unwrap(),
    )
    .unwrap();
    let sender: lee_core::account::AccountId = bootstrap["config"]["public_sender_id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let supply = bootstrap["config"]["genesis_supply"]["supply_account"]["balance"]
        .as_u64()
        .unwrap() as u128;
    let funding = bootstrap["config"]["funding_amount"].as_u64().unwrap() as u128;
    let exported: lee::V03State =
        borsh::from_slice(&fs::read(base.join("reports/standalone-genesis.state.borsh")).unwrap())
            .unwrap();
    let base_state =
        borsh::to_vec(&exported.with_public_accounts([(sender, Account::funded(supply))])).unwrap();
    let mut paths = vec![
        base.join("reports/2026-10-12-standalone-funding/transaction-succinct.borsh"),
        report.join("create/transaction-succinct.borsh"),
    ];
    if include_payment {
        paths.push(report.join("spend/transaction-succinct.borsh"));
    }
    let steps = paths
        .iter()
        .enumerate()
        .map(|(i, path)| AcceptedStep {
            block_id: i as u64 + 2,
            timestamp_ms: 1000 + i as u64,
            transaction: fs::read(path).unwrap(),
        })
        .collect();
    let context = Context::from_descriptor(&descriptor).unwrap();
    let (_, vault, _, _) = context.parts().unwrap();
    let archived_head = fs::read(report.join(if include_payment {
        "after-spend.state.borsh"
    } else {
        "after-creation.state.borsh"
    }))
    .unwrap();
    let head: lee::V03State = borsh::from_slice(&archived_head).unwrap();
    let mut public_post = Account::funded(supply - funding);
    public_post.nonce =
        (bootstrap["config"]["public_sender_nonce"].as_u64().unwrap() as u128 + 1).into();
    let head = borsh::to_vec(&head.with_public_accounts([(sender, public_post)])).unwrap();
    let checkpoint = TrustedCheckpoint {
        format: CHECKPOINT_FORMAT.into(),
        guest_sha256: GUEST_SHA256.into(),
        network_domain_hex: hex::encode(descriptor.network_id),
        vault_hex: hex::encode(vault.value()),
        base_state_sha256: hex::encode(hash(&base_state)),
        base_block_id: 1,
        head_state_sha256: hex::encode(hash(&head)),
        head_block_id: if include_payment { 4 } else { 3 },
        observed_at_ms: 10_000,
        valid_until_ms: 20_000,
        minimum_proposal_counter: u64::from(current_note),
    };
    Fixture {
        evidence: StateEvidence {
            version: EVIDENCE_VERSION,
            descriptor,
            vault_account: account,
            base_state,
            steps,
        },
        checkpoint,
        nullifier_key: *nsk,
    }
}
