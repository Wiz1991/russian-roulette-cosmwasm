use rand_chacha::ChaChaRng;
use rand_core::{RngCore, SeedableRng};
use sha2::{Digest, Sha256};
pub fn sha_256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let hash = hasher.finalize();

    let mut result = [0u8; 32];
    result.copy_from_slice(hash.as_slice());
    result
}

pub fn new_entropy(input: &[[u8; 8]], seed: &[u8]) -> [u8; 32] {
    let mut rng_entropy = vec![];
    for entropy in input {
        rng_entropy.extend_from_slice(entropy);
    }
    let mut rng = CryptoRandom::new(seed, &rng_entropy);

    rng.rand_bytes()
}
pub struct CryptoRandom {
    generator: ChaChaRng,
}
impl CryptoRandom {
    pub fn new(seed: &[u8], entropy: &[u8]) -> Self {
        let mut hasher = Sha256::new();

        hasher.update(&seed);
        hasher.update(&entropy);
        let hash = hasher.finalize();
        let mut hash_slice = [0u8; 32];
        hash_slice.copy_from_slice(hash.as_slice());

        Self {
            generator: ChaChaRng::from_seed(hash_slice),
        }
    }
    pub fn rand_bytes(&mut self) -> [u8; 32] {
        let mut result = [0u8; 32];
        self.generator.fill_bytes(&mut result);
        result
    }
}
