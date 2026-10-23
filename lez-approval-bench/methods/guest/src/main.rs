#![no_main]
use approval_core::{verify, Input};
use risc0_zkvm::guest::env;
risc0_zkvm::guest::entry!(main);

pub fn main() {
    let input: Input = env::read();
    let verified = verify(&input).expect("approval policy rejected");
    env::commit(&verified);
}
