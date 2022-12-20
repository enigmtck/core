use crate::activity_pub::Actor;
use crate::models::profiles::Profile;
use rsa::pkcs1v15::{SigningKey, VerifyingKey};
use rsa::signature::{RandomizedSigner, Signature};
use rsa::{pkcs8::DecodePrivateKey, RsaPrivateKey};
use sha2::Sha256;

pub fn sign(profile: Profile, request_target: String, host: String, date: String) -> String {
    // (request-target): get /users/username/outbox
    // host: mastodon.example
    // date: 18 Dec 2019 10:08:46 GMT

    let actor = Actor::from(profile.clone());

    let private_key = RsaPrivateKey::from_pkcs8_pem(&profile.private_key).unwrap();
    let signing_key = SigningKey::<Sha256>::new(private_key);
    let verifying_key: VerifyingKey<_> = (&signing_key).into();

    let structured_data = format!(
        "(request-target): {}\nhost: {}\ndate: {}\n",
        request_target, host, date
    );

    let mut rng = rand::thread_rng();
    let signature = signing_key.sign_with_rng(&mut rng, structured_data.as_bytes());

    format!(
        "keyId=\"{}\",headers=\"(request-target) host date\",signature=\"{}\"",
        actor.public_key.id,
        base64::encode(signature.as_bytes())
    )
}
