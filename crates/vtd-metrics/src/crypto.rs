use aes_gcm::{
    AeadCore,
    AeadInPlace,
    Aes256Gcm,
    Key,
    KeyInit,
};
use base64::{
    Engine,
    prelude::BASE64_STANDARD,
};
use rand_chacha::ChaChaRng;
use rsa::{
    Pkcs1v15Encrypt,
    RsaPublicKey,
    pkcs8::DecodePublicKey,
    rand_core::SeedableRng,
};
use sha1::{
    Digest,
    Sha1,
};

use crate::error::{
    MetricsError,
    MetricsResult,
};

pub struct MetricsCrypto {
    key_id: String,
    public_key: RsaPublicKey,

    rng: ChaChaRng,
    aes_key: Key<Aes256Gcm>,
}

impl MetricsCrypto {
    pub fn new(public_key: &[u8]) -> MetricsResult<Self> {
        let key_id = {
            let mut hasher = Sha1::new();
            hasher.update(public_key);

            let hash = hasher.finalize();
            BASE64_STANDARD.encode(&hash[..])
        };

        let public_key = RsaPublicKey::from_public_key_der(public_key)
            .map_err(MetricsError::CryptoPublicKeyImport)?;

        let mut rng = ChaChaRng::seed_from_u64(rand::random());
        let aes_key = Aes256Gcm::generate_key(&mut rng);

        Ok(Self {
            key_id,
            public_key,

            rng,
            aes_key,
        })
    }

    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    pub fn encrypt(&mut self, mut payload: &mut [u8]) -> MetricsResult<Vec<u8>> {
        let nonce = Aes256Gcm::generate_nonce(&mut self.rng);

        let cipher = Aes256Gcm::new(&self.aes_key);
        let tag = cipher
            .encrypt_in_place_detached(&nonce, b"", &mut payload)
            .map_err(MetricsError::CryptoEncryptPayload)?;

        let mut crypto_header = [0u8; 0x20 + 0x0C + 0x10];
        crypto_header[0x00..0x20].copy_from_slice(self.aes_key.as_slice());
        crypto_header[0x20..0x2C].copy_from_slice(nonce.as_slice());
        crypto_header[0x2C..0x3C].copy_from_slice(tag.as_slice());

        let mut crypto_header = self
            .public_key
            .encrypt(&mut self.rng, Pkcs1v15Encrypt, &crypto_header)
            .map_err(MetricsError::CryptoEncryptHeader)?;

        crypto_header.extend_from_slice(&payload);
        Ok(crypto_header)
    }
}
