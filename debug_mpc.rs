use frost_ed25519::{Identifier, keys::KeyPackage};
use rand::rngs::OsRng;

fn main() {
    let mut rng = OsRng;
    let participant_id = Identifier::try_from(1u16).unwrap();
    
    // Test key package generation
    let (shares, pubkey_package) = frost_ed25519::keys::generate_with_dealer(
        3, // max_signers
        2, // min_signers
        frost_ed25519::keys::IdentifierList::Default,
        &mut rng,
    ).unwrap();
    
    let our_share = shares.get(&participant_id).unwrap();
    
    // Test serialization
    let serialized = our_share.serialize().unwrap();
    println!("Serialized length: {}", serialized.len());
    
    // Test deserialization
    let deserialized = KeyPackage::deserialize(&serialized);
    match deserialized {
        Ok(_) => println!("✅ Deserialization successful"),
        Err(e) => println!("❌ Deserialization failed: {}", e),
    }
}
