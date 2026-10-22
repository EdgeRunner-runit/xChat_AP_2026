use hmac::{Hmac, Mac};
use sha2::Sha256;
use zeroize::{Zeroize, ZeroizeOnDrop};

type HmacSha256 = Hmac<Sha256>;

pub const DR_ROOT_INFO: &[u8] = b"xchat-dr-root-v1";
pub const DR_CHAIN_ADVANCE: &[u8] = &[0x01];
pub const DR_MESSAGE_KEY: &[u8] = &[0x02];
pub const X3DH_INFO: &[u8] = b"xchat-x3dh-v1";
pub const DISCOVERY_INFO: &[u8] = b"xchat-discovery-token-v1";

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SecretKey32(pub [u8; 32]);

impl SecretKey32 {
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Derive new root key and chain key from an existing root key and a DH shared secret.
pub fn kdf_rk(root_key: &[u8; 32], dh_out: &[u8; 32]) -> ([u8; 32], [u8; 32]) {
    let hk = hkdf::Hkdf::<Sha256>::new(Some(root_key), dh_out);
    let mut okm = [0u8; 64];
    hk.expand(DR_ROOT_INFO, &mut okm)
        .expect("64 bytes is valid for HKDF-SHA256");

    let mut next_root = [0u8; 32];
    let mut chain_key = [0u8; 32];
    next_root.copy_from_slice(&okm[..32]);
    chain_key.copy_from_slice(&okm[32..]);
    (next_root, chain_key)
}

/// Advance symmetric chain and produce (next_chain_key, message_key).
pub fn kdf_ck(chain_key: &[u8; 32]) -> ([u8; 32], [u8; 32]) {
    let mut mac_msg = HmacSha256::new_from_slice(chain_key).expect("HMAC can take 32 byte key");
    mac_msg.update(DR_MESSAGE_KEY);
    let message_key: [u8; 32] = mac_msg.finalize().into_bytes().into();

    let mut mac_next = HmacSha256::new_from_slice(chain_key).expect("HMAC can take 32 byte key");
    mac_next.update(DR_CHAIN_ADVANCE);
    let next_chain_key: [u8; 32] = mac_next.finalize().into_bytes().into();

    (next_chain_key, message_key)
}

/// Derive shared secret for X3DH.
pub fn kdf_x3dh(dh_concat: &[u8]) -> [u8; 32] {
    let salt = [0u8; 32]; // 32 bytes of zeros according to X3DH spec
    let hk = hkdf::Hkdf::<Sha256>::new(Some(&salt), dh_concat);
    let mut secret = [0u8; 32];
    hk.expand(X3DH_INFO, &mut secret)
        .expect("32 bytes is valid for HKDF-SHA256");
    secret
}

/// Derive rotating ephemeral discovery token from identity key and epoch.
pub fn derive_discovery_token(identity_pubkey: &[u8; 32], epoch: u64) -> [u8; 16] {
    let mut epoch_bytes = [0u8; 8];
    epoch_bytes.copy_from_slice(&epoch.to_be_bytes());

    let hk = hkdf::Hkdf::<Sha256>::new(Some(identity_pubkey), &epoch_bytes);
    let mut token = [0u8; 16];
    hk.expand(DISCOVERY_INFO, &mut token)
        .expect("16 bytes is valid for HKDF-SHA256");
    token
}
