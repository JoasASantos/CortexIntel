//! Authentication: local accounts with Argon2id password hashing, session
//! tokens, a password-strength policy and failed-attempt lockout.
//!
//! This is a single-node operator tool, so accounts persist to
//! `~/.cortexintel/users.json` (0600) and sessions live in memory for the life
//! of the server process. It is not a multi-tenant IdP — it gates access to the
//! local workspace and keeps credentials hashed at rest.

use crate::store;
use anyhow::{anyhow, Result};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_FAILED: u32 = 5;
const LOCKOUT_SECS: u64 = 15 * 60;
const SESSION_TTL_SECS: u64 = 12 * 60 * 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub password_hash: String,
    pub created_at: u64,
    #[serde(default)]
    pub failed_attempts: u32,
    #[serde(default)]
    pub locked_until: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct UserDb {
    users: Vec<User>,
}

#[derive(Clone)]
struct Session {
    user_id: String,
    email: String,
    role: String,
    expires_at: u64,
}

/// Public, non-secret view of the signed-in user.
#[derive(Debug, Clone, Serialize)]
pub struct AuthUser {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub role: String,
}

/// Result of a successful login/register.
#[derive(Debug, Clone, Serialize)]
pub struct AuthResult {
    pub token: String,
    pub user: AuthUser,
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

// In-memory session table (process-lifetime).
fn sessions() -> &'static Mutex<HashMap<String, Session>> {
    use std::sync::OnceLock;
    static S: OnceLock<Mutex<HashMap<String, Session>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(HashMap::new()))
}

fn load_db() -> UserDb {
    store::read_json_or_default(&store::users_file())
}
fn save_db(db: &UserDb) -> Result<()> {
    store::write_json(&store::users_file(), db)
}

/// Password policy: >=10 chars, with upper, lower and a digit. Returns the first
/// violation as an error message.
pub fn check_password_policy(pw: &str) -> Result<()> {
    if pw.chars().count() < 10 {
        return Err(anyhow!("password must be at least 10 characters"));
    }
    if !pw.chars().any(|c| c.is_ascii_uppercase()) {
        return Err(anyhow!("password must contain an uppercase letter"));
    }
    if !pw.chars().any(|c| c.is_ascii_lowercase()) {
        return Err(anyhow!("password must contain a lowercase letter"));
    }
    if !pw.chars().any(|c| c.is_ascii_digit()) {
        return Err(anyhow!("password must contain a digit"));
    }
    Ok(())
}

fn hash_password(pw: &str) -> Result<String> {
    let salt = SaltString::generate(&mut rand::rngs::OsRng);
    let hash = Argon2::default()
        .hash_password(pw.as_bytes(), &salt)
        .map_err(|e| anyhow!("hashing failed: {e}"))?;
    Ok(hash.to_string())
}

fn verify_password(pw: &str, phc: &str) -> bool {
    match PasswordHash::new(phc) {
        Ok(parsed) => Argon2::default().verify_password(pw.as_bytes(), &parsed).is_ok(),
        Err(_) => false,
    }
}

fn random_token() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

/// Register a new account. First-ever account becomes the `admin`.
pub fn register(email: &str, display_name: &str, password: &str) -> Result<AuthResult> {
    let email = normalize_email(email);
    if !email.contains('@') || !email.contains('.') {
        return Err(anyhow!("enter a valid email address"));
    }
    check_password_policy(password)?;
    let mut db = load_db();
    if db.users.iter().any(|u| u.email == email) {
        return Err(anyhow!("an account with that email already exists"));
    }
    let role = if db.users.is_empty() { "admin" } else { "analyst" };
    let user = User {
        id: format!("usr-{}", uuid::Uuid::new_v4().simple()),
        email: email.clone(),
        display_name: if display_name.trim().is_empty() { email.clone() } else { display_name.trim().to_string() },
        role: role.to_string(),
        password_hash: hash_password(password)?,
        created_at: now(),
        failed_attempts: 0,
        locked_until: 0,
    };
    db.users.push(user.clone());
    save_db(&db)?;
    Ok(issue_session(&user))
}

/// Log in with email + password, enforcing lockout on repeated failures.
pub fn login(email: &str, password: &str) -> Result<AuthResult> {
    let email = normalize_email(email);
    let mut db = load_db();
    let idx = db
        .users
        .iter()
        .position(|u| u.email == email)
        .ok_or_else(|| anyhow!("invalid email or password"))?;

    let t = now();
    if db.users[idx].locked_until > t {
        let mins = (db.users[idx].locked_until - t + 59) / 60;
        return Err(anyhow!("account locked; try again in {mins} minute(s)"));
    }

    if verify_password(password, &db.users[idx].password_hash) {
        db.users[idx].failed_attempts = 0;
        db.users[idx].locked_until = 0;
        let user = db.users[idx].clone();
        save_db(&db)?;
        Ok(issue_session(&user))
    } else {
        db.users[idx].failed_attempts += 1;
        if db.users[idx].failed_attempts >= MAX_FAILED {
            db.users[idx].locked_until = t + LOCKOUT_SECS;
            db.users[idx].failed_attempts = 0;
        }
        save_db(&db)?;
        Err(anyhow!("invalid email or password"))
    }
}

fn issue_session(user: &User) -> AuthResult {
    let token = random_token();
    let sess = Session {
        user_id: user.id.clone(),
        email: user.email.clone(),
        role: user.role.clone(),
        expires_at: now() + SESSION_TTL_SECS,
    };
    sessions().lock().unwrap().insert(token.clone(), sess);
    AuthResult {
        token,
        user: AuthUser {
            id: user.id.clone(),
            email: user.email.clone(),
            display_name: user.display_name.clone(),
            role: user.role.clone(),
        },
    }
}

/// Validate a bearer token, returning the user if the session is live.
pub fn validate(token: &str) -> Option<AuthUser> {
    let mut map = sessions().lock().unwrap();
    let s = map.get(token)?.clone();
    if s.expires_at < now() {
        map.remove(token);
        return None;
    }
    let db = load_db();
    db.users.iter().find(|u| u.id == s.user_id).map(|u| AuthUser {
        id: u.id.clone(),
        email: s.email.clone(),
        display_name: u.display_name.clone(),
        role: s.role.clone(),
    })
}

pub fn logout(token: &str) {
    sessions().lock().unwrap().remove(token);
}

/// Whether any account exists yet (drives register-first UX).
pub fn has_accounts() -> bool {
    !load_db().users.is_empty()
}
