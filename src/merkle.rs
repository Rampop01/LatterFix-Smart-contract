use soroban_sdk::{Bytes, BytesN, Env, Vec};

pub fn verify_merkle_proof(
    env: &Env,
    root: &BytesN<32>,
    leaf: &BytesN<32>,
    proof: &Vec<BytesN<32>>,
) -> bool {
    let mut computed_hash = leaf.clone();

    for proof_element in proof.iter() {
        let mut data = Bytes::new(env);

        // Standard convention: sort sibling and computed hash lexicographically
        if computed_hash <= proof_element {
            data.append(&computed_hash.into());
            data.append(&proof_element.into());
        } else {
            data.append(&proof_element.into());
            data.append(&computed_hash.into());
        }

        computed_hash = env.crypto().sha256(&data).into();
    }

    computed_hash == *root
}
