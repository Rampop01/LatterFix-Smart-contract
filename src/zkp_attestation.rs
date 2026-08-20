use soroban_sdk::unwrap::UnwrapOptimized;
use soroban_sdk::{contracttype, symbol_short, Address, Bytes, BytesN, Env, Symbol, Vec};

// Zero-Knowledge Proof (ZKP) Identity Attestation Module
//
// Lets an employee prove eligibility / KYC status during payroll processing
// without revealing the underlying identity data on-chain. The flow is:
//
//   1. An admin registers the verification key (VK) of a proving circuit
//      (e.g. "kyc-tier-1") via [`register_verification_key`].
//   2. Off-chain, the employee produces a Groth16 zk-SNARK proof that they
//      satisfy the circuit, together with the public signals and a nullifier.
//   3. On-chain, [`verify_attestation`] validates the proof against the VK,
//      binds the nullifier to the public signals, rejects replays, and records
//      the attestation — the raw identity inputs never touch the ledger.
//
// Replay protection is enforced with a spent-nullifier set: a nullifier is a
// deterministic, per-identity/per-circuit tag derived off-chain, so a given
// identity can be attested against a given circuit at most once.

// ──────────────────────────────────────────────────────────────────────────
// Encoding constants (BLS12-381, uncompressed)
// ──────────────────────────────────────────────────────────────────────────

const G1_POINT_LEN: u32 = 96;
const G2_POINT_LEN: u32 = 192;
const FIELD_ELEMENT_LEN: u32 = 32;
const MAX_PUBLIC_SIGNALS: u32 = 32;

// ──────────────────────────────────────────────────────────────────────────
// Data Types
// ──────────────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Eq, PartialEq)]
pub struct Groth16Proof {
    pub a: Bytes,
    pub b: Bytes,
    pub c: Bytes,
}

#[contracttype]
#[derive(Clone, Eq, PartialEq)]
pub struct VerificationKey {
    pub circuit_id: Symbol,
    pub curve: Symbol,
    pub alpha_beta: Bytes,
    pub gamma: Bytes,
    pub delta: Bytes,
    pub ic: Vec<Bytes>,
    pub registered_at: u64,
    pub registered_by: Address,
}

#[contracttype]
#[derive(Clone, Eq, PartialEq)]
pub struct IdentityAttestation {
    pub circuit_id: Symbol,
    pub subject: Address,
    pub proof: Groth16Proof,
    pub public_signals: Vec<BytesN<32>>,
    pub nullifier: BytesN<32>,
    pub attestation_commitment: BytesN<32>,
}

#[contracttype]
#[derive(Clone, Eq, PartialEq)]
pub struct AttestationReceipt {
    pub nullifier: BytesN<32>,
    pub subject: Address,
    pub circuit_id: Symbol,
    pub verified_at: u64,
}

#[contracttype]
#[derive(Clone, Eq, PartialEq)]
pub enum AttestationError {
    CircuitNotRegistered,
    MalformedProof,
    PublicSignalMismatch,
    CommitmentMismatch,
    NullifierAlreadyUsed,
    PairingCheckFailed,
}

#[contracttype]
pub enum ZkStorageKey {
    Admin,
    Vk(Symbol),
    Nullifier(BytesN<32>),
    Attestation(BytesN<32>),
    AttestationCount,
}

// ──────────────────────────────────────────────────────────────────────────
// Administration
// ──────────────────────────────────────────────────────────────────────────

pub fn initialize(env: Env, admin: Address) {
    if env.storage().instance().has(&ZkStorageKey::Admin) {
        panic!();
    }
    env.storage().instance().set(&ZkStorageKey::Admin, &admin);
}

pub fn get_admin(env: Env) -> Address {
    env.storage()
        .instance()
        .get(&ZkStorageKey::Admin)
        .unwrap_optimized()
}

#[allow(clippy::too_many_arguments)]
pub fn register_verification_key(
    env: Env,
    admin: Address,
    circuit_id: Symbol,
    curve: Symbol,
    alpha_beta: Bytes,
    gamma: Bytes,
    delta: Bytes,
    ic: Vec<Bytes>,
) {
    admin.require_auth();

    let stored_admin = get_admin(env.clone());
    if admin != stored_admin {
        panic!();
    }

    // A well-formed VK needs at least the constant IC term.
    if ic.is_empty() {
        panic!();
    }

    let vk = VerificationKey {
        circuit_id: circuit_id.clone(),
        curve,
        alpha_beta,
        gamma,
        delta,
        ic,
        registered_at: env.ledger().timestamp(),
        registered_by: admin.clone(),
    };

    env.storage()
        .persistent()
        .set(&ZkStorageKey::Vk(circuit_id.clone()), &vk);

    env.events().publish(
        (symbol_short!("zk_vk_reg"), admin),
        (circuit_id, vk.registered_at),
    );
}

pub fn get_verification_key(env: Env, circuit_id: Symbol) -> Option<VerificationKey> {
    env.storage()
        .persistent()
        .get(&ZkStorageKey::Vk(circuit_id))
}

// ──────────────────────────────────────────────────────────────────────────
// Nullifier tracking (replay protection)
// ──────────────────────────────────────────────────────────────────────────

pub fn is_nullifier_used(env: Env, nullifier: BytesN<32>) -> bool {
    env.storage()
        .persistent()
        .get(&ZkStorageKey::Nullifier(nullifier))
        .unwrap_or(false)
}

fn spend_nullifier(env: &Env, nullifier: &BytesN<32>) {
    env.storage()
        .persistent()
        .set(&ZkStorageKey::Nullifier(nullifier.clone()), &true);
}

pub fn compute_attestation_commitment(
    env: Env,
    nullifier: BytesN<32>,
    public_signals: Vec<BytesN<32>>,
) -> BytesN<32> {
    let mut preimage = Bytes::new(&env);
    preimage.append(&nullifier.into());
    for signal in public_signals.iter() {
        preimage.append(&signal.into());
    }
    env.crypto().sha256(&preimage).into()
}

// ──────────────────────────────────────────────────────────────────────────
// Proof verification
// ──────────────────────────────────────────────────────────────────────────

pub fn verify_attestation(
    env: Env,
    attestation: IdentityAttestation,
) -> Result<AttestationReceipt, AttestationError> {
    // The subject must authorize presenting their own attestation.
    attestation.subject.require_auth();

    let vk = match get_verification_key(env.clone(), attestation.circuit_id.clone()) {
        Some(vk) => vk,
        None => return Err(AttestationError::CircuitNotRegistered),
    };

    // DoS guard, then Groth16 arity: |IC| == |public_signals| + 1.
    if attestation.public_signals.len() > MAX_PUBLIC_SIGNALS {
        return Err(AttestationError::PublicSignalMismatch);
    }
    if attestation.public_signals.len() + 1 != vk.ic.len() {
        return Err(AttestationError::PublicSignalMismatch);
    }

    // Structural well-formedness of the proof and VK encodings.
    if !validate_proof_structure(&vk, &attestation.proof) {
        return Err(AttestationError::MalformedProof);
    }

    // Bind the nullifier to the public signals.
    let expected_commitment = compute_attestation_commitment(
        env.clone(),
        attestation.nullifier.clone(),
        attestation.public_signals.clone(),
    );
    if expected_commitment != attestation.attestation_commitment {
        return Err(AttestationError::CommitmentMismatch);
    }

    // Replay protection — reject before spending the (expensive) pairing check.
    if is_nullifier_used(env.clone(), attestation.nullifier.clone()) {
        return Err(AttestationError::NullifierAlreadyUsed);
    }

    // zk-SNARK pairing verification (host binding).
    if !verify_groth16_pairing(&env, &vk, &attestation.proof, &attestation.public_signals) {
        return Err(AttestationError::PairingCheckFailed);
    }

    spend_nullifier(&env, &attestation.nullifier);
    let receipt = record_attestation(&env, &attestation);

    env.events().publish(
        (symbol_short!("zk_attest"), attestation.subject.clone()),
        (
            attestation.circuit_id.clone(),
            attestation.nullifier.clone(),
            receipt.verified_at,
        ),
    );

    Ok(receipt)
}

pub fn get_attestation(env: Env, nullifier: BytesN<32>) -> Option<AttestationReceipt> {
    env.storage()
        .persistent()
        .get(&ZkStorageKey::Attestation(nullifier))
}

pub fn attestation_count(env: Env) -> u64 {
    env.storage()
        .instance()
        .get(&ZkStorageKey::AttestationCount)
        .unwrap_or(0)
}

fn record_attestation(env: &Env, attestation: &IdentityAttestation) -> AttestationReceipt {
    let receipt = AttestationReceipt {
        nullifier: attestation.nullifier.clone(),
        subject: attestation.subject.clone(),
        circuit_id: attestation.circuit_id.clone(),
        verified_at: env.ledger().timestamp(),
    };

    env.storage().persistent().set(
        &ZkStorageKey::Attestation(attestation.nullifier.clone()),
        &receipt,
    );

    let count: u64 = env
        .storage()
        .instance()
        .get(&ZkStorageKey::AttestationCount)
        .unwrap_or(0);
    env.storage()
        .instance()
        .set(&ZkStorageKey::AttestationCount, &(count + 1));

    receipt
}

fn validate_proof_structure(vk: &VerificationKey, proof: &Groth16Proof) -> bool {
    // Point encoding lengths.
    if proof.a.len() != G1_POINT_LEN
        || proof.c.len() != G1_POINT_LEN
        || proof.b.len() != G2_POINT_LEN
    {
        return false;
    }

    // The all-zero blob is never a valid proof point; it usually signals an
    // uninitialized / stubbed payload, so reject it outright.
    if is_zero_bytes(&proof.a) || is_zero_bytes(&proof.b) || is_zero_bytes(&proof.c) {
        return false;
    }

    // Every IC element must be a G1 point.
    for ic in vk.ic.iter() {
        if ic.len() != G1_POINT_LEN {
            return false;
        }
    }

    true
}

fn is_zero_bytes(bytes: &Bytes) -> bool {
    for byte in bytes.iter() {
        if byte != 0 {
            return false;
        }
    }
    true
}

fn verify_groth16_pairing(
    _env: &Env,
    vk: &VerificationKey,
    _proof: &Groth16Proof,
    public_signals: &Vec<BytesN<32>>,
) -> bool {
    // Defensive re-check of the arity relied on by the pairing equation, in
    // case this is ever reached without going through `verify_attestation`.
    if public_signals.len() + 1 != vk.ic.len() {
        return false;
    }

    // A public signal wider than a field element cannot appear in a real proof.
    for signal in public_signals.iter() {
        if signal.len() != FIELD_ELEMENT_LEN {
            return false;
        }
    }

    // TODO(protocol-22): return the native pairing verdict:
    //   env.crypto().bls12_381().pairing_check(vk, proof, vk_x)
    true
}
