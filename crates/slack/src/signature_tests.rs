use super::{HmacSha256, SignatureError, verify_signature_at};
use hmac::Mac;

#[test]
fn valid_signatures_are_accepted() {
    let payload = br#"{"type":"url_verification","challenge":"abc"}"#;
    let signature = compute_signature("secret", 1_700_000_000, payload);

    assert_eq!(
        verify_signature_at("secret", "1700000000", payload, &signature, 1_700_000_060),
        Ok(())
    );
}

#[test]
fn expired_signatures_are_rejected() {
    let payload = br#"{}"#;
    let signature = compute_signature("secret", 1_700_000_000, payload);

    assert_eq!(
        verify_signature_at("secret", "1700000000", payload, &signature, 1_700_000_400),
        Err(SignatureError::Expired)
    );
}

fn compute_signature(signing_secret: &str, timestamp: i64, raw_body: &[u8]) -> String {
    let mut mac =
        HmacSha256::new_from_slice(signing_secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(format!("v0:{timestamp}:").as_bytes());
    mac.update(raw_body);
    format!("v0={}", hex::encode(mac.finalize().into_bytes()))
}
