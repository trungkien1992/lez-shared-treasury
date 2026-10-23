//! Export the exact private/public pre-genesis state from the pinned upstream library.
fn main() {
    let path = std::env::args().nth(1).expect("usage: export-genesis OUTPUT.borsh");
    let state = testnet_initial_state::initial_state(false);
    std::fs::write(path, borsh::to_vec(&state).expect("serialize genesis")).expect("save genesis");
    println!("{:?}", state.commitment_root());
}
