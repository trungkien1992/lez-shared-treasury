//! PUBLIC deterministic fixtures. None of these keys may hold real assets.
use anyhow::{Result, ensure};
use k256::schnorr::SigningKey;
use lee::{AccountId, PublicKey, V03State, program::Program};
use lee_core::{
    AuthorizationSecretKey, Commitment, DUMMY_COMMITMENT_HASH, EncryptedAccountData,
    EncryptionScheme, Identifier, Nullifier, NullifierPublicKey, NullifierSecretKey,
    NullifierWitness, PrivacyPreservingCircuitOutput, PrivateAccountKind, PrivateWitness,
    SharedSecretKey, WitnessKind,
    account::{Account, Nonce},
    encryption::ViewingPublicKey,
    execution_state::RootCall,
    program::{ApplyInput, PdaSeed},
};
use vault_core::{Approval, Request, Transfer, VaultState};

pub const EXPIRES: u64 = 2_000_000_000_000;
pub const BALANCE: u128 = 1000;
pub const AMOUNT: u128 = 37;

pub struct Fixture {
    pub state: V03State,
    pub vault: AccountId,
    pub recipient: AccountId,
    pub program_id: AccountId,
    pub vault_pre: Account,
    pub configuration: VaultState,
    pub request: Request,
    pub witnesses: Vec<PrivateWitness>,
    pub member_ids: Vec<AccountId>,
}

pub fn make(program: &Program, threshold: u32) -> Result<Fixture> {
    ensure!((1..=7).contains(&threshold), "threshold must be 1..=7");
    let program_id = AccountId::for_shadow_program(&program.id());
    let seed = PdaSeed::new([44; 32]);
    let ask = AuthorizationSecretKey([13; 32]);
    let nsk: NullifierSecretKey = (&ask).into();
    let npk = NullifierPublicKey::from(&nsk);
    let vpk = ViewingPublicKey::from_seed(&[31; 32], &[32; 32]);
    let vault = AccountId::for_private_pda(&program_id, &seed, &npk, &vpk, Identifier::ZERO);
    let recipient_ask = AuthorizationSecretKey([38; 32]);
    let recipient_nsk: NullifierSecretKey = (&recipient_ask).into();
    let recipient_npk = NullifierPublicKey::from(&recipient_nsk);
    let recipient_vpk = ViewingPublicKey::from_seed(&[83; 32], &[84; 32]);
    let recipient =
        AccountId::for_regular_private_account(&recipient_npk, &recipient_vpk, Identifier::ZERO);
    let keys: Vec<_> = (1..=7)
        .map(|i| {
            let mut key = [0; 32];
            key[31] = i;
            SigningKey::from_bytes(&key).unwrap()
        })
        .collect();
    let member_keys: Vec<[u8; 32]> = keys
        .iter()
        .map(|k| k.verifying_key().to_bytes().into())
        .collect();
    let member_ids = member_keys
        .iter()
        .map(|k| PublicKey::try_new(*k).map(|pk| AccountId::from(&pk)))
        .collect::<Result<Vec<_>, _>>()?;
    let configuration = VaultState {
        version: vault_core::VERSION,
        network_id: [77; 32],
        epoch: 1,
        threshold,
        member_keys,
        seed,
        next_proposal: 0,
    };
    let vault_pre = Account {
        nonce: Nonce::private_account_nonce_init(&vault),
        ..Account::funded(BALANCE)
            .with_shard(program_id, borsh::to_vec(&configuration)?.try_into()?)
    };
    let commitment = Commitment::new(&vault, &vault_pre);
    let state = V03State::new()
        .with_private_accounts([(commitment, Nullifier::for_account_initialization(&vault))]);
    let witnesses = vec![
        PrivateWitness {
            vpk: vpk.clone(),
            random_seed: [91; 32],
            identifier: Identifier::ZERO,
            kind: WitnessKind::Pda {
                binding: (program_id, seed),
            },
            nullifier: NullifierWitness::Update {
                account: vault_pre.clone(),
                view_tag: EncryptedAccountData::compute_view_tag(&npk, &vpk),
                nsk,
                membership_proof: state.get_proof_for_commitment(&commitment).unwrap(),
            },
        },
        PrivateWitness {
            vpk: recipient_vpk,
            random_seed: [92; 32],
            identifier: Identifier::ZERO,
            kind: WitnessKind::Regular { ask: None },
            nullifier: NullifierWitness::Init {
                npk: recipient_npk,
                commitment_root: DUMMY_COMMITMENT_HASH,
            },
        },
    ];
    let transfer = Transfer {
        epoch: 1,
        proposal: 0,
        expires_at_ms: EXPIRES,
        seed,
        recipient,
        amount: AMOUNT,
    };
    let digest = approval_core::action_digest(
        &vault_core::configuration(&configuration),
        &vault_core::action(&configuration, vault, program_id, &transfer),
    );
    let approvals = keys
        .iter()
        .take(threshold as usize)
        .enumerate()
        .map(|(i, k)| Approval {
            member_index: i as u32,
            signature: k
                .sign_prehash_with_aux_rand(&digest, &[42; 32])
                .unwrap()
                .to_bytes(),
        })
        .collect();
    Ok(Fixture {
        state,
        vault,
        recipient,
        program_id,
        vault_pre,
        configuration,
        request: Request {
            transfer,
            approvals,
        },
        witnesses,
        member_ids,
    })
}

impl Fixture {
    pub fn root(&self) -> Result<RootCall> {
        Ok(RootCall {
            program_account_id: self.program_id,
            shard_selectors: vault_core::selectors(self.vault, self.program_id, self.recipient),
            instruction_data: borsh::to_vec(&self.request)?,
            authorized_accounts: vec![],
        })
    }
    pub fn apply_input(&self) -> Result<ApplyInput> {
        Ok(ApplyInput {
            self_account_id: self.program_id,
            selector: lee_core::account::ProgramShardSelector::new(self.vault, self.program_id),
            pre_data: self.vault_pre.data.shard(self.program_id).clone(),
            effect_data: borsh::to_vec(&self.request)?,
        })
    }
    pub fn check_output(&self, output: &PrivacyPreservingCircuitOutput) -> Result<()> {
        ensure!(
            output.public_actions.is_empty(),
            "expected fully private account effects"
        );
        ensure!(
            output.program_image_claims.is_empty(),
            "shadow program must not disclose image claim"
        );
        ensure!(
            output.private_actions.len() == 2,
            "expected exactly two private notes"
        );
        let nsk: NullifierSecretKey = (&AuthorizationSecretKey([13; 32])).into();
        let mut expected_configuration = self.configuration.clone();
        expected_configuration.next_proposal += 1;
        let expected_vault = Account {
            nonce: self.vault_pre.nonce.private_account_nonce_increment(&nsk),
            ..Account::funded(BALANCE - AMOUNT).with_shard(
                self.program_id,
                borsh::to_vec(&expected_configuration)?.try_into()?,
            )
        };
        let expected_recipient = Account {
            nonce: Nonce::private_account_nonce_init(&self.recipient),
            ..Account::funded(AMOUNT)
        };
        let expected = [
            (
                self.vault,
                expected_vault,
                PrivateAccountKind::Pda {
                    account_id: self.program_id,
                    seed: self.configuration.seed,
                    identifier: Identifier::ZERO,
                },
                [31; 32],
                [32; 32],
            ),
            (
                self.recipient,
                expected_recipient,
                PrivateAccountKind::Regular(Identifier::ZERO),
                [83; 32],
                [84; 32],
            ),
        ];
        for (id, account, kind, d, z) in expected {
            let commitment = Commitment::new(&id, &account);
            // LEZ independently sorts commitments and nullifier/note pairs.
            // A note's array position does not link it to that row's commitment.
            ensure!(output.private_actions.iter().any(|a| a.commitment == commitment),
                "missing expected post-state commitment");
            let expected_nullifier = if id == self.vault {
                Nullifier::for_account_update(&Commitment::new(&self.vault, &self.vault_pre), &nsk)
            } else {
                Nullifier::for_account_initialization(&id)
            };
            let action = output
                .private_actions
                .iter()
                .find(|a| a.nullifier == expected_nullifier)
                .ok_or_else(|| anyhow::anyhow!("missing expected nullifier and note"))?;
            let key = SharedSecretKey::decapsulate(&action.encrypted_post_state.epk, &d, &z)
                .ok_or_else(|| anyhow::anyhow!("cannot decapsulate note"))?;
            let (actual_kind, actual_account) = EncryptionScheme::decrypt(
                &action.encrypted_post_state.ciphertext,
                &key,
                &action.nullifier,
            )
            .ok_or_else(|| anyhow::anyhow!("cannot decrypt note"))?;
            ensure!(
                actual_kind == kind && actual_account == account,
                "decrypted account differs from expected transition"
            );
        }
        Ok(())
    }
}
