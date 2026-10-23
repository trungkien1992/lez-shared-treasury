//! Public deterministic test inputs. No fixture account may hold real funds.
use anyhow::{Context, Result, ensure};
use k256::schnorr::SigningKey;
use lee::{V03State, program::Program};
use lee_core::{
    AuthorizationSecretKey, Commitment, DUMMY_COMMITMENT_HASH, EncryptedAccountData,
    EncryptionScheme, Identifier, Nullifier, NullifierPublicKey, NullifierSecretKey,
    NullifierWitness, PrivacyPreservingCircuitOutput, PrivateAccountKind, PrivateWitness,
    SharedSecretKey, WitnessKind,
    account::{Account, AccountId, Nonce, ProgramShardSelector, ShardData},
    encryption::ViewingPublicKey,
    execution_state::RootCall,
    program::ApplyInput,
};
use lifecycle_core::transfer as vault_core;
use lifecycle_core::{Create, Descriptor, Funding, Instruction};

pub const EXPIRES: u64 = 2_000_000_000_000;
pub const INITIAL_FUNDER: u128 = 1000;
pub const DEPOSIT: u128 = 500;
pub const SPEND: u128 = 37;

/// The unchanged pinned node seeds private accounts and immutable program mirrors.
/// The standalone snapshot is exported from its exact testnet_initial_state library.
pub fn base_state() -> Result<V03State> {
    let profile = std::env::var("LIFECYCLE_STATE_PROFILE").unwrap_or_else(|_| "isolated".into());
    ensure!(["isolated", "standalone"].contains(&profile.as_str()), "unknown state profile");
    let state = V03State::new();
    if profile == "isolated" { return Ok(state); }
    let path = std::env::var("LIFECYCLE_GENESIS_STATE_FILE")
        .unwrap_or_else(|_| "reports/standalone-genesis.state.borsh".into());
    Ok(borsh::from_slice(&std::fs::read(path).context("export the pinned standalone genesis snapshot first")?)?)
}

pub fn keys() -> Vec<SigningKey> {
    (1..=7)
        .map(|i| {
            let mut secret = [0; 32];
            secret[31] = i;
            SigningKey::from_bytes(&secret).unwrap()
        })
        .collect()
}

pub fn approve(digest: &[u8; 32], count: usize) -> Vec<vault_core::Approval> {
    keys()
        .iter()
        .take(count)
        .enumerate()
        .map(|(i, key)| vault_core::Approval {
            member_index: i as u32,
            signature: key
                .sign_prehash_with_aux_rand(digest, &[42; 32])
                .unwrap()
                .to_bytes(),
        })
        .collect()
}

pub fn nsk(byte: u8) -> NullifierSecretKey {
    (&AuthorizationSecretKey([byte; 32])).into()
}

pub struct Fixture {
    pub imported_approvals: bool,
    spend_request: Option<vault_core::Request>,
    pub state_profile: String,
    pub program_id: AccountId,
    pub vault: AccountId,
    pub funder: AccountId,
    pub recipient: AccountId,
    pub create: Create,
    pub state: V03State,
    pub funder_pre: Account,
    pub vault_created: Account,
    pub witnesses: Vec<PrivateWitness>,
}

impl Fixture {
    pub fn make(program: &Program) -> Result<Self> {
        let program_id = AccountId::for_shadow_program(&program.id());
        let mut descriptor = Descriptor {
            version: lifecycle_core::ENROLLMENT_VERSION,
            network_id: [77; 32],
            salt: [44; 32],
            threshold: 5,
            member_keys: keys()
                .iter()
                .map(|k| k.verifying_key().to_bytes().into())
                .collect(),
            npk: NullifierPublicKey::from(&nsk(13)),
            vpk: ViewingPublicKey::from_seed(&[31; 32], &[32; 32]),
            identifier: Identifier::ZERO,
        };
        let create_path = std::env::var("LIFECYCLE_CREATE_INSTRUCTION").ok();
        let spend_path = std::env::var("LIFECYCLE_TRANSFER_INSTRUCTION").ok();
        ensure!(create_path.is_some() == spend_path.is_some(),
            "supply both client-exported creation and transfer instructions");
        let imported_create = if let Some(path) = &create_path {
            let Instruction::Create(c) = borsh::from_slice::<Instruction>(&std::fs::read(path)?)? else {
                anyhow::bail!("client creation file is not a creation instruction");
            };
            ensure!(c.descriptor.npk == descriptor.npk && c.descriptor.vpk == descriptor.vpk
                && c.descriptor.identifier == Identifier::ZERO,
                "the local client bridge retains the fixed test-only private account material");
            descriptor = c.descriptor.clone();
            Some(c)
        } else { None };
        let vault = lifecycle_core::vault_id(&descriptor, program_id);
        let funder_npk = NullifierPublicKey::from(&nsk(17));
        let funder_vpk = ViewingPublicKey::from_seed(&[51; 32], &[52; 32]);
        let funder =
            AccountId::for_regular_private_account(&funder_npk, &funder_vpk, Identifier::ZERO);
        let recipient = AccountId::for_regular_private_account(
            &NullifierPublicKey::from(&nsk(38)),
            &ViewingPublicKey::from_seed(&[83; 32], &[84; 32]),
            Identifier::ZERO,
        );
        let mut create = Create {
            descriptor,
            expires_at_ms: EXPIRES,
            enrollments: vec![],
            funding: Some(Funding {
                from: funder,
                amount: DEPOSIT,
            }),
        };
        if let Some(imported) = imported_create {
            ensure!(imported.expires_at_ms == EXPIRES,
                "local bridge requires the recorded test expiry");
            let funding = imported.funding.as_ref().context("local bridge requires funding")?;
            ensure!(funding.from == funder && funding.amount == DEPOSIT,
                "local bridge requires the independent fixture funder and 500 test units");
            create = imported;
        } else {
            create.enrollments = approve(&lifecycle_core::enrollment_digest(&create, program_id), 7);
        }
        let funder_pre = Account {
            nonce: Nonce::private_account_nonce_init(&funder),
            ..Account::funded(INITIAL_FUNDER)
        };
        let commitment = Commitment::new(&funder, &funder_pre);
        // Only the independent funding account exists at genesis. No vault shard is seeded.
        let state = base_state()?
            .with_private_accounts([(commitment, Nullifier::for_account_initialization(&funder))]);
        let witnesses = vec![
            PrivateWitness {
                vpk: create.descriptor.vpk.clone(),
                random_seed: [91; 32],
                identifier: Identifier::ZERO,
                kind: WitnessKind::Pda {
                    binding: (program_id, lifecycle_core::seed(&create.descriptor)),
                },
                nullifier: NullifierWitness::Init {
                    npk: create.descriptor.npk,
                    commitment_root: DUMMY_COMMITMENT_HASH,
                },
            },
            PrivateWitness {
                vpk: funder_vpk.clone(),
                random_seed: [93; 32],
                identifier: Identifier::ZERO,
                kind: WitnessKind::Regular {
                    ask: Some(AuthorizationSecretKey([17; 32])),
                },
                nullifier: NullifierWitness::Update {
                    account: funder_pre.clone(),
                    view_tag: EncryptedAccountData::compute_view_tag(&funder_npk, &funder_vpk),
                    nsk: nsk(17),
                    membership_proof: state.get_proof_for_commitment(&commitment).unwrap(),
                },
            },
        ];
        let vault_created = Account {
            nonce: Nonce::private_account_nonce_init(&vault),
            ..Account::funded(DEPOSIT).with_shard(
                program_id,
                borsh::to_vec(&lifecycle_core::initial_state(&create.descriptor))?.try_into()?,
            )
        };
        let spend_request = if let Some(path) = &spend_path {
            let Instruction::Transfer(r) = borsh::from_slice::<Instruction>(&std::fs::read(path)?)? else {
                anyhow::bail!("client spend file is not a transfer instruction");
            };
            let t = &r.transfer;
            ensure!(t.epoch == 1 && t.proposal == 0 && t.expires_at_ms == EXPIRES
                && t.seed == lifecycle_core::seed(&create.descriptor)
                && t.recipient == recipient && t.amount == SPEND,
                "local bridge requires proposal 0 and 37 test units to the fixture recipient");
            Some(r)
        } else { None };
        let result = Self {
            imported_approvals: create_path.is_some(),
            spend_request,
            state_profile: std::env::var("LIFECYCLE_STATE_PROFILE").unwrap_or_else(|_| "isolated".into()),
            program_id,
            vault,
            funder,
            recipient,
            create,
            state,
            funder_pre,
            vault_created,
            witnesses,
        };
        if result.imported_approvals {
            lifecycle_core::apply(result.create_apply()?)
                .map_err(|e| anyhow::anyhow!("client enrollment failed native guest checks: {e}"))?;
            lifecycle_core::apply(result.transfer_apply()?)
                .map_err(|e| anyhow::anyhow!("client transfer failed native guest checks: {e}"))?;
        }
        Ok(result)
    }

    pub fn create_root(&self) -> Result<RootCall> {
        Ok(RootCall {
            program_account_id: self.program_id,
            shard_selectors: lifecycle_core::selectors(&self.create, self.program_id),
            instruction_data: borsh::to_vec(&Instruction::Create(self.create.clone()))?,
            authorized_accounts: vec![],
        })
    }

    pub fn create_apply(&self) -> Result<ApplyInput> {
        Ok(ApplyInput {
            self_account_id: self.program_id,
            selector: ProgramShardSelector::new(self.vault, self.program_id),
            pre_data: ShardData::empty(),
            effect_data: borsh::to_vec(&Instruction::Create(self.create.clone()))?,
        })
    }

    pub fn transfer(&self) -> vault_core::Request {
        if let Some(r) = &self.spend_request { return r.clone(); }
        let state = lifecycle_core::initial_state(&self.create.descriptor);
        let transfer = vault_core::Transfer {
            epoch: 1,
            proposal: 0,
            expires_at_ms: EXPIRES,
            seed: state.seed,
            recipient: self.recipient,
            amount: SPEND,
        };
        let digest = approval_core::action_digest(
            &vault_core::configuration(&state),
            &vault_core::action(&state, self.vault, self.program_id, &transfer),
        );
        vault_core::Request {
            transfer,
            approvals: approve(&digest, 5),
        }
    }

    pub fn transfer_apply(&self) -> Result<ApplyInput> {
        Ok(ApplyInput {
            self_account_id: self.program_id,
            selector: ProgramShardSelector::new(self.vault, self.program_id),
            pre_data: self.vault_created.data.shard(self.program_id).clone(),
            effect_data: borsh::to_vec(&Instruction::Transfer(self.transfer()))?,
        })
    }

    pub fn spend_inputs(
        &self,
        state: &V03State,
        vault_pre: Account,
    ) -> Result<(RootCall, Vec<PrivateWitness>)> {
        ensure!(vault_pre == self.vault_created, "creation output changed");
        let d = &self.create.descriptor;
        let vault_commitment = Commitment::new(&self.vault, &vault_pre);
        let witnesses = vec![
            PrivateWitness {
                vpk: d.vpk.clone(),
                random_seed: [94; 32],
                identifier: d.identifier,
                kind: WitnessKind::Pda {
                    binding: (self.program_id, lifecycle_core::seed(d)),
                },
                nullifier: NullifierWitness::Update {
                    account: vault_pre,
                    view_tag: EncryptedAccountData::compute_view_tag(&d.npk, &d.vpk),
                    nsk: nsk(13),
                    membership_proof: state
                        .get_proof_for_commitment(&vault_commitment)
                        .context("created commitment absent")?,
                },
            },
            PrivateWitness {
                vpk: ViewingPublicKey::from_seed(&[83; 32], &[84; 32]),
                random_seed: [95; 32],
                identifier: Identifier::ZERO,
                kind: WitnessKind::Regular { ask: None },
                nullifier: NullifierWitness::Init {
                    npk: NullifierPublicKey::from(&nsk(38)),
                    commitment_root: DUMMY_COMMITMENT_HASH,
                },
            },
        ];
        Ok((
            RootCall {
                program_account_id: self.program_id,
                shard_selectors: vault_core::selectors(self.vault, self.program_id, self.recipient),
                instruction_data: borsh::to_vec(&Instruction::Transfer(self.transfer()))?,
                authorized_accounts: vec![],
            },
            witnesses,
        ))
    }

    pub fn check_bootstrap(&self, output: &PrivacyPreservingCircuitOutput) -> Result<()> {
        ensure!(
            output.private_actions.len() == 1 && output.public_actions.len() == 1,
            "unexpected native funding action count"
        );
        check_note(
            output,
            self.funder,
            &self.funder_pre,
            &PrivateAccountKind::Regular(Identifier::ZERO),
            Nullifier::for_account_initialization(&self.funder),
            51,
            52,
        )?;
        Ok(())
    }

    pub fn check_created(&self, output: &PrivacyPreservingCircuitOutput) -> Result<Account> {
        check_private(output, 2)?;
        let vault = check_note(
            output,
            self.vault,
            &self.vault_created,
            &PrivateAccountKind::Pda {
                account_id: self.program_id,
                seed: lifecycle_core::seed(&self.create.descriptor),
                identifier: Identifier::ZERO,
            },
            Nullifier::for_account_initialization(&self.vault),
            31,
            32,
        )?;
        let funder_post = Account {
            nonce: self
                .funder_pre
                .nonce
                .private_account_nonce_increment(&nsk(17)),
            ..Account::funded(INITIAL_FUNDER - DEPOSIT)
        };
        check_note(
            output,
            self.funder,
            &funder_post,
            &PrivateAccountKind::Regular(Identifier::ZERO),
            Nullifier::for_account_update(
                &Commitment::new(&self.funder, &self.funder_pre),
                &nsk(17),
            ),
            51,
            52,
        )?;
        Ok(vault)
    }

    pub fn check_spent(&self, output: &PrivacyPreservingCircuitOutput) -> Result<()> {
        check_private(output, 2)?;
        let mut state = lifecycle_core::initial_state(&self.create.descriptor);
        state.next_proposal = 1;
        let vault_post = Account {
            nonce: self
                .vault_created
                .nonce
                .private_account_nonce_increment(&nsk(13)),
            ..Account::funded(DEPOSIT - SPEND)
                .with_shard(self.program_id, borsh::to_vec(&state)?.try_into()?)
        };
        check_note(
            output,
            self.vault,
            &vault_post,
            &PrivateAccountKind::Pda {
                account_id: self.program_id,
                seed: state.seed,
                identifier: Identifier::ZERO,
            },
            Nullifier::for_account_update(
                &Commitment::new(&self.vault, &self.vault_created),
                &nsk(13),
            ),
            31,
            32,
        )?;
        let recipient_post = Account {
            nonce: Nonce::private_account_nonce_init(&self.recipient),
            ..Account::funded(SPEND)
        };
        check_note(
            output,
            self.recipient,
            &recipient_post,
            &PrivateAccountKind::Regular(Identifier::ZERO),
            Nullifier::for_account_initialization(&self.recipient),
            83,
            84,
        )?;
        Ok(())
    }
}

fn check_private(output: &PrivacyPreservingCircuitOutput, count: usize) -> Result<()> {
    ensure!(
        output.public_actions.is_empty() && output.program_image_claims.is_empty(),
        "expected private accounts and shadow program"
    );
    ensure!(
        output.private_actions.len() == count,
        "unexpected note count"
    );
    ensure!(
        output.timestamp_validity_window == (..EXPIRES).into(),
        "expiry window changed"
    );
    Ok(())
}

fn check_note(
    output: &PrivacyPreservingCircuitOutput,
    id: AccountId,
    expected: &Account,
    kind: &PrivateAccountKind,
    nullifier: Nullifier,
    d: u8,
    z: u8,
) -> Result<Account> {
    let commitment = Commitment::new(&id, expected);
    ensure!(
        output
            .private_actions
            .iter()
            .any(|a| a.commitment == commitment),
        "missing commitment"
    );
    let note = output
        .private_actions
        .iter()
        .find(|a| a.nullifier == nullifier)
        .context("missing nullifier")?;
    let key = SharedSecretKey::decapsulate(&note.encrypted_post_state.epk, &[d; 32], &[z; 32])
        .context("note decapsulation failed")?;
    let (actual_kind, account) =
        EncryptionScheme::decrypt(&note.encrypted_post_state.ciphertext, &key, &nullifier)
            .context("note decryption failed")?;
    ensure!(
        &actual_kind == kind && &account == expected,
        "unexpected decrypted account"
    );
    Ok(account)
}
