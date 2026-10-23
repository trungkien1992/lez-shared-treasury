//! Standalone approval verifier prototype. Not a deployed LEZ multisig.
//! Authoritative account state, account-key ownership, time, custody, and replay
//! storage must be supplied by the eventual LEZ program integration.
use std::collections::BTreeSet;

use k256::schnorr::{signature::hazmat::PrehashVerifier, Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const MAX_MEMBERS: usize = 64;
pub const MAX_ACTION_BYTES: usize = 4096;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Configuration {
    pub epoch: u64,
    pub threshold: u32,
    pub member_keys: Vec<[u8; 32]>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Action {
    pub network_id: [u8; 32],
    pub instance_id: [u8; 32],
    pub program_id: [u8; 32],
    pub configuration_epoch: u64,
    pub proposal_id: [u8; 32],
    pub target_program: [u8; 32],
    pub expires_at: u64,
    pub instruction: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Approval {
    pub member_index: u32,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Input {
    pub configuration: Configuration,
    pub action: Action,
    pub approvals: Vec<Approval>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifiedApprovalSet {
    pub configuration_digest: [u8; 32],
    pub action_digest: [u8; 32],
    pub threshold: u32,
    pub member_count: u32,
    pub signer_indices: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidConfiguration,
    DuplicateMember,
    InvalidMemberKey,
    StaleConfiguration,
    ActionTooLarge,
    InsufficientApprovals,
    TooManyApprovals,
    InvalidMemberIndex,
    DuplicateApproval,
    InvalidSignature,
}

pub fn configuration_digest(config: &Configuration) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"LEZ-RFP005/configuration/v1\0");
    h.update(config.epoch.to_le_bytes());
    h.update(config.threshold.to_le_bytes());
    h.update((config.member_keys.len() as u32).to_le_bytes());
    for key in &config.member_keys {
        h.update(key);
    }
    h.finalize().into()
}

/// Explicit, length-delimited application message. This is not LEZ wire format.
pub fn action_digest(config: &Configuration, action: &Action) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"LEZ-RFP005/approval/v1\0");
    h.update(configuration_digest(config));
    h.update(action.network_id);
    h.update(action.instance_id);
    h.update(action.program_id);
    h.update(action.configuration_epoch.to_le_bytes());
    h.update(action.proposal_id);
    h.update(action.target_program);
    h.update(action.expires_at.to_le_bytes());
    h.update((action.instruction.len() as u32).to_le_bytes());
    h.update(&action.instruction);
    h.finalize().into()
}

pub fn verify(input: &Input) -> Result<VerifiedApprovalSet, Error> {
    let c = &input.configuration;
    let n = c.member_keys.len();
    if n == 0 || n > MAX_MEMBERS || c.threshold == 0 || c.threshold as usize > n {
        return Err(Error::InvalidConfiguration);
    }
    let mut unique_keys = BTreeSet::new();
    for bytes in &c.member_keys {
        if !unique_keys.insert(*bytes) {
            return Err(Error::DuplicateMember);
        }
        VerifyingKey::from_bytes(bytes).map_err(|_| Error::InvalidMemberKey)?;
    }
    if input.action.configuration_epoch != c.epoch {
        return Err(Error::StaleConfiguration);
    }
    if input.action.instruction.len() > MAX_ACTION_BYTES {
        return Err(Error::ActionTooLarge);
    }
    if input.approvals.len() < c.threshold as usize {
        return Err(Error::InsufficientApprovals);
    }
    if input.approvals.len() > n {
        return Err(Error::TooManyApprovals);
    }

    let digest = action_digest(c, &input.action);
    let mut signers = BTreeSet::new();
    for approval in &input.approvals {
        let index = approval.member_index as usize;
        let bytes = c.member_keys.get(index).ok_or(Error::InvalidMemberIndex)?;
        if !signers.insert(approval.member_index) {
            return Err(Error::DuplicateApproval);
        }
        let key = VerifyingKey::from_bytes(bytes).map_err(|_| Error::InvalidMemberKey)?;
        let sig = Signature::try_from(approval.signature.as_slice())
            .map_err(|_| Error::InvalidSignature)?;
        key.verify_prehash(&digest, &sig)
            .map_err(|_| Error::InvalidSignature)?;
    }
    Ok(VerifiedApprovalSet {
        configuration_digest: configuration_digest(c),
        action_digest: digest,
        threshold: c.threshold,
        member_count: n as u32,
        signer_indices: signers.into_iter().collect(),
    })
}

/// Public deterministic fixtures only. Never use these keys to hold assets.
#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures {
    use super::*;
    use k256::schnorr::SigningKey;

    pub fn make(threshold: u32, members: usize) -> Input {
        assert!(members > 0 && members <= MAX_MEMBERS);
        assert!(threshold > 0 && threshold as usize <= members);
        let keys: Vec<SigningKey> = (1..=members)
            .map(|i| {
                let mut secret = [0u8; 32];
                secret[31] = i as u8;
                SigningKey::from_bytes(&secret).unwrap()
            })
            .collect();
        let configuration = Configuration {
            epoch: 7,
            threshold,
            member_keys: keys
                .iter()
                .map(|k| k.verifying_key().to_bytes().into())
                .collect(),
        };
        let action = Action {
            network_id: [1; 32],
            instance_id: [2; 32],
            program_id: [3; 32],
            configuration_epoch: 7,
            proposal_id: [4; 32],
            target_program: [5; 32],
            expires_at: 2_000_000_000,
            instruction: b"synthetic approval benchmark; no transfer".to_vec(),
        };
        let digest = action_digest(&configuration, &action);
        let approvals = keys
            .iter()
            .take(threshold as usize)
            .enumerate()
            .map(|(i, k)| Approval {
                member_index: i as u32,
                signature: k
                    .sign_prehash_with_aux_rand(&digest, &[42; 32])
                    .unwrap()
                    .to_bytes()
                    .to_vec(),
            })
            .collect();
        Input {
            configuration,
            action,
            approvals,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fixtures::make;

    #[test]
    fn five_distinct_approvers_pass() {
        let report = verify(&make(5, 7)).unwrap();
        assert_eq!(report.signer_indices, vec![0, 1, 2, 3, 4]);
        assert_eq!(report.member_count, 7);
    }

    #[test]
    fn four_approvers_do_not_meet_five() {
        let mut input = make(5, 7);
        input.approvals.pop();
        assert_eq!(verify(&input), Err(Error::InsufficientApprovals));
    }

    #[test]
    fn duplicate_approval_is_not_a_fifth_member() {
        let mut input = make(5, 7);
        input.approvals[4] = input.approvals[0].clone();
        assert_eq!(verify(&input), Err(Error::DuplicateApproval));
    }

    #[test]
    fn distinct_signature_with_wrong_member_is_rejected() {
        let mut input = make(5, 7);
        input.approvals[4].member_index = 5;
        assert_eq!(verify(&input), Err(Error::InvalidSignature));
    }

    #[test]
    fn configuration_epoch_is_current() {
        let mut input = make(5, 7);
        input.configuration.epoch += 1;
        assert_eq!(verify(&input), Err(Error::StaleConfiguration));
    }

    #[test]
    fn action_fields_are_bound() {
        let original = make(5, 7);
        for field in 0..8 {
            let mut input = original.clone();
            match field {
                0 => input.action.network_id[0] ^= 1,
                1 => input.action.instance_id[0] ^= 1,
                2 => input.action.program_id[0] ^= 1,
                3 => input.action.proposal_id[0] ^= 1,
                4 => input.action.target_program[0] ^= 1,
                5 => input.action.expires_at += 1,
                6 => input.action.instruction.push(1),
                _ => input.configuration.threshold = 4,
            }
            assert_eq!(
                verify(&input),
                Err(Error::InvalidSignature),
                "field {field}"
            );
        }
    }

    #[test]
    fn member_and_signature_shape_are_checked() {
        let mut input = make(5, 7);
        input.configuration.member_keys[6] = input.configuration.member_keys[0];
        assert_eq!(verify(&input), Err(Error::DuplicateMember));
        let mut input = make(5, 7);
        input.approvals[0].signature.pop();
        assert_eq!(verify(&input), Err(Error::InvalidSignature));
        let mut input = make(5, 7);
        input.approvals[0].member_index = 7;
        assert_eq!(verify(&input), Err(Error::InvalidMemberIndex));
    }

    #[test]
    fn approval_order_does_not_change_result() {
        let mut input = make(5, 7);
        let expected = verify(&input);
        input.approvals.reverse();
        assert_eq!(verify(&input), expected);
    }

    #[test]
    fn configuration_and_action_bounds() {
        let mut input = make(5, 7);
        input.configuration.threshold = 0;
        assert_eq!(verify(&input), Err(Error::InvalidConfiguration));
        let mut input = make(5, 7);
        input.action.instruction.resize(MAX_ACTION_BYTES + 1, 0);
        assert_eq!(verify(&input), Err(Error::ActionTooLarge));
    }

    #[test]
    fn bitcoin_bip340_published_32_byte_vectors() {
        let csv = include_str!("../../testdata/bip340-test-vectors.csv");
        let mut checked = 0;
        for row in csv.lines().skip(1).filter(|l| !l.trim().is_empty()) {
            let cols: Vec<_> = row.splitn(8, ',').collect();
            let message = hex::decode(cols[4]).unwrap();
            // LEZ's inspected API accepts prehashed 32-byte messages.
            if message.len() != 32 {
                continue;
            }
            let public_key = hex::decode(cols[2]).unwrap();
            let signature = hex::decode(cols[5]).unwrap();
            let actual = VerifyingKey::from_bytes(&public_key)
                .ok()
                .zip(Signature::try_from(signature.as_slice()).ok())
                .is_some_and(|(pk, sig)| pk.verify_prehash(&message, &sig).is_ok());
            assert_eq!(actual, cols[6] == "TRUE", "published vector {}", cols[0]);
            checked += 1;
        }
        assert_eq!(checked, 15);
    }
}
