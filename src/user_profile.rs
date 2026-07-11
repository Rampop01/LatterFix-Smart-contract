use soroban_sdk::{contractimpl, contracttype, Address, Env, String};
use crate::{DataKey, TaskManagerContract};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserProfile {
    pub address: Address,
    pub username: String,
    pub reputation: u32,
    pub completed_tasks: u32,
    pub joined_at: u64,
    pub bio: String,
}

#[contractimpl]
impl TaskManagerContract {
    pub fn create_profile(env: Env, user: Address, username: String, bio: String) {
        user.require_auth();

        let key = DataKey::UserProfile(user.clone());
        if env.storage().persistent().has(&key) {
            panic!("profile already exists");
        }

        let profile = UserProfile {
            address: user.clone(),
            username,
            reputation: 100,
            completed_tasks: 0,
            joined_at: env.ledger().timestamp(),
            bio,
        };

        env.storage().persistent().set(&key, &profile);
    }

    pub fn update_bio(env: Env, user: Address, new_bio: String) {
        user.require_auth();

        let key = DataKey::UserProfile(user.clone());
        let mut profile: UserProfile = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic!("profile not found"));

        profile.bio = new_bio;
        env.storage().persistent().set(&key, &profile);
    }

    pub fn reward_contribution(env: Env, admin: Address, user: Address, points: u32) {
        admin.require_auth();

        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if admin != stored_admin {
            panic!("not admin");
        }

        let key = DataKey::UserProfile(user.clone());
        let mut profile: UserProfile = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic!("profile not found"));

        profile.reputation += points;
        profile.completed_tasks += 1;

        env.storage().persistent().set(&key, &profile);
    }

    pub fn get_profile(env: Env, user: Address) -> Option<UserProfile> {
        let key = DataKey::UserProfile(user);
        env.storage().persistent().get(&key)
    }
}
