use lee_core::program::{GuestOutput, ProgramCall, read_program_call};

fn main() {
    let output = match read_program_call::<lifecycle_core::Instruction>() {
        ProgramCall::Plan(input, request) => GuestOutput::Plan(
            lifecycle_core::plan(input, request).expect("lifecycle plan rejected"),
        ),
        ProgramCall::Apply(input) => {
            GuestOutput::Apply(lifecycle_core::apply(input).expect("lifecycle apply rejected"))
        }
    };
    output.write();
}
