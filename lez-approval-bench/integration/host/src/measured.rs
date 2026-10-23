//! Instrumented equivalent of pinned LEE's private transaction prover.
//! Uses its unmodified ExecutionState, native token implementation and PPE ELF.
//! Differences: bounded segments, stage evidence/checkpoints and execute-only mode.
use anyhow::{Context, Result, anyhow, bail, ensure};
use lee::{PRIVACY_PRESERVING_CIRCUIT_ELF, PRIVACY_PRESERVING_CIRCUIT_ID, program::Program};
use lee_core::{
    PrivacyPreservingCircuitInput, PrivacyPreservingCircuitOutput, PrivateWitness, ProvenCall,
    ShadowProgramWitness,
    execution_state::{Backend, DeferPublicEffects, ExecutionState, RootCall},
    from_frame,
    native_token::{self, NATIVE_TOKEN_PROGRAM_ID},
    program::{ApplyInput, ApplyOutput, GuestOutput, PlanInput, PlanOutput, ProgramEvent},
    to_borsh_frame,
};
use risc0_zkvm::{
    ExecutorEnv, ExecutorEnvBuilder, ExitCode, InnerReceipt, ProverOpts, Receipt, default_executor,
    default_prover,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    borrow::Cow,
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

pub const SEGMENT_PO2: u32 = 18;
pub const SESSION_LIMIT: u64 = 128 * 1024 * 1024;
pub fn sha(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

pub struct Runner {
    pub prove: bool,
    pub dir: PathBuf,
    pub stages: Vec<Value>,
    pub assumptions: ExecutorEnvBuilder<'static>,
}

impl Runner {
    fn run(
        &mut self,
        label: &str,
        program: &Program,
        encoded: &[u8],
        write: impl FnOnce(&mut ExecutorEnvBuilder) -> Result<()>,
    ) -> Result<Vec<u8>> {
        let mut builder = ExecutorEnv::builder();
        builder
            .segment_limit_po2(SEGMENT_PO2)
            .session_limit(Some(SESSION_LIMIT));
        write(&mut builder)?;
        let started = Instant::now();
        if self.prove {
            let (receipt, metadata) = self.prove_stage(label, program, encoded, builder)?;
            self.stages.push(metadata);
            self.assumptions.add_assumption(receipt.clone());
            Ok(receipt.journal.bytes)
        } else {
            let session = default_executor().execute(builder.build()?, program.elf())?;
            ensure!(
                session.exit_code == ExitCode::Halted(0),
                "{label}: guest failed: {:?}",
                session.exit_code
            );
            self.stages.push(json!({"stage":label, "mode":"execute-only", "seconds":started.elapsed().as_secs_f64(),
                "user_cycles":session.cycles(), "segments":session.segments.len(),
                "image_id":program.id(), "input_sha256":sha(encoded), "journal_bytes":session.journal.bytes.len()}));
            // Unresolved claims are used ONLY to measure execution, never to
            // generate a proof or validate a transaction against LEE state.
            self.assumptions
                .add_assumption(session.receipt_claim.context("missing executed claim")?);
            Ok(session.journal.bytes)
        }
    }

    fn prove_stage(
        &self,
        label: &str,
        program: &Program,
        encoded: &[u8],
        mut builder: ExecutorEnvBuilder,
    ) -> Result<(Receipt, Value)> {
        let receipt_path = self.dir.join(format!("{label}.receipt"));
        let metadata_path = self.dir.join(format!("{label}.json"));
        if receipt_path.exists() && metadata_path.exists() {
            let receipt: Receipt = bincode::deserialize(&fs::read(&receipt_path)?)?;
            let metadata: Value = serde_json::from_slice(&fs::read(&metadata_path)?)?;
            ensure!(
                metadata["input_sha256"] == sha(encoded),
                "checkpoint input changed"
            );
            ensure!(
                metadata["receipt_sha256"] == sha(&fs::read(&receipt_path)?),
                "checkpoint hash changed"
            );
            reject_fake(&receipt)?;
            receipt.verify(program.id())?;
            eprintln!("Reusing independently verified stage {label}");
            return Ok((receipt, metadata));
        }
        ensure!(
            !receipt_path.exists() && !metadata_path.exists(),
            "incomplete stage checkpoint: {label}"
        );
        eprintln!("Proving {label} (segment 2^{SEGMENT_PO2})");
        let started = Instant::now();
        let result = default_prover().prove_with_opts(
            builder.build()?,
            program.elf(),
            &ProverOpts::composite().with_dev_mode(false),
        )?;
        reject_fake(&result.receipt)?;
        result.receipt.verify(program.id())?;
        let bytes = bincode::serialize(&result.receipt)?;
        let metadata = json!({"stage":label,"mode":"real-proof", "seconds":started.elapsed().as_secs_f64(),
            "stats":result.stats,"image_id":program.id(),"input_sha256":sha(encoded),
            "receipt_sha256":sha(&bytes),"receipt_bytes":bytes.len(),"journal_bytes":result.receipt.journal.bytes.len()});
        fs::write(&receipt_path, bytes)?;
        write_json(&metadata_path, &metadata)?;
        eprintln!("Completed {label}: {metadata}");
        Ok((result.receipt, metadata))
    }
}

pub fn reject_fake(receipt: &Receipt) -> Result<()> {
    ensure!(
        !matches!(receipt.inner, InnerReceipt::Fake(_)),
        "fake receipts forbidden"
    );
    Ok(())
}
pub fn write_json(path: &Path, value: &Value) -> Result<()> {
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

struct MeasuredBackend<'a> {
    program: &'a Program,
    runner: &'a mut Runner,
    calls: Vec<ProvenCall>,
}
impl Backend for MeasuredBackend<'_> {
    type PublicEffects = DeferPublicEffects;
    type Call = Option<ProvenCall>;
    type Error = anyhow::Error;
    fn plan(
        &mut self,
        input: &PlanInput,
        _: &ExecutionState<'_>,
    ) -> Result<(PlanOutput, Self::Call)> {
        if input.self_account_id == NATIVE_TOKEN_PROGRAM_ID {
            return Ok((
                native_token::plan(
                    input.caller_account_id,
                    &input.accounts,
                    &input.instruction_data,
                )?,
                None,
            ));
        }
        ensure!(
            input.self_account_id == lee::AccountId::for_shadow_program(&self.program.id()),
            "undeclared program"
        );
        let bytes = self
            .runner
            .run("vault-plan", self.program, &borsh::to_vec(input)?, |env| {
                Program::write_plan_inputs(input, env)?;
                Ok(())
            })?;
        let GuestOutput::Plan(plan) = decode(&bytes)? else {
            bail!("expected plan journal")
        };
        Ok((
            plan.clone(),
            Some(ProvenCall {
                plan,
                private_apply_outputs: vec![],
            }),
        ))
    }
    fn apply(&mut self, call: &mut Self::Call, input: &ApplyInput) -> Result<ApplyOutput> {
        let Some(call) = call else {
            return Ok(native_token::apply_output(input)?);
        };
        let bytes =
            self.runner
                .run("vault-apply", self.program, &borsh::to_vec(input)?, |env| {
                    Program::write_apply_inputs(input, env)?;
                    Ok(())
                })?;
        let GuestOutput::Apply(output) = decode(&bytes)? else {
            bail!("expected apply journal")
        };
        call.private_apply_outputs.push(output.clone());
        Ok(output)
    }
    fn complete(
        &mut self,
        call: Self::Call,
        _: Vec<ProgramEvent>,
        _: &ExecutionState<'_>,
    ) -> Result<()> {
        self.calls.extend(call);
        Ok(())
    }
}
fn decode(journal: &[u8]) -> Result<GuestOutput> {
    Ok(borsh::from_slice(
        from_frame(journal).ok_or_else(|| anyhow!("invalid journal frame"))?,
    )?)
}

pub fn transaction_output(
    root: RootCall,
    witnesses: Vec<PrivateWitness>,
    program: &Program,
    runner: &mut Runner,
) -> Result<(PrivacyPreservingCircuitOutput, Option<Receipt>)> {
    let mut backend = MeasuredBackend {
        program,
        runner,
        calls: vec![],
    };
    ExecutionState::initialize(root.clone(), &witnesses)?.run(&mut backend)?;
    let calls = backend.calls;
    let input = PrivacyPreservingCircuitInput {
        root,
        private_witnesses: witnesses,
        calls,
        dummy_inputs: vec![],
        ciphertext_padding: Some(1024),
        program_image_witnesses: vec![],
        shadow_program_witnesses: vec![ShadowProgramWitness {
            image_id: program.id(),
        }],
    };
    let encoded = to_borsh_frame(&input);
    let mut builder = std::mem::replace(&mut runner.assumptions, ExecutorEnv::builder());
    builder
        .segment_limit_po2(SEGMENT_PO2)
        .session_limit(Some(SESSION_LIMIT))
        .write_slice(&encoded);
    let (journal, receipt) = if runner.prove {
        let ppe = Program::new_unchecked(
            PRIVACY_PRESERVING_CIRCUIT_ID,
            Cow::Borrowed(PRIVACY_PRESERVING_CIRCUIT_ELF),
        );
        let (receipt, metadata) = runner.prove_stage("ppe-composite", &ppe, &encoded, builder)?;
        runner.stages.push(metadata);
        (receipt.journal.bytes.clone(), Some(receipt))
    } else {
        let started = Instant::now();
        let session =
            default_executor().execute(builder.build()?, PRIVACY_PRESERVING_CIRCUIT_ELF)?;
        ensure!(
            session.exit_code == ExitCode::Halted(0),
            "PPE execution failed"
        );
        runner.stages.push(
            json!({"stage":"ppe","mode":"execute-only","seconds":started.elapsed().as_secs_f64(),
            "user_cycles":session.cycles(),"segments":session.segments.len(),
            "input_sha256":sha(&encoded),"journal_bytes":session.journal.bytes.len()}),
        );
        (session.journal.bytes, None)
    };
    let output = borsh::from_slice(from_frame(&journal).context("bad PPE output frame")?)?;
    Ok((output, receipt))
}

pub fn compress(receipt: &Receipt, dir: &Path) -> Result<(Receipt, Value)> {
    reject_fake(receipt)?;
    let path = dir.join("ppe-succinct.receipt");
    if path.exists() {
        let saved: Receipt = bincode::deserialize(&fs::read(path)?)?;
        reject_fake(&saved)?;
        saved.verify(PRIVACY_PRESERVING_CIRCUIT_ID)?;
        ensure!(
            saved.journal == receipt.journal,
            "compressed receipt journal changed"
        );
        return Ok((
            saved,
            serde_json::from_slice(&fs::read(dir.join("ppe-succinct.json"))?)?,
        ));
    }
    eprintln!("Compressing complete PPE receipt to upstream's succinct format");
    let started = Instant::now();
    let saved = default_prover().compress(&ProverOpts::succinct().with_dev_mode(false), receipt)?;
    reject_fake(&saved)?;
    saved.verify(PRIVACY_PRESERVING_CIRCUIT_ID)?;
    let bytes = bincode::serialize(&saved)?;
    let metadata = json!({"stage":"ppe-succinct","mode":"real-proof","seconds":started.elapsed().as_secs_f64(),
        "receipt_bytes":bytes.len(),"receipt_sha256":sha(&bytes),"journal_bytes":saved.journal.bytes.len()});
    fs::write(path, bytes)?;
    write_json(&dir.join("ppe-succinct.json"), &metadata)?;
    Ok((saved, metadata))
}
