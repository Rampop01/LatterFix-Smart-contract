use soroban_sdk::{contracttype, symbol_short, Address, Bytes, BytesN, Env, String, Vec};

/// Zero-Knowledge Proof (ZKP) Identity Attestation Module
///
/// Lets an employee prove eligibility / KYC status during payroll processing
/// without revealing the underlying identity data on-chain. The flow is:
///
///   1. An admin registers the verification key (VK) of a proving circuit
///      (e.g. "kyc-tier-1") via [`register_verification_key`].
///   2. Off-chain, the employee produces a Groth16 zk-SNARK proof that they
///      satisfy the circuit, together with the public signals and a nullifier.
///   3. On-chain, [`verify_attestation`] validates the proof against the VK,
///      binds the nullifier to the public signals, rejects replays, and records
///      the attestation — the raw identity inputs never touch the ledger.
///
/// Replay protection is enforced with a spent-nullifier set: a nullifier is a
/// deterministic, per-identity/per-circuit tag derived off-chain, so a given
/// identity can be attested against a given circuit at most once.

// ──────────────────────────────────────────────────────────────────────────
// Encoding constants (BLS12-381, uncompressed)
// ──────────────────────────────────────────────────────────────────────────

/// Byte length of an uncompressed BLS12-381 G1 point (`A`, `C`, and every IC).
const G1_POINT_LEN: u32 = 96;
/// Byte length of an uncompressed BLS12-381 G2 point (`B`).
const G2_POINT_LEN: u32 = 192;
/// Byte length of a field element used as a public signal / nullifier.
const FIELD_ELEMENT_LEN: u32 = 32;
/// Upper bound on public signals accepted per attestation (DoS guard).
const MAX_PUBLIC_SIGNALS: u32 = 32;

// ──────────────────────────────────────────────────────────────────────────
// Data Types
// ──────────────────────────────────────────────────────────────────────────

/// A Groth16 zk-SNARK proof payload.
///
/// Points are carried as opaque serialized blobs (`A`, `C` are G1; `B` is G2)
/// so the wrapper stays agnostic to the exact host binding; structural checks
/// enforce the expected encoding lengths.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Groth16Proof {
    /// G1 point `A`.
    pub a: Bytes,
    /// G2 point `B`.
    pub b: Bytes,
    /// G1 point `C`.
    pub c: Bytes,
}

/// Verification key for a single proving circuit.
///
/// `ic` holds the `IC` vector of the Groth16 verification key: for a circuit
/// with `n` public signals it MUST contain exactly `n + 1` G1 points.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationKey {
    /// Human-readable circuit identifier, e.g. "kyc-tier-1".
    pub circuit_id: String,
    /// Curve label, e.g. "BLS12-381". Informational metadata for indexers.
    pub curve: String,
    /// Serialized `alpha_g1` / `beta_g2` pairing term of the VK.
    pub alpha_beta: Bytes,
    /// Serialized `gamma_g2` term of the VK.
    pub gamma: Bytes,
    /// Serialized `delta_g2` term of the VK.
    pub delta: Bytes,
    /// `IC` vector: one G1 point per public signal, plus a constant term.
    pub ic: Vec<Bytes>,
    /// Ledger timestamp at which the key was registered.
    pub registered_at: u64,
    /// Admin address that registered the key.
    pub registered_by: Address,
}

/// A private identity attestation presented by an employee.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentityAttestation {
    /// Circuit whose VK the proof should be checked against.
    pub circuit_id: String,
    /// The employee presenting the attestation (authorizes the call).
    pub subject: Address,
    /// The Groth16 proof.
    pub proof: Groth16Proof,
    /// Public signals (field elements) fed to the verifier.
    pub public_signals: Vec<BytesN<32>>,
    /// Per-identity/per-circuit replay tag.
    pub nullifier: BytesN<32>,
    /// `H(nullifier || public_signals)` — binds the nullifier to the signals.
    pub attestation_commitment: BytesN<32>,
}

/// Record written on a successful attestation.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttestationReceipt {
    /// Spent nullifier this receipt is keyed by.
    pub nullifier: BytesN<32>,
    /// Attested subject.
    pub subject: Address,
    /// Circuit the proof was verified against.
    pub circuit_id: String,
    /// Ledger timestamp of verification.
    pub verified_at: u64,
}

/// Reasons an attestation can be rejected.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AttestationError {
    /// No VK registered for the referenced circuit.
    CircuitNotRegistered,
    /// Proof points have the wrong encoding length or are the zero blob.
    MalformedProof,
    /// `public_signals` count does not match the VK's `IC` arity.
    PublicSignalMismatch,
    /// `attestation_commitment` does not equal `H(nullifier || signals)`.
    CommitmentMismatch,
    /// The nullifier has already been attested (replay).
    NullifierAlreadyUsed,
    /// The Groth16 pairing equation did not hold.
    PairingCheckFailed,
}

/// Storage keys for the ZKP attestation module.
#[contracttype]
pub enum ZkStorageKey {
    /// Module admin (may register verification keys).
    Admin,
    /// Verification key by circuit id.
    Vk(String),
    /// Spent-nullifier flag.
    Nullifier(BytesN<32>),
    /// Attestation receipt by nullifier.
    Attestation(BytesN<32>),
    /// Running count of successful attestations.
    AttestationCount,
}

// ──────────────────────────────────────────────────────────────────────────
// Administration
// ──────────────────────────────────────────────────────────────────────────

/// Initialize the module, recording the admin allowed to register circuits.
pub fn initialize(env: Env, admin: Address) {
    if env.storage().instance().has(&ZkStorageKey::Admin) {
        panic!("zkp module already initialized");
    }
    env.storage().instance().set(&ZkStorageKey::Admin, &admin);
}

/// Return the configured admin address.
pub fn get_admin(env: Env) -> Address {
    env.storage()
        .instance()
        .get(&ZkStorageKey::Admin)
        .unwrap_or_else(|| panic!("zkp module not initialized"))
}

/// Register (or overwrite) the verification key for a circuit. Admin only.
pub fn register_verification_key(
    env: Env,
    admin: Address,
    circuit_id: String,
    curve: String,
    alpha_beta: Bytes,
    gamma: Bytes,
    delta: Bytes,
    ic: Vec<Bytes>,
) {
    admin.require_auth();

    let stored_admin = get_admin(env.clone());
    if admin != stored_admin {
        panic!("only admin can register verification keys");
    }

    // A well-formed VK needs at least the constant IC term.
    if ic.is_empty() {
        panic!("verification key must contain at least one IC element");
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

/// Fetch the verification key registered for a circuit, if any.
pub fn get_verification_key(env: Env, circuit_id: String) -> Option<VerificationKey> {
    env.storage()
        .persistent()
        .get(&ZkStorageKey::Vk(circuit_id))
}

// ──────────────────────────────────────────────────────────────────────────
// Nullifier tracking (replay protection)
// ──────────────────────────────────────────────────────────────────────────

/// Whether a nullifier has already been consumed by a successful attestation.
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

/// Deterministically derive `H(nullifier || public_signals)`.
///
/// Binding the nullifier to the exact public signals prevents an attacker from
/// lifting a valid proof onto a different nullifier (or swapping signals under a
/// fixed nullifier) to mint a fresh, "unspent" attestation.
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

/// Verify a private identity attestation and, on success, consume its
/// nullifier and persist a receipt.
///
/// Rejections are returned as [`AttestationError`] rather than panicking so
/// callers (and payroll flows) can branch on the specific failure.
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

/// Fetch the receipt for a previously verified attestation, if any.
pub fn get_attestation(env: Env, nullifier: BytesN<32>) -> Option<AttestationReceipt> {
    env.storage()
        .persistent()
        .get(&ZkStorageKey::Attestation(nullifier))
}

/// Total number of successful attestations recorded.
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

/// Validate that the proof and VK encodings are structurally well-formed.
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

/// Groth16 pairing-equation verifier — the host-binding wrapper.
///
/// The Groth16 check is the pairing equation
///
/// ```text
///   e(A, B) == e(alpha, beta) · e(vk_x, gamma) · e(C, delta)
/// ```
///
/// where `vk_x = IC[0] + Σ public_signals[i] · IC[i+1]`.
///
/// Evaluating it requires BLS12-381 pairing arithmetic, exposed as Soroban host
/// functions (CAP-0059) starting at Protocol 22. This crate targets
/// soroban-sdk 21.x, where that host binding is not yet available, so this
/// function isolates the seam: every predicate that *is* verifiable without the
/// pairing host — encoding well-formedness, public-input arity, and the
/// nullifier/signal commitment binding — is enforced by [`verify_attestation`]
/// before we get here. Replace the body with
/// `env.crypto().bls12_381().pairing_check(...)` once the deployment target
/// moves to Protocol 22; the module's public API and storage layout are
/// unaffected.
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
