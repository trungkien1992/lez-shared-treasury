//! Execution-only validation of the accelerated backend. This is a separate
//! program and is never used as the approval proof's guest image.
#![no_main]
use approval_core::{verify, Input};
use k256::schnorr::{signature::hazmat::PrehashVerifier, Signature, VerifyingKey};
use risc0_zkvm::guest::env;
risc0_zkvm::guest::entry!(main);

pub fn main() {
    // Expected outcomes come from the host's unpatched upstream implementation.
    let cases: Vec<(Input, String)> = env::read();
    for (index, (input, expected)) in cases.iter().enumerate() {
        assert_eq!(&format!("{:?}", verify(input)), expected, "case {index}");
    }
    let mut vectors_checked = 0u32;
    let csv = include_str!("../../../../testdata/bip340-test-vectors.csv");
    for row in csv.lines().skip(1).filter(|line| !line.trim().is_empty()) {
        let columns: Vec<_> = row.splitn(8, ',').collect();
        let message = hex::decode(columns[4]).unwrap();
        if message.len() != 32 {
            continue;
        }
        let public_key = hex::decode(columns[2]).unwrap();
        let signature = hex::decode(columns[5]).unwrap();
        let actual = VerifyingKey::from_bytes(&public_key)
            .ok()
            .zip(Signature::try_from(signature.as_slice()).ok())
            .is_some_and(|(key, signature)| key.verify_prehash(&message, &signature).is_ok());
        assert_eq!(
            actual,
            columns[6] == "TRUE",
            "published vector {}",
            columns[0]
        );
        vectors_checked += 1;
    }
    assert_eq!(vectors_checked, 15);
    env::commit(&(cases.len() as u32, vectors_checked));
}
