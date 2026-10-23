use anyhow::{Context as _, Result, bail, ensure};
use client::state_import::{self, TrustedCheckpoint, VerifiedState};
use k256::schnorr::SigningKey;
use lifecycle_core::{Descriptor, Funding};
use rand_core::{OsRng, RngCore};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::json;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};
use treasury_client::{self as client, Bundle, Context, Request, Response};
use zeroize::Zeroizing;

const HELP: &str = "treasury-client commands:
  keygen KEY_FILE PUBLIC_JSON
  context DESCRIPTOR_BORSH CONTEXT_JSON
  configure TEMPLATE_DESCRIPTOR_BORSH MEMBER_PUBLIC_KEYS_JSON THRESHOLD CONTEXT_JSON
  create CONTEXT_JSON EXPIRES_MS FUNDER_HEX_OR_NONE AMOUNT_OR_ZERO REQUEST_JSON
  propose CONTEXT_JSON RECIPIENT_HEX AMOUNT_BASE_UNITS EXPIRES_MS REQUEST_JSON
  review CONTEXT_JSON REQUEST_JSON
  respond-offline CONTEXT_JSON REQUEST_JSON KEY_FILE approve|reject REVIEW_DIGEST RESPONSE_JSON
  import-state EVIDENCE_BORSH TRUSTED_CHECKPOINT_JSON NULLIFIER_KEY_FILE CONTEXT_JSON
  review-verified EVIDENCE_BORSH TRUSTED_CHECKPOINT_JSON NULLIFIER_KEY_FILE REQUEST_JSON
  respond-verified EVIDENCE_BORSH TRUSTED_CHECKPOINT_JSON NULLIFIER_KEY_FILE REQUEST_JSON KEY_FILE approve|reject REVIEW_DIGEST RESPONSE_JSON
  collect CONTEXT_JSON REQUEST_JSON BUNDLE_JSON RESPONSE_JSON...
  status CONTEXT_JSON REQUEST_JSON BUNDLE_JSON
  export CONTEXT_JSON REQUEST_JSON BUNDLE_JSON NEW_DIRECTORY

Offline test client. Amounts are integer base units. Read client/README.md.
Verified commands require an independently authenticated checkpoint and local nullifier key.
respond-offline is for deliberately unverified local workflows. Exported instructions are not transactions.";

fn now_ms() -> Result<u64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_millis()
        .try_into()?)
}
fn read(path: impl AsRef<Path>) -> Result<Vec<u8>> {
    read_limited(path, client::MAX_FILE_BYTES)
}
fn read_limited(path: impl AsRef<Path>, limit: usize) -> Result<Vec<u8>> {
    let path = path.as_ref();
    let mut file =
        fs::File::open(path).with_context(|| format!("cannot read {}", path.display()))?;
    let mut data = vec![];
    std::io::Read::read_to_end(
        &mut std::io::Read::take(&mut file, limit as u64 + 1),
        &mut data,
    )?;
    ensure!(data.len() <= limit, "input file too large");
    Ok(data)
}
fn load<T: DeserializeOwned>(path: &str) -> Result<T> {
    Ok(serde_json::from_slice(&read(path)?)?)
}
fn save_bytes(path: impl AsRef<Path>, data: &[u8], secret: bool) -> Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(if secret { 0o600 } else { 0o644 });
    }
    #[cfg(not(unix))]
    {
        ensure!(
            !secret,
            "private-key files currently require Unix permission support"
        );
    }
    let path = path.as_ref();
    let mut f = options.open(path).with_context(|| {
        format!(
            "cannot create {}; existing files are preserved",
            path.display()
        )
    })?;
    f.write_all(data)?;
    f.sync_all()?;
    Ok(())
}
fn save<T: Serialize>(path: impl AsRef<Path>, data: &T) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(data)?;
    bytes.push(b'\n');
    save_bytes(path, &bytes, true)
}
fn print<T: Serialize>(data: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(data)?);
    Ok(())
}
fn private_secret(path: &str) -> Result<Zeroizing<[u8; 32]>> {
    let metadata = fs::symlink_metadata(path)?;
    ensure!(
        metadata.file_type().is_file(),
        "key must be a regular file, not a symlink"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        ensure!(
            metadata.permissions().mode() & 0o077 == 0,
            "private key permissions must be 0600 or stricter"
        );
    }
    #[cfg(not(unix))]
    bail!("private-key files currently require Unix permission support");
    let bytes = Zeroizing::new(read(path)?);
    let text = std::str::from_utf8(&bytes)?;
    let secret = Zeroizing::new(client::unhex::<32>(text.trim())?);
    Ok(secret)
}
fn key(path: &str) -> Result<SigningKey> {
    Ok(SigningKey::from_bytes(private_secret(path)?.as_ref())?)
}
fn verified(evidence: &str, checkpoint: &str, nullifier_key: &str) -> Result<VerifiedState> {
    let checkpoint: TrustedCheckpoint = load(checkpoint)?;
    let encoded = read_limited(evidence, state_import::MAX_EVIDENCE_BYTES)?;
    let secret = private_secret(nullifier_key)?;
    state_import::verify(&encoded, &checkpoint, &secret, now_ms()?)
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let a: Vec<_> = args.iter().map(String::as_str).collect();
    match a.as_slice() {
        [] | ["help"] | ["--help"] => println!("{HELP}"),
        ["keygen", secret_path, public_path] => {
            ensure!(
                !Path::new(secret_path).exists() && !Path::new(public_path).exists(),
                "choose fresh key and public-key paths"
            );
            ensure!(
                secret_path != public_path,
                "private and public files must be different"
            );
            let key = SigningKey::random(&mut OsRng);
            let secret = Zeroizing::new(format!("{}\n", hex::encode(key.to_bytes())));
            save_bytes(secret_path, secret.as_bytes(), true)?;
            let public = json!({"scheme":"bip340","public_key_hex":hex::encode(key.verifying_key().to_bytes())});
            save(public_path, &public)?;
            print(&public)?;
        }
        ["context", descriptor_path, out] => {
            let descriptor: Descriptor = borsh::from_slice(&read(descriptor_path)?)?;
            let context = Context::from_descriptor(&descriptor)?;
            save(out, &context)?;
            print(
                &json!({"context_id":hex::encode(context.id()?),"state_source":"expected local initial configuration; no chain authentication"}),
            )?;
        }
        ["configure", template_path, members_path, threshold, out] => {
            let mut descriptor: Descriptor = borsh::from_slice(&read(template_path)?)?;
            let members: Vec<String> = load(members_path)?;
            descriptor.member_keys = members
                .iter()
                .map(|s| client::unhex::<32>(s))
                .collect::<Result<_>>()?;
            descriptor.threshold = threshold.parse()?;
            OsRng
                .try_fill_bytes(&mut descriptor.salt)
                .context("operating-system randomness unavailable")?;
            let context = Context::from_descriptor(&descriptor)?;
            save(out, &context)?;
            print(
                &json!({"context_id":hex::encode(context.id()?),"fresh_salt":true,"private_account_material":"retained from the explicitly supplied descriptor template"}),
            )?;
        }
        ["create", context_path, expiry, funder, amount, out] => {
            let context: Context = load(context_path)?;
            let funding = if *funder == "none" {
                ensure!(*amount == "0", "unfunded creation requires amount zero");
                None
            } else {
                Some(Funding {
                    from: lee_core::account::AccountId::new(client::unhex::<32>(funder)?),
                    amount: client::decimal_amount(amount)?,
                })
            };
            let request = client::unsigned_create(&context, expiry.parse()?, funding)?;
            client::ensure_unexpired(&client::decode(&context, &request)?, now_ms()?)?;
            save(out, &request)?;
            print(&client::review(&context, &request)?)?;
        }
        ["propose", context_path, recipient, amount, expiry, out] => {
            let context: Context = load(context_path)?;
            let request = client::unsigned_transfer(
                &context,
                lee_core::account::AccountId::new(client::unhex::<32>(recipient)?),
                client::decimal_amount(amount)?,
                expiry.parse()?,
            )?;
            client::ensure_unexpired(&client::decode(&context, &request)?, now_ms()?)?;
            save(out, &request)?;
            print(&client::review(&context, &request)?)?;
        }
        ["review", context_path, request_path] => {
            let context: Context = load(context_path)?;
            let request: Request = load(request_path)?;
            let mut review = client::review(&context, &request)?;
            review["expired_at_local_clock"] =
                json!(now_ms()? >= client::decode(&context, &request)?.action.expires_at);
            print(&review)?;
        }
        [
            "respond-offline",
            context_path,
            request_path,
            key_path,
            decision,
            review_digest,
            out,
        ] => {
            let context: Context = load(context_path)?;
            let request: Request = load(request_path)?;
            let response = client::respond(
                &context,
                &request,
                &key(key_path)?,
                client::parse_decision(decision)?,
                &client::unhex::<32>(review_digest)?,
                now_ms()?,
            )?;
            save(out, &response)?;
            print(
                &json!({"member_index":response.member_index,"decision":response.decision,"action_digest":response.action_digest}),
            )?;
        }
        ["respond", ..] => bail!(
            "choose respond-verified for checkpoint-backed state, or respond-offline for an explicitly unverified local workflow"
        ),
        ["import-state", evidence, checkpoint, nullifier_key, out] => {
            let state = verified(evidence, checkpoint, nullifier_key)?;
            save(out, state.context())?;
            print(&state.report()?)?;
        }
        [
            "review-verified",
            evidence,
            checkpoint,
            nullifier_key,
            request_path,
        ] => {
            let state = verified(evidence, checkpoint, nullifier_key)?;
            let request: Request = load(request_path)?;
            print(&state.review(&request, now_ms()?)?)?;
        }
        [
            "respond-verified",
            evidence,
            checkpoint,
            nullifier_key,
            request_path,
            key_path,
            decision,
            review_digest,
            out,
        ] => {
            let state = verified(evidence, checkpoint, nullifier_key)?;
            let request: Request = load(request_path)?;
            let response = state.respond(
                &request,
                &key(key_path)?,
                client::parse_decision(decision)?,
                &client::unhex::<32>(review_digest)?,
                now_ms()?,
            )?;
            save(out, &response)?;
            print(
                &json!({"member_index":response.member_index,"decision":response.decision,"action_digest":response.action_digest,"verified_state":state.report()?}),
            )?;
        }
        ["collect", context_path, request_path, out, responses @ ..] => {
            ensure!(!responses.is_empty(), "supply at least one response file");
            let context: Context = load(context_path)?;
            let request: Request = load(request_path)?;
            let messages = responses
                .iter()
                .map(|p| load::<Response>(p))
                .collect::<Result<Vec<_>>>()?;
            let bundle = client::collect(&context, &request, messages, now_ms()?)?;
            save(out, &bundle)?;
            print(&client::status(&context, &request, &bundle, now_ms()?)?)?;
        }
        ["status", context_path, request_path, bundle_path] => {
            let context: Context = load(context_path)?;
            let request: Request = load(request_path)?;
            let bundle: Bundle = load(bundle_path)?;
            print(&client::status(&context, &request, &bundle, now_ms()?)?)?;
        }
        ["export", context_path, request_path, bundle_path, directory] => {
            let context: Context = load(context_path)?;
            let request: Request = load(request_path)?;
            let bundle: Bundle = load(bundle_path)?;
            let (instruction, native_output) =
                client::export(&context, &request, &bundle, now_ms()?)?;
            let dir = Path::new(directory);
            let mut builder = fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            builder
                .create(dir)
                .context("export directory must be new and have an existing parent")?;
            save_bytes(dir.join("instruction.borsh"), &instruction, true)?;
            save_bytes(
                dir.join("native-config-apply-output.borsh"),
                &native_output,
                true,
            )?;
            save(dir.join("context.json"), &context)?;
            save(dir.join("request.json"), &request)?;
            save(dir.join("responses.json"), &bundle)?;
            save(
                dir.join("review.json"),
                &client::review(&context, &request)?,
            )?;
            save(
                dir.join("predicted-context.json"),
                &client::advance_expected_context(&context, &instruction)?,
            )?;
            let mut report = client::status(&context, &request, &bundle, now_ms()?)?;
            report["instruction_sha256"] = json!(hex::encode(client::hash(&instruction)));
            report["native_config_apply_passed"] = json!(true);
            report["proof_generated"] = json!(false);
            report["sequencer_submission"] = json!(false);
            report["predicted_context_authenticated"] = json!(false);
            save(dir.join("export.json"), &report)?;
            print(&report)?;
        }
        _ => bail!("invalid arguments\n{HELP}"),
    }
    Ok(())
}
