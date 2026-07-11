//! Versioned email encryption with AEAD and HMAC blind index.
//!
//! Each key version provides an independent AES-256-GCM key (for ciphertext)
//! and an HMAC-SHA256 key (for deterministic lookup). The persistent wire
//! format is `base64(version_byte || nonce[12] || ciphertext)`.
//!
//! Blind-index hashes are hex-encoded and must never be stored together with
//! the AEAD key outside `EmailEncryption` — they are used for exact-match
//! lookups only, not for recovery.

use hmac::Mac as _;
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::rand::{SecureRandom, SystemRandom};
use sha2::Sha256;

type HmacSha256 = hmac::Hmac<Sha256>;
/// Wire byte-length for a version-0 / version-1 AES-256-GCM nonce.
const NONCE_BYTES: usize = 12;
/// Extra bytes appended by AES-256-GCM (the authentication tag).
const TAG_BYTES: usize = 16;
/// Maximum plaintext email length we encrypt (arbitrary safety bound).
const MAX_EMAIL_BYTES: usize = 320;

/// Holds the active encryption keys and optional legacy decryption keys.
#[derive(Clone)]
pub struct EmailEncryption {
    active_version: u8,
    active_aead_key_bytes: [u8; 32],
    active_blind_index_key: HmacSha256,
    /// (version, aead_raw_bytes, blind_index_hmac) for decryption during rotation.
    legacy: Vec<(u8, [u8; 32], HmacSha256)>,
}

/// Normalise an email address before encryption or hashing.
fn normalise(email: &str) -> String {
    email.trim().to_ascii_lowercase()
}

// ── Construction ──────────────────────────────────────────────────────────

impl EmailEncryption {
    /// Build from active key material. Returns `None` when encryption is not
    /// configured (keys are all-zero).
    pub fn from_keys(
        active_version: u8,
        active_aead_hex: &str,
        active_blind_hex: &str,
        legacy_hex_pairs: &[(u8, String, String)],
    ) -> Result<Option<Self>, anyhow::Error> {
        let aead_bytes = hex_key(active_aead_hex, "active AEAD key")?;
        let blind_bytes = hex_key(active_blind_hex, "active blind-index key")?;

        if aead_bytes == [0u8; 32] && blind_bytes == [0u8; 32] {
            return Ok(None);
        }
        if aead_bytes == [0u8; 32] || blind_bytes == [0u8; 32] {
            anyhow::bail!("email encryption keys are partially configured — refusing to start");
        }

        let active_aead_key_bytes = aead_bytes;
        let active_blind_index_key = hmac_key(&blind_bytes);
        // Drop local key copies — the struct fields are the authority.
        let _blind_bytes = blind_bytes;
        let _aead_bytes = aead_bytes;

        let mut legacy = Vec::with_capacity(legacy_hex_pairs.len());
        for &(version, ref aead_hex, ref blind_hex) in legacy_hex_pairs {
            let la = hex_key(aead_hex, "legacy AEAD key")?;
            let lb = hex_key(blind_hex, "legacy blind-index key")?;
            if la == [0u8; 32] || lb == [0u8; 32] {
                anyhow::bail!("legacy email encryption key v{version} is partially configured");
            }
            let l_hmac = hmac_key(&lb);
            legacy.push((version, la, l_hmac));
        }

        Ok(Some(Self { active_version, active_aead_key_bytes, active_blind_index_key, legacy }))
    }
}

// ── Public API ────────────────────────────────────────────────────────────

impl EmailEncryption {
    /// Encrypt `email`, returning the base64 wire format.
    pub fn encrypt(&self, email: &str) -> Result<String, anyhow::Error> {
        let plain = normalise(email);
        if plain.len() > MAX_EMAIL_BYTES {
            anyhow::bail!("email too long to encrypt");
        }

        let mut nonce_bytes = [0u8; NONCE_BYTES];
        SystemRandom::new()
            .fill(&mut nonce_bytes)
            .map_err(|_| anyhow::anyhow!("CSPRNG failure"))?;

        let unbound = UnboundKey::new(&AES_256_GCM, &self.active_aead_key_bytes)
            .map_err(|_| anyhow::anyhow!("invalid AEAD key"))?;
        let key = LessSafeKey::new(unbound);
        let nonce = Nonce::assume_unique_for_key(nonce_bytes);

        // ring seal_in_place_append_tag appends the 16-byte tag to buf.
        let mut buf = plain.into_bytes();
        key.seal_in_place_append_tag(nonce, Aad::empty(), &mut buf)
            .map_err(|_| anyhow::anyhow!("encryption failed"))?;
        // buf is now [ciphertext || tag]
        let ct_len = buf.len() - TAG_BYTES;
        let wire: Vec<u8> = std::iter::once(self.active_version)
            .chain(nonce_bytes)
            .chain(buf[..ct_len].iter().copied())
            .chain(buf[ct_len..].iter().copied())
            .collect();
        Ok(base64_encode(&wire))
    }

    /// Decrypt `wire` (base64) back to the normalised email.
    pub fn decrypt(&self, wire: &str) -> Result<String, anyhow::Error> {
        let raw = base64_decode(wire, "email ciphertext")?;
        if raw.len() < 1 + NONCE_BYTES + TAG_BYTES {
            anyhow::bail!("email ciphertext too short");
        }
        let version = raw[0];
        let nonce_bytes: &[u8; NONCE_BYTES] = raw[1..1 + NONCE_BYTES]
            .try_into()
            .map_err(|_| anyhow::anyhow!("invalid nonce length"))?;
        let ct_and_tag = &raw[1 + NONCE_BYTES..];

        let aead_bytes = self.aead_key_for(version)?;
        let unbound = UnboundKey::new(&AES_256_GCM, aead_bytes)
            .map_err(|_| anyhow::anyhow!("invalid AEAD key"))?;
        let key = LessSafeKey::new(unbound);
        let nonce = Nonce::assume_unique_for_key(*nonce_bytes);

        let mut buf = ct_and_tag.to_vec();
        let plain = key
            .open_in_place(nonce, Aad::empty(), &mut buf)
            .map_err(|_| anyhow::anyhow!("decryption failed — wrong key or tampered ciphertext"))?;
        String::from_utf8(plain.to_vec())
            .map_err(|_| anyhow::anyhow!("invalid UTF-8 in decrypted email"))
    }

    /// Hex-encoded HMAC blind index for exact-match lookups using the active key.
    pub fn blind_index(&self, email: &str) -> String {
        blind_index_hex(&self.active_blind_index_key, email)
    }

    /// Blind index for a specific legacy version (None if unknown).
    pub fn blind_index_for_version(&self, version: u8, email: &str) -> Option<String> {
        if version == self.active_version {
            return Some(self.blind_index(email));
        }
        self.legacy
            .iter()
            .find(|(v, _, _)| *v == version)
            .map(|(_, _, hmac)| blind_index_hex(hmac, email))
    }

    /// The key version used for new encryptions.
    pub fn active_version(&self) -> u8 {
        self.active_version
    }

    /// All key versions that can decrypt (active + legacy).
    pub fn known_versions(&self) -> Vec<u8> {
        let mut versions: Vec<u8> = self.legacy.iter().map(|(v, _, _)| *v).collect();
        versions.push(self.active_version);
        versions
    }

    // ── helpers ────────────────────────────────────────────────────────

    fn aead_key_for(&self, version: u8) -> Result<&[u8; 32], anyhow::Error> {
        if version == self.active_version {
            return Ok(&self.active_aead_key_bytes);
        }
        self.legacy
            .iter()
            .find(|(v, _, _)| *v == version)
            .map(|(_, bytes, _)| bytes)
            .ok_or_else(|| anyhow::anyhow!("unknown email encryption key version {version}"))
    }
}

// ── Free helpers ──────────────────────────────────────────────────────────

fn blind_index_hex(hmac: &HmacSha256, email: &str) -> String {
    let normalised = normalise(email);
    let mut mac = hmac.clone();
    mac.update(normalised.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

fn hex_key(hex_str: &str, label: &str) -> Result<[u8; 32], anyhow::Error> {
    let trimmed = hex_str.trim();
    if trimmed.len() != 64 {
        anyhow::bail!("{label} must be 64 hex characters (32 bytes), got {}", trimmed.len());
    }
    let mut key = [0u8; 32];
    hex::decode_to_slice(trimmed, &mut key)
        .map_err(|_| anyhow::anyhow!("{label} is not valid hex"))?;
    Ok(key)
}

fn hmac_key(bytes: &[u8; 32]) -> HmacSha256 {
    HmacSha256::new_from_slice(bytes).expect("HMAC-SHA256 accepts 32-byte keys")
}

fn base64_encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn base64_decode(encoded: &str, label: &str) -> Result<Vec<u8>, anyhow::Error> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| anyhow::anyhow!("{label} is not valid base64"))
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_keys() -> EmailEncryption {
        // deterministic test keys (do NOT use in production)
        let aead = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
        let blind = "1f1e1d1c1b1a191817161514131211100f0e0d0c0b0a09080706050403020100";
        EmailEncryption::from_keys(1, aead, blind, &[]).unwrap().unwrap()
    }

    fn test_keys_v0v1() -> EmailEncryption {
        let aead0 = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
        let blind0 = "1f1e1d1c1b1a191817161514131211100f0e0d0c0b0a09080706050403020100";
        let aead1 = "101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f";
        let blind1 = "2f2e2d2c2b2a292827262524232221201f1e1d1c1b1a19181716151413121110";
        EmailEncryption::from_keys(1, aead1, blind1, &[(0, aead0.to_string(), blind0.to_string())])
            .unwrap()
            .unwrap()
    }

    #[test]
    fn roundtrip_same_email() {
        let enc = test_keys();
        let wire = enc.encrypt("test@tongji.edu.cn").unwrap();
        let decrypted = enc.decrypt(&wire).unwrap();
        assert_eq!(decrypted, "test@tongji.edu.cn");
    }

    #[test]
    fn ciphertext_is_nondeterministic() {
        let enc = test_keys();
        let w1 = enc.encrypt("a@tongji.edu.cn").unwrap();
        let w2 = enc.encrypt("a@tongji.edu.cn").unwrap();
        assert_ne!(w1, w2, "same email must produce different ciphertexts");
    }

    #[test]
    fn blind_index_is_deterministic() {
        let enc = test_keys();
        let h1 = enc.blind_index("Test@Tongji.edu.cn");
        let h2 = enc.blind_index("test@tongji.edu.cn");
        assert_eq!(h1, h2, "blind index must be case-normalized");
        assert_eq!(h1.len(), 64, "HMAC-SHA256 hex is 64 chars");
    }

    #[test]
    fn blind_index_differs_per_email() {
        let enc = test_keys();
        assert_ne!(enc.blind_index("a@tongji.edu.cn"), enc.blind_index("b@tongji.edu.cn"));
    }

    #[test]
    fn blind_index_differs_per_key() {
        let enc = test_keys_v0v1();
        let email = "x@tongji.edu.cn";
        assert_ne!(enc.blind_index(email), enc.blind_index_for_version(0, email).unwrap());
    }

    #[test]
    fn tampered_ciphertext_fails() {
        let enc = test_keys();
        let mut wire = enc.encrypt("real@tongji.edu.cn").unwrap();
        // flip a bit near the end (in the tag)
        wire.replace_range(wire.len() - 2..wire.len() - 1, "X");
        assert!(enc.decrypt(&wire).is_err());
    }

    #[test]
    fn truncated_ciphertext_fails() {
        let enc = test_keys();
        let wire = enc.encrypt("real@tongji.edu.cn").unwrap();
        assert!(enc.decrypt(&wire[..wire.len() - 8]).is_err());
    }

    #[test]
    fn unknown_key_version_fails() {
        let enc = test_keys();
        let wire = enc.encrypt("test@tongji.edu.cn").unwrap();
        // flip version byte from 1 to 99
        let raw = base64_decode(&wire, "wire").unwrap();
        let mut raw = raw;
        raw[0] = 99;
        let bad_wire = base64_encode(&raw);
        assert!(enc.decrypt(&bad_wire).is_err());
    }

    #[test]
    fn key_rotation_reads_old_writes_new() {
        let enc = test_keys_v0v1();
        // simulate v0 ciphertext
        let aead0_bytes =
            hex_key("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f", "v0")
                .unwrap();
        let unbound = UnboundKey::new(&AES_256_GCM, &aead0_bytes).unwrap();
        let key = LessSafeKey::new(unbound);
        let mut nonce_bytes = [0u8; NONCE_BYTES];
        SystemRandom::new().fill(&mut nonce_bytes).unwrap();
        let nonce = Nonce::assume_unique_for_key(nonce_bytes);
        let plain = b"old@tongji.edu.cn";
        let mut buf = plain.to_vec();
        key.seal_in_place_append_tag(nonce, Aad::empty(), &mut buf).unwrap();
        let ct_len = buf.len() - TAG_BYTES;
        let wire: Vec<u8> = std::iter::once(0u8)
            .chain(nonce_bytes)
            .chain(buf[..ct_len].iter().copied())
            .chain(buf[ct_len..].iter().copied())
            .collect();
        let wire_b64 = base64_encode(&wire);

        // should decrypt with rotation
        assert_eq!(enc.decrypt(&wire_b64).unwrap(), "old@tongji.edu.cn");

        // new writes use v1
        let new = enc.encrypt("new@tongji.edu.cn").unwrap();
        let raw = base64_decode(&new, "new").unwrap();
        assert_eq!(raw[0], 1);
    }

    #[test]
    fn unconfigured_returns_none() {
        let enc = EmailEncryption::from_keys(
            1,
            "0000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000000",
            &[],
        )
        .unwrap();
        assert!(enc.is_none());
    }

    #[test]
    fn partially_configured_fails() {
        assert!(EmailEncryption::from_keys(
            1,
            "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
            "0000000000000000000000000000000000000000000000000000000000000000",
            &[],
        )
        .is_err());
    }
}
