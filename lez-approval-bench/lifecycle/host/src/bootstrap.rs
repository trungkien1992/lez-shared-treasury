//! Real native-token funding for the local standalone-sequencer fixture.
//! The public source is funded only in the explicitly isolated node's genesis.
use crate::{fixture, measured, proof};
use anyhow::{Context, Result, ensure};
use lee::{
    AccountId, PrivacyPreservingTransaction, PrivateKey, PublicKey,
    privacy_preserving_transaction::{circuit::Proof, message::Message, witness_set::WitnessSet},
    program::Program,
};
use lee_core::{
    Commitment, DUMMY_COMMITMENT_HASH, NullifierPublicKey, NullifierWitness, PrivateWitness,
    WitnessKind,
    account::{Account, ProgramShardSelector},
    execution_state::RootCall,
    native_token::{self, NATIVE_TOKEN_PROGRAM_ID},
};
use risc0_zkvm::{ExecutorEnv, Receipt};
use serde_json::{Value, json};
use std::{fs, path::Path};

const PUBLIC_SOURCE_BALANCE: u128 = 1_000_000;

pub fn run(mode: &str, program: &Program, f: &fixture::Fixture, dir: &Path) -> Result<Value> {
    let key = PrivateKey::try_new([71; 32]).context("invalid fixed test key")?;
    let sender = AccountId::from(&PublicKey::new_from_private_key(&key));
    let public_pre = Account::funded(PUBLIC_SOURCE_BALANCE);
    let state = fixture::base_state()?.with_public_accounts([(sender, public_pre.clone())]);
    let commitment = Commitment::new(&f.funder, &f.funder_pre);
    let config = json!({"public_sender_id":sender.to_string(),"public_sender_nonce":public_pre.nonce,
        "genesis_supply":{"supply_account":{"account_id":sender.to_string(),"balance":PUBLIC_SOURCE_BALANCE}},
        "funding_amount":fixture::INITIAL_FUNDER,"private_funder_id":f.funder.to_string(),
        "expected_funder_commitment":commitment,"expected_funder_commitment_root":f.state.commitment_root(),
        "test_only_public_private_key":true,"state_profile":f.state_profile,
        "expected_genesis_private_root":fixture::base_state()?.commitment_root(),"sequencer_submission":false});
    if mode == "bootstrap-config" {
        return Ok(config);
    }
    let checking = mode == "check-bootstrap";
    let prior_ppe_receipt = dir.join("ppe-composite.receipt").exists();
    let (output, receipt, tx, stages) = if checking {
        let receipt: Receipt = bincode::deserialize(&fs::read(dir.join("ppe-succinct.receipt"))?)?;
        measured::reject_fake(&receipt)?;
        receipt.verify(lee::PRIVACY_PRESERVING_CIRCUIT_ID)?;
        let output: lee_core::PrivacyPreservingCircuitOutput = borsh::from_slice(
            lee_core::from_frame(&receipt.journal.bytes).context("invalid bootstrap journal")?,
        )?;
        let tx: PrivacyPreservingTransaction =
            borsh::from_slice(&fs::read(dir.join("transaction-succinct.borsh"))?)?;
        ensure!(
            tx.message == Message::from_circuit_output(vec![public_pre.nonce], borsh::from_slice(&borsh::to_vec(&output)?)?),
            "bootstrap receipt/message mismatch"
        );
        ensure!(
            tx.witness_set.proof().clone().into_inner() == borsh::to_vec(&receipt.inner)?,
            "bootstrap transaction proof changed"
        );
        (output, receipt, tx, vec![])
    } else {
        let root = RootCall {
            program_account_id: NATIVE_TOKEN_PROGRAM_ID,
            shard_selectors: vec![
                ProgramShardSelector::native_balance(sender),
                ProgramShardSelector::native_balance(f.funder),
            ],
            authorized_accounts: vec![sender],
            instruction_data: borsh::to_vec(&native_token::Instruction::Transfer {
                amount: fixture::INITIAL_FUNDER,
            })?,
        };
        let witnesses = vec![PrivateWitness {
            vpk: f.witnesses[1].vpk.clone(),
            random_seed: [97; 32],
            identifier: f.witnesses[1].identifier,
            kind: WitnessKind::Regular { ask: None },
            nullifier: NullifierWitness::Init {
                npk: NullifierPublicKey::from(&fixture::nsk(17)),
                commitment_root: DUMMY_COMMITMENT_HASH,
            },
        }];
        let mut runner = measured::Runner {
            prove: mode != "bootstrap-execute",
            dir: dir.to_path_buf(),
            stages: vec![],
            assumptions: ExecutorEnv::builder(),
        };
        let (output, receipt) =
            measured::transaction_output(root, witnesses, program, &mut runner)?;
        if mode == "bootstrap-execute" {
            ensure!(receipt.is_none(), "execution-only native funding returned a receipt");
            f.check_bootstrap(&output)?;
            return Ok(json!({"mode":"execution-only-native-funding", "config":config,
                "private_funder_note_decrypted":true,"public_effects":output.public_actions.len(),
                "stages":runner.stages,"proof_generated":false,"settlement_verified":false,
                "sequencer_submission":false}));
        }
        let receipt = receipt.context("bootstrap returned no real receipt")?;
        f.check_bootstrap(&output)?;
        let receipt = measured::compress(&receipt, dir)?.0;
        let message = Message::from_circuit_output(vec![public_pre.nonce], borsh::from_slice(&borsh::to_vec(&output)?)?);
        let witness = WitnessSet::for_message(
            &message,
            Proof::from_inner(borsh::to_vec(&receipt.inner)?),
            &[&key],
        );
        let tx = PrivacyPreservingTransaction::new(message, witness);
        (output, receipt, tx, runner.stages)
    };
    measured::reject_fake(&receipt)?;
    receipt.verify(lee::PRIVACY_PRESERVING_CIRCUIT_ID)?;
    f.check_bootstrap(&output)?;
    let mut accepted = state.clone();
    accepted.transition_from_privacy_preserving_transaction(&tx, 1, fixture::EXPIRES - 1)?;
    let public_post = accepted.get_account_by_id(sender);
    ensure!(
        public_post.data.native_balance()? == PUBLIC_SOURCE_BALANCE - fixture::INITIAL_FUNDER,
        "incorrect public funding debit"
    );
    ensure!(
        accepted.commitment_root() == f.state.commitment_root(),
        "native funding root differs from lifecycle fixture"
    );
    ensure!(
        accepted.get_proof_for_commitment(&commitment)
            == f.state.get_proof_for_commitment(&commitment),
        "native funding membership witness differs from lifecycle fixture"
    );
    let after = borsh::to_vec(&accepted)?;
    let replay = accepted
        .transition_from_privacy_preserving_transaction(&tx, 2, fixture::EXPIRES - 1)
        .expect_err("native funding replay accepted");
    ensure!(
        borsh::to_vec(&accepted)? == after,
        "native funding replay changed state"
    );
    let encoding = if checking {
        json!({"saved_transaction_verified":true})
    } else {
        proof::save_transaction(&tx, dir, "succinct")?
    };
    Ok(
        json!({"mode":mode,"config":config,"real_receipt_verified":true,
        "private_funder_note_decrypted":true,"native_funding_root_matches_lifecycle":true,
        "native_funding_membership_matches_lifecycle":true,"public_balance_after":public_post.data.native_balance()?,
        "replay_rejected":format!("{replay:?}"),"encoding":encoding,"stages":stages,
        "transaction_hash":hex::encode(tx.hash()),"commitments":tx.message.commitments(),
        "checkpoint_reused":prior_ppe_receipt,"sequencer_submission":false,"live_devnet_submission":false}),
    )
}
