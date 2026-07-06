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

#[derive(Clone, Serialize, Deserialize)]
struct Session {
    user_id: String,
    email: String,
    role: String,
    expires_at: u64,
}

fn sessions_file() -> std::path::PathBuf {
    crate::store::base_dir().join("sessions.json")
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

// Session table, persisted to disk so a login survives app restarts (the
// desktop app restarts the embedded server each launch).
fn sessions() -> &'static Mutex<HashMap<String, Session>> {
    use std::sync::OnceLock;
    static S: OnceLock<Mutex<HashMap<String, Session>>> = OnceLock::new();
    S.get_or_init(|| {
        let loaded: HashMap<String, Session> = std::fs::read_to_string(sessions_file())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        // Drop already-expired sessions on load.
        let n = now();
        let live: HashMap<String, Session> = loaded.into_iter().filter(|(_, s)| s.expires_at > n).collect();
        Mutex::new(live)
    })
}

fn persist_sessions(map: &HashMap<String, Session>) {
    let _ = crate::store::write_json(&sessions_file(), map);
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
/// Valid roles, most→least privileged. admin manages users & everything;
/// analyst runs analyses & edits; viewer is read-only.
pub const ROLES: [&str; 3] = ["admin", "analyst", "viewer"];
fn valid_role(r: &str) -> bool { ROLES.contains(&r) }

fn new_user(email: &str, display_name: &str, password: &str, role: &str) -> Result<User> {
    Ok(User {
        id: format!("usr-{}", uuid::Uuid::new_v4().simple()),
        email: email.to_string(),
        display_name: if display_name.trim().is_empty() { email.to_string() } else { display_name.trim().to_string() },
        role: role.to_string(),
        password_hash: hash_password(password)?,
        created_at: now(),
        failed_attempts: 0,
        locked_until: 0,
    })
}

/// Bootstrap registration: ONLY allowed for the very first account (which becomes
/// the admin). After that, sign-up is closed — an admin must add users.
pub fn register(email: &str, display_name: &str, password: &str) -> Result<AuthResult> {
    let email = normalize_email(email);
    if !email.contains('@') || !email.contains('.') {
        return Err(anyhow!("enter a valid email address"));
    }
    let mut db = load_db();
    if !db.users.is_empty() {
        return Err(anyhow!("sign-up is closed — ask an administrator to create your account"));
    }
    check_password_policy(password)?;
    let user = new_user(&email, display_name, password, "admin")?;
    db.users.push(user.clone());
    save_db(&db)?;
    Ok(issue_session(&user))
}

/// Public view of a user for admin listings.
#[derive(Debug, Clone, Serialize)]
pub struct UserRow {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub created_at: u64,
    pub locked: bool,
}

pub fn list_users() -> Vec<UserRow> {
    let n = now();
    load_db().users.iter().map(|u| UserRow {
        id: u.id.clone(), email: u.email.clone(), display_name: u.display_name.clone(),
        role: u.role.clone(), created_at: u.created_at, locked: u.locked_until > n,
    }).collect()
}

/// Admin-only: create a user with a role (default analyst).
pub fn admin_create_user(actor: &AuthUser, email: &str, display_name: &str, password: &str, role: &str) -> Result<UserRow> {
    if actor.role != "admin" {
        return Err(anyhow!("only an administrator can add users"));
    }
    let role = if valid_role(role) { role } else { "analyst" };
    let email = normalize_email(email);
    if !email.contains('@') || !email.contains('.') {
        return Err(anyhow!("enter a valid email address"));
    }
    check_password_policy(password)?;
    let mut db = load_db();
    if db.users.iter().any(|u| u.email == email) {
        return Err(anyhow!("an account with that email already exists"));
    }
    let u = new_user(&email, display_name, password, role)?;
    db.users.push(u.clone());
    save_db(&db)?;
    Ok(UserRow { id: u.id, email: u.email, display_name: u.display_name, role: u.role, created_at: u.created_at, locked: false })
}

/// Admin-only: change a user's role. Cannot demote the last remaining admin.
pub fn admin_set_role(actor: &AuthUser, user_id: &str, role: &str) -> Result<()> {
    if actor.role != "admin" {
        return Err(anyhow!("only an administrator can change roles"));
    }
    if !valid_role(role) {
        return Err(anyhow!("invalid role"));
    }
    let mut db = load_db();
    let admins = db.users.iter().filter(|u| u.role == "admin").count();
    let target = db.users.iter_mut().find(|u| u.id == user_id).ok_or_else(|| anyhow!("user not found"))?;
    if target.role == "admin" && role != "admin" && admins <= 1 {
        return Err(anyhow!("cannot demote the last administrator"));
    }
    target.role = role.to_string();
    save_db(&db)
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
    {
        let mut map = sessions().lock().unwrap();
        map.insert(token.clone(), sess);
        persist_sessions(&map);
    }
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
        persist_sessions(&map);
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
    let mut map = sessions().lock().unwrap();
    map.remove(token);
    persist_sessions(&map);
}

/// Whether any account exists yet (drives register-first UX).
pub fn has_accounts() -> bool {
    !load_db().users.is_empty()
}
