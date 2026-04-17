pub mod errors;
pub mod jwt;
pub mod password;
pub mod email;

use rand::RngCore;
use rand::rngs::OsRng;

/// Generate a cryptographically secure hex token
pub fn generate_token(len: usize) -> String {
    let mut bytes = vec![0u8; len];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}
