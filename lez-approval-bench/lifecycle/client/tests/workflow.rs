use k256::schnorr::SigningKey;
use lifecycle_core::{Descriptor, Instruction};
use treasury_client::*;

fn fixture() -> (Context, Vec<SigningKey>) {
    let d: Descriptor = borsh::from_slice(include_bytes!(
        "../../reports/lifecycle/enrolled-descriptor.borsh"
    ))
    .unwrap();
    let keys = (1..=7)
        .map(|i| {
            let mut s = [0; 32];
            s[31] = i;
            SigningKey::from_bytes(&s).unwrap()
        })
        .collect();
    (Context::from_descriptor(&d).unwrap(), keys)
}
fn spend(c: &Context) -> Request {
    unsigned_transfer(
        c,
        lee_core::account::AccountId::new([18; 32]),
        37,
        2_000_000_000_000,
    )
    .unwrap()
}
fn approvals(c: &Context, r: &Request, keys: &[SigningKey], indices: &[usize]) -> Bundle {
    let digest = decode(c, r).unwrap().review_digest;
    collect(
        c,
        r,
        indices
            .iter()
            .map(|i| respond(c, r, &keys[*i], Decision::Approve, &digest, 1).unwrap()),
        1,
    )
    .unwrap()
}

#[test]
fn distinct_nonconsecutive_signers_export_the_guest_instruction() {
    let (c, keys) = fixture();
    let r = spend(&c);
    let b = approvals(&c, &r, &keys, &[0, 2, 3, 5, 6]);
    let (bytes, _) = export(&c, &r, &b, 1).unwrap();
    let Instruction::Transfer(actual) = borsh::from_slice(&bytes).unwrap() else {
        panic!()
    };
    assert_eq!(
        actual
            .approvals
            .iter()
            .map(|a| a.member_index)
            .collect::<Vec<_>>(),
        vec![0, 2, 3, 5, 6]
    );
    assert_eq!(actual.transfer.amount, 37);
}
#[test]
fn creation_requires_every_member_even_with_threshold_five() {
    let (c, keys) = fixture();
    let r = unsigned_create(&c, 2_000_000_000_000, None).unwrap();
    assert!(export(&c, &r, &approvals(&c, &r, &keys, &[0, 1, 2, 3, 4, 5]), 1).is_err());
    assert!(export(&c, &r, &approvals(&c, &r, &keys, &[0, 1, 2, 3, 4, 5, 6]), 1).is_ok());
}
#[test]
fn replayed_and_reordered_deliveries_never_inflate_quorum() {
    let (c, keys) = fixture();
    let r = spend(&c);
    let b = approvals(&c, &r, &keys, &[0, 1, 2, 3]);
    let mut deliveries = b.responses.clone();
    deliveries.extend(b.responses.clone());
    deliveries.reverse();
    let canonical = collect(&c, &r, deliveries, 1).unwrap();
    assert_eq!(canonical.responses.len(), 4);
    assert!(export(&c, &r, &canonical, 1).is_err());
}
#[test]
fn signing_requires_the_exact_reviewed_bytes() {
    let (c, keys) = fixture();
    let mut r = spend(&c);
    let review = decode(&c, &r).unwrap().review_digest;
    let Instruction::Transfer(mut t) = decode(&c, &r).unwrap().instruction else {
        panic!()
    };
    t.transfer.amount += 1;
    r.instruction_hex = hex::encode(borsh::to_vec(&Instruction::Transfer(t)).unwrap());
    assert!(respond(&c, &r, &keys[0], Decision::Approve, &review, 1).is_err());
}
#[test]
fn a_rejection_cannot_be_presented_as_an_approval() {
    let (c, keys) = fixture();
    let r = spend(&c);
    let digest = decode(&c, &r).unwrap().review_digest;
    let mut rejection = respond(&c, &r, &keys[0], Decision::Reject, &digest, 1).unwrap();
    rejection.decision = Decision::Approve;
    assert!(collect(&c, &r, [rejection], 1).is_err());
}
#[test]
fn a_rejection_is_not_a_veto_but_conflicting_decisions_are_detected() {
    let (c, keys) = fixture();
    let r = spend(&c);
    let digest = decode(&c, &r).unwrap().review_digest;
    let mut responses = approvals(&c, &r, &keys, &[0, 1, 2, 3, 4]).responses;
    responses.push(respond(&c, &r, &keys[5], Decision::Reject, &digest, 1).unwrap());
    let b = collect(&c, &r, responses.clone(), 1).unwrap();
    assert!(export(&c, &r, &b, 1).is_ok());
    responses.push(respond(&c, &r, &keys[0], Decision::Reject, &digest, 1).unwrap());
    assert!(collect(&c, &r, responses, 1).is_err());
}
#[test]
fn expired_requests_fail_at_the_exclusive_boundary() {
    let (c, keys) = fixture();
    let r = spend(&c);
    let b = approvals(&c, &r, &keys, &[0, 1, 2, 3, 4]);
    assert!(export(&c, &r, &b, 2_000_000_000_000).is_err());
    assert!(export(&c, &r, &b, 1_999_999_999_999).is_ok());
}
#[test]
fn next_counter_context_does_not_accept_old_requests() {
    let (c, keys) = fixture();
    let r = spend(&c);
    let b = approvals(&c, &r, &keys, &[0, 1, 2, 3, 4]);
    let (bytes, _) = export(&c, &r, &b, 1).unwrap();
    let next = advance_expected_context(&c, &bytes).unwrap();
    assert!(decode(&next, &r).is_err());
}
#[test]
fn changed_signed_fields_and_response_indices_fail_verification() {
    let (c, keys) = fixture();
    let r = spend(&c);
    let b = approvals(&c, &r, &keys, &[0, 1, 2, 3, 4]);
    for field in 0..4 {
        let mut altered = r.clone();
        let Instruction::Transfer(mut t) = decode(&c, &r).unwrap().instruction else {
            panic!()
        };
        match field {
            0 => t.transfer.amount += 1,
            1 => t.transfer.expires_at_ms += 1,
            2 => t.transfer.recipient = lee_core::account::AccountId::new([19; 32]),
            _ => t.transfer.proposal += 1,
        };
        altered.instruction_hex = hex::encode(borsh::to_vec(&Instruction::Transfer(t)).unwrap());
        assert!(export(&c, &altered, &b, 1).is_err());
    }
    let mut wrong = b.clone();
    wrong.responses[0].member_index = 6;
    assert!(export(&c, &r, &wrong, 1).is_err());
}
#[test]
fn review_preserves_large_amounts_as_decimal_base_units() {
    let (c, _) = fixture();
    let r = unsigned_transfer(
        &c,
        lee_core::account::AccountId::new([18; 32]),
        u128::MAX,
        2_000_000_000_000,
    )
    .unwrap();
    assert_eq!(
        review(&c, &r).unwrap()["action"]["amount_base_units"],
        u128::MAX.to_string()
    );
    for s in [
        "0",
        "01",
        "1.0",
        "1e3",
        "-1",
        "340282366920938463463374607431768211456",
    ] {
        assert!(decimal_amount(s).is_err());
    }
}
#[test]
fn proposer_descriptions_and_unknown_metadata_are_rejected() {
    let (c, _) = fixture();
    let r = spend(&c);
    let mut json = serde_json::to_value(r).unwrap();
    json["description"] = serde_json::json!("send a different amount");
    assert!(serde_json::from_value::<Request>(json).is_err());
}
#[test]
fn unsigned_inbound_funding_scope_is_explicit() {
    let (c, _) = fixture();
    let a = unsigned_create(&c, 2_000_000_000_000, None).unwrap();
    let b = unsigned_create(
        &c,
        2_000_000_000_000,
        Some(lifecycle_core::Funding {
            from: lee_core::account::AccountId::new([18; 32]),
            amount: 500,
        }),
    )
    .unwrap();
    assert_eq!(
        decode(&c, &a).unwrap().digest,
        decode(&c, &b).unwrap().digest
    );
    assert_ne!(
        decode(&c, &a).unwrap().review_digest,
        decode(&c, &b).unwrap().review_digest
    );
    assert_eq!(
        review(&c, &b).unwrap()["action"]["funding"]["approved_by_this_enrollment"],
        false
    );
}

#[test]
fn archived_real_creation_instruction_roundtrips_without_a_byte_change() {
    let original =
        include_bytes!("../../reports/2026-10-12-standalone-execution/create-instruction.borsh");
    let Instruction::Create(mut create) = borsh::from_slice::<Instruction>(original).unwrap()
    else {
        panic!()
    };
    let context = Context::from_descriptor(&create.descriptor).unwrap();
    let approvals = std::mem::take(&mut create.enrollments);
    let r = request(&context, Instruction::Create(create)).unwrap();
    let d = decode(&context, &r).unwrap();
    let responses = approvals.into_iter().map(|a| Response {
        format: RESPONSE_FORMAT.into(),
        context_id: r.context_id.clone(),
        action_digest: hex::encode(d.digest),
        member_index: a.member_index,
        decision: Decision::Approve,
        signature: hex::encode(a.signature),
    });
    let b = collect(&context, &r, responses, 1).unwrap();
    assert_eq!(export(&context, &r, &b, 1).unwrap().0, original);
}
