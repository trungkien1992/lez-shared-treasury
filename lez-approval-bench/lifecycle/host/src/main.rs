mod bootstrap;
mod fixture;
#[path = "../../../integration/host/src/measured.rs"]
#[allow(dead_code)]
mod measured;
mod proof;
mod validation;

use anyhow::{Result, ensure};
use lee::program::Program;
use risc0_zkvm::ExecutorEnv;
use serde_json::json;
use std::{borrow::Cow, fs, path::PathBuf, time::Instant};

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    ensure!(
        args.len() == 3,
        "usage: lifecycle-host execute|validate|prove|succinct|check|check-succinct|bootstrap-config|bootstrap|check-bootstrap REPORT_DIR"
    );
    ensure!(
        std::env::var("RISC0_DEV_MODE").unwrap_or_else(|_| "0".into()) == "0",
        "dev mode forbidden"
    );
    let mode = args[1].as_str();
    ensure!(
        [
            "execute",
            "validate",
            "prove",
            "succinct",
            "check",
            "check-succinct",
            "check-node-witness",
            "bootstrap-config",
            "bootstrap-execute",
            "bootstrap",
            "check-bootstrap"
        ]
        .contains(&mode),
        "unknown mode"
    );
    let dir = PathBuf::from(&args[2]);
    fs::create_dir_all(&dir)?;
    let mut report_path = dir.join(format!("{mode}.json"));
    if mode.starts_with("check") && report_path.exists() {
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos();
        report_path = dir.join(format!("{mode}-{stamp}.json"));
    }
    ensure!(
        !report_path.exists(),
        "report exists; choose a fresh report directory"
    );
    let program = Program::new(Cow::Borrowed(lifecycle_methods::LIFECYCLE_GUEST_ELF))?;
    let f = fixture::Fixture::make(&program)?;
    ensure!(mode != "validate" || !f.imported_approvals,
        "the historical negative-case suite uses only its original fixture; use execute or proving modes for client exports");
    let started = Instant::now();
    let mut report = if mode.starts_with("bootstrap") || mode == "check-bootstrap" {
        bootstrap::run(mode, &program, &f, &dir)?
    } else if mode == "check-node-witness" {
        proof::check_node_witness(&f, &dir)?
    } else if mode == "validate" {
        validation::run(&program, &f, &dir)?
    } else if mode != "execute" {
        proof::run(mode, &program, &f, &dir)?
    } else {
        let mut create_runner = measured::Runner {
            prove: false,
            dir: dir.join("create"),
            stages: vec![],
            assumptions: ExecutorEnv::builder(),
        };
        let (created, receipt) = measured::transaction_output(
            f.create_root()?,
            f.witnesses.clone(),
            &program,
            &mut create_runner,
        )?;
        ensure!(
            receipt.is_none(),
            "execution mode must not return a receipt"
        );
        let vault_pre = f.check_created(&created)?;
        // Carry the executed candidate commitments forward for the second execution.
        // This is an explicit simulation, NOT proof-verified settlement or a Fake receipt.
        let simulated = f.state.clone().with_private_accounts(
            created
                .private_actions
                .iter()
                .map(|a| (a.commitment, a.nullifier)),
        );
        let (root, witnesses) = f.spend_inputs(&simulated, vault_pre)?;
        let mut spend_runner = measured::Runner {
            prove: false,
            dir: dir.join("spend"),
            stages: vec![],
            assumptions: ExecutorEnv::builder(),
        };
        let (spent, receipt) =
            measured::transaction_output(root, witnesses, &program, &mut spend_runner)?;
        ensure!(
            receipt.is_none(),
            "execution mode must not return a receipt"
        );
        f.check_spent(&spent)?;
        fs::write(dir.join("creation-output.borsh"), borsh::to_vec(&created)?)?;
        fs::write(dir.join("spend-output.borsh"), borsh::to_vec(&spent)?)?;
        fs::write(
            dir.join("enrolled-descriptor.borsh"),
            borsh::to_vec(&f.create.descriptor)?,
        )?;
        fs::write(
            dir.join("create-instruction.borsh"),
            borsh::to_vec(&lifecycle_core::Instruction::Create(f.create.clone()))?,
        )?;
        fs::write(dir.join("transfer-instruction.borsh"),
            borsh::to_vec(&lifecycle_core::Instruction::Transfer(f.transfer()))?)?;
        json!({"mode":"execution-only", "unanimous_enrollment":f.create.descriptor.member_keys.len(),
            "spending_threshold":f.create.descriptor.threshold,
            "genesis":"only the independent private funding account; vault is absent",
            "creation":{"funder_before":1000,"funder_after":500,"vault_before":0,"vault_after":500,
                "initial_proposal":0,"stages":create_runner.stages},
            "spend":{"vault_before":500,"vault_after":463,"recipient_before":0,"recipient_after":37,
                "next_proposal":1,"stages":spend_runner.stages},
            "all_four_notes_decrypted_and_checked":true,
            "private_notes_per_transaction":2,"ciphertext_padding":1024,"dummy_notes":0,
            "proof_generated":false,"settlement_verified":false,
            "state_carry":"explicit simulated commitment insertion from checked creation output",
            "vault_id":f.vault.to_string(), "funder_id":f.funder.to_string(),
            "enrollment_digest":hex::encode(lifecycle_core::enrollment_digest(&f.create, f.program_id)),
            "descriptor_digest":hex::encode(lifecycle_core::descriptor_digest(&f.create.descriptor)),
            "member_keys":f.create.descriptor.member_keys.iter().map(hex::encode).collect::<Vec<_>>()})
    };
    report["elapsed_seconds"] = json!(started.elapsed().as_secs_f64());
    report["state_profile"] = json!(f.state_profile);
    if f.state_profile == "standalone" {
        let file = std::env::var("LIFECYCLE_GENESIS_STATE_FILE")
            .unwrap_or_else(|_| "reports/standalone-genesis.state.borsh".into());
        report["genesis_snapshot_sha256"] = json!(measured::sha(&fs::read(file)?));
    }
    report["guest_image_id"] = json!(program.id());
    report["guest_sha256"] = json!(measured::sha(program.elf()));
    report["upstream_commit"] = json!("411adc8fdb3f4c3af64354c4fc26d3e4a6a3d3f4");
    report["ppe_elf_sha256"] = json!(measured::sha(lee::PRIVACY_PRESERVING_CIRCUIT_ELF));
    let fixture_members: Vec<[u8; 32]> = fixture::keys().iter()
        .map(|k| k.verifying_key().to_bytes().into()).collect();
    report["deterministic_public_test_keys"] = json!(f.create.descriptor.member_keys == fixture_members);
    report["member_signing_source"] = json!(if f.imported_approvals {"client-exported responses"} else {"fixed public fixtures"});
    report["member_keys_equal_public_fixture"] = json!(f.create.descriptor.member_keys == fixture_members);
    report["public_test_private_account_material"] = json!(true);
    report["creation_instruction_sha256"] = json!(measured::sha(&borsh::to_vec(
        &lifecycle_core::Instruction::Create(f.create.clone()))?));
    report["transfer_instruction_sha256"] = json!(measured::sha(&borsh::to_vec(
        &lifecycle_core::Instruction::Transfer(f.transfer()))?));
    report["live_devnet_submission"] = json!(false);
    fs::write(dir.join("lifecycle-guest.bin"), program.elf())?;
    measured::write_json(&report_path, &report)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
