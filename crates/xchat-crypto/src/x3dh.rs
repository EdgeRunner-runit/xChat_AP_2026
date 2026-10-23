use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::{
    errors::CryptoError,
    identity::{IdentityKeyPair, IdentityPublicKey},
    kdf::kdf_x3dh,
};

/// Signed prekey pair created by an xChat node and periodically rotated.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct SignedPrekeyPair {
    pub secret: StaticSecret,
    #[zeroize(skip)]
    pub pubkey: [u8; 32],
    #[zeroize(skip)]
    pub signature: [u8; 64],
}

impl SignedPrekeyPair {
    pub fn generate(identity: &IdentityKeyPair) -> Self {
        let rng = OsRng;
        let secret = StaticSecret::random_from_rng(rng);
        let pubkey = X25519PublicKey::from(&secret).to_bytes();
        let signature = identity.sign(&pubkey);
        Self {
            secret,
            pubkey,
            signature,
        }
    }
}

/// One-time prekey pair used to guarantee forward secrecy during asynchronous initial exchange.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct OneTimePrekeyPair {
    pub id: u32,
    pub secret: StaticSecret,
    #[zeroize(skip)]
    pub pubkey: [u8; 32],
}

impl OneTimePrekeyPair {
    pub fn generate(id: u32) -> Self {
        let rng = OsRng;
        let secret = StaticSecret::random_from_rng(rng);
        let pubkey = X25519PublicKey::from(&secret).to_bytes();
        Self { id, secret, pubkey }
    }
}

mod serde_sig {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(bytes)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = Vec::<u8>::deserialize(deserializer)?;
        if v.len() != 64 {
            return Err(serde::de::Error::custom("expected 64 bytes signature"));
        }
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&v);
        Ok(arr)
    }
}

/// Published prekey bundle allowing offline peers to initiate secure sessions asynchronously.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct PrekeyBundle {
    pub identity_key: IdentityPublicKey,
    pub signed_prekey: [u8; 32],
    #[serde(with = "serde_sig")]
    pub signed_prekey_signature: [u8; 64],
    pub one_time_prekey: Option<[u8; 32]>,
}

/// Header sent with the initial message to allow the recipient to complete the X3DH calculation.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct X3dhInitMessage {
    pub sender_identity: IdentityPublicKey,
    pub ephemeral_pubkey: [u8; 32],
    pub used_one_time_prekey: Option<[u8; 32]>,
}

/// Sender initiates X3DH key agreement using recipient's public prekey bundle.
pub fn x3dh_initiate(
    sender_identity: &IdentityKeyPair,
    recipient_bundle: &PrekeyBundle,
) -> Result<(X3dhInitMessage, [u8; 32]), CryptoError> {
    // 1. Verify recipient's signed prekey signature
    recipient_bundle.identity_key.verify(
        &recipient_bundle.signed_prekey,
        &recipient_bundle.signed_prekey_signature,
    )?;

    // 2. Generate sender ephemeral DH keypair
    let rng = OsRng;
    let ephemeral_secret = StaticSecret::random_from_rng(rng);
    let ephemeral_pubkey = X25519PublicKey::from(&ephemeral_secret).to_bytes();

    // 3. Compute DH components
    // DH1 = DH(IK_sender, SPK_recipient)
    let dh1 = sender_identity.diffie_hellman(&recipient_bundle.signed_prekey);

    // DH2 = DH(EK_sender, IK_recipient)
    let dh2 = ephemeral_secret
        .diffie_hellman(&X25519PublicKey::from(
            recipient_bundle.identity_key.dh_pubkey,
        ))
        .to_bytes();

    // DH3 = DH(EK_sender, SPK_recipient)
    let dh3 = ephemeral_secret
        .diffie_hellman(&X25519PublicKey::from(recipient_bundle.signed_prekey))
        .to_bytes();

    let mut dh_concat = Vec::with_capacity(128);
    dh_concat.extend_from_slice(&dh1);
    dh_concat.extend_from_slice(&dh2);
    dh_concat.extend_from_slice(&dh3);

    // DH4 = DH(EK_sender, OPK_recipient) if available
    let used_one_time_prekey = if let Some(opk) = recipient_bundle.one_time_prekey {
        let dh4 = ephemeral_secret
            .diffie_hellman(&X25519PublicKey::from(opk))
            .to_bytes();
        dh_concat.extend_from_slice(&dh4);
        Some(opk)
    } else {
        None
    };

    // 4. Derive initial master shared secret via HKDF
    let shared_secret = kdf_x3dh(&dh_concat);

    let init_msg = X3dhInitMessage {
        sender_identity: sender_identity.public_key(),
        ephemeral_pubkey,
        used_one_time_prekey,
    };

    Ok((init_msg, shared_secret))
}

/// Recipient calculates the identical shared secret upon receiving sender's X3DH init message.
pub fn x3dh_respond(
    recipient_identity: &IdentityKeyPair,
    recipient_signed_prekey: &StaticSecret,
    recipient_one_time_prekeys: &[OneTimePrekeyPair],
    init_msg: &X3dhInitMessage,
) -> Result<[u8; 32], CryptoError> {
    let sender_ik = X25519PublicKey::from(init_msg.sender_identity.dh_pubkey);
    let sender_ek = X25519PublicKey::from(init_msg.ephemeral_pubkey);

    // DH1 = DH(SPK_recipient, IK_sender)
    let dh1 = recipient_signed_prekey
        .diffie_hellman(&sender_ik)
        .to_bytes();

    // DH2 = DH(IK_recipient, EK_sender)
    let dh2 = recipient_identity
        .dh_secret
        .diffie_hellman(&sender_ek)
        .to_bytes();

    // DH3 = DH(SPK_recipient, EK_sender)
    let dh3 = recipient_signed_prekey
        .diffie_hellman(&sender_ek)
        .to_bytes();

    let mut dh_concat = Vec::with_capacity(128);
    dh_concat.extend_from_slice(&dh1);
    dh_concat.extend_from_slice(&dh2);
    dh_concat.extend_from_slice(&dh3);

    // DH4 = DH(OPK_recipient, EK_sender) if requested
    if let Some(used_opk_bytes) = init_msg.used_one_time_prekey {
        let opk_match = recipient_one_time_prekeys
            .iter()
            .find(|pair| pair.pubkey == used_opk_bytes)
            .ok_or_else(|| CryptoError::X3dhError("Referenced One-Time Prekey not found".into()))?;
        let dh4 = opk_match.secret.diffie_hellman(&sender_ek).to_bytes();
        dh_concat.extend_from_slice(&dh4);
    }

    let shared_secret = kdf_x3dh(&dh_concat);
    Ok(shared_secret)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_x3dh_key_agreement_with_opk() {
        let alice_identity = IdentityKeyPair::generate();
        let bob_identity = IdentityKeyPair::generate();

        let bob_spk = SignedPrekeyPair::generate(&bob_identity);
        let bob_opk = OneTimePrekeyPair::generate(1);

        let bob_bundle = PrekeyBundle {
            identity_key: bob_identity.public_key(),
            signed_prekey: bob_spk.pubkey,
            signed_prekey_signature: bob_spk.signature,
            one_time_prekey: Some(bob_opk.pubkey),
        };

        // Alice initiates
        let (init_msg, alice_secret) =
            x3dh_initiate(&alice_identity, &bob_bundle).expect("initiate succeeds");

        // Bob responds
        let bob_secret = x3dh_respond(&bob_identity, &bob_spk.secret, &[bob_opk], &init_msg)
            .expect("respond succeeds");

        assert_eq!(alice_secret, bob_secret);
    }

    #[test]
    fn test_x3dh_key_agreement_without_opk() {
        let alice_identity = IdentityKeyPair::generate();
        let bob_identity = IdentityKeyPair::generate();

        let bob_spk = SignedPrekeyPair::generate(&bob_identity);

        let bob_bundle = PrekeyBundle {
            identity_key: bob_identity.public_key(),
            signed_prekey: bob_spk.pubkey,
            signed_prekey_signature: bob_spk.signature,
            one_time_prekey: None,
        };

        let (init_msg, alice_secret) =
            x3dh_initiate(&alice_identity, &bob_bundle).expect("initiate succeeds");

        let bob_secret =
            x3dh_respond(&bob_identity, &bob_spk.secret, &[], &init_msg).expect("respond succeeds");

        assert_eq!(alice_secret, bob_secret);
    }
}
