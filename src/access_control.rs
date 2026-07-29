use soroban_sdk::{contracttype, Address, Env, Vec};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Role {
    Admin,
    Manager,
    Moderator,
    Verifier,
    /// Emergency guardian: authorized to veto a pending contract WASM
    /// upgrade during its timelock window (see `upgrade.rs`). Deliberately
    /// separate from `Admin` so upgrade proposals can be checked by a party
    /// other than the one proposing them.
    Guardian,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleData {
    pub role: Role,
    pub granted_at: u64,
    pub granted_by: Address,
}

#[contracttype]
pub enum AccessControlKey {
    Role(Address),
    RoleMembers(Role),
    Admin,
}

pub fn grant_role(env: Env, admin: Address, user: Address, role: Role) {
    admin.require_auth();

    let stored_admin: Address = env
        .storage()
        .instance()
        .get(&AccessControlKey::Admin)
        .unwrap_or_else(|| panic!("not initialized"));

    if admin != stored_admin {
        panic!("only admin can grant roles");
    }

    let key = AccessControlKey::Role(user.clone());
    let role_data = RoleData {
        role: role.clone(),
        granted_at: env.ledger().timestamp(),
        granted_by: admin,
    };

    env.storage().instance().set(&key, &role_data);

    // Track role members
    let members_key = AccessControlKey::RoleMembers(role);
    let mut members: Vec<Address> = env
        .storage()
        .instance()
        .get(&members_key)
        .unwrap_or_else(|| Vec::new(&env));

    if !members.contains(&user) {
        members.push_back(user);
        env.storage().instance().set(&members_key, &members);
    }
}

pub fn revoke_role(env: Env, admin: Address, user: Address) {
    admin.require_auth();

    let stored_admin: Address = env
        .storage()
        .instance()
        .get(&AccessControlKey::Admin)
        .unwrap_or_else(|| panic!("not initialized"));

    if admin != stored_admin {
        panic!("only admin can revoke roles");
    }

    let key = AccessControlKey::Role(user.clone());

    if let Some(role_data) = env.storage().instance().get::<_, RoleData>(&key) {
        // Remove from role members list
        let members_key = AccessControlKey::RoleMembers(role_data.role);
        if let Some(members) = env
            .storage()
            .instance()
            .get::<_, Vec<Address>>(&members_key)
        {
            let mut new_members = Vec::new(&env);
            for member in members.iter() {
                if member != user {
                    new_members.push_back(member);
                }
            }
            env.storage().instance().set(&members_key, &new_members);
        }

        env.storage().instance().remove(&key);
    }
}

pub fn has_role(env: Env, user: Address, role: Role) -> bool {
    let key = AccessControlKey::Role(user);
    env.storage()
        .instance()
        .get::<_, RoleData>(&key)
        .map(|r| r.role == role)
        .unwrap_or(false)
}

pub fn get_role(env: Env, user: Address) -> Option<RoleData> {
    let key = AccessControlKey::Role(user);
    env.storage().instance().get(&key)
}

pub fn require_role(env: Env, user: Address, role: Role) {
    if !has_role(env.clone(), user, role) {
        panic!("access denied: required role not found");
    }
}
