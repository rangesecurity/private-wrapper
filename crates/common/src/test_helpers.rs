use solana_sdk::signature::Keypair;

pub fn test_key() -> Keypair {
    // key for testing, do not use in production
    const TEST_KEY: [u8; 64] = [79, 251, 195, 141, 225, 159, 25, 231, 191, 119, 234, 193, 148, 63, 41, 128, 173, 25, 165, 181, 193, 138, 45, 18, 67, 199, 63, 192, 102, 99, 183, 172, 89, 13, 108, 50, 130, 244, 101, 42, 181, 222, 140, 119, 245, 34, 13, 212, 240, 162, 32, 123, 95, 158, 133, 195, 152, 177, 87, 44, 213, 241, 249, 249];
    Keypair::from_bytes(&TEST_KEY).unwrap()
}