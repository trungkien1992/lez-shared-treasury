use lee_core::program::{GuestOutput, ProgramCall, read_program_call};
fn main() {
    let output = match read_program_call::<vault_core::Request>() {
        ProgramCall::Plan(input, request) => {
            GuestOutput::Plan(vault_core::plan(input, request).expect("invalid vault plan"))
        }
        ProgramCall::Apply(input) => {
            GuestOutput::Apply(vault_core::apply(input).expect("vault approval rejected"))
        }
    };
    output.write();
}
