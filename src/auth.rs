//! Authentication, shared by the json api and the html pages.
//!
//! A request proves who it is with either the `x-api-key` header (api clients)
//! or the `pimbo_key` cookie (browsers, set by `POST /login`). Both are looked
//! up in the `accesskeys` table: a key *is* the session.
//!
//! Logging in with an email and password mints one of those keys, so passwords
//! are checked in exactly one place, here, and never travel further than the
//! login form.

use crate::{AppState, db};
use argon2::{
    Argon2, PasswordHasher, PasswordVerifier,
    password_hash::{PasswordHash, SaltString, rand_core::OsRng, rand_core::RngCore},
};
use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
    response::Redirect,
};
use std::sync::Arc;

pub const COOKIE_NAME: &str = "pimbo_key";
const COOKIE_MAX_AGE: i64 = 60 * 60 * 24 * 30;

/// An authenticated request. Handlers get the owning user's id and scope every
/// query to it.
pub struct Auth {
    pub user_id: i32,
}

/// Same as [`Auth`], but a browser that is not logged in is sent to the login
/// page instead of getting a bare 401.
pub struct WebAuth {
    pub user_id: i32,
}

/// Whoever this is, if anyone. Used by pages that render for both.
pub struct MaybeAuth(pub Option<i32>);

/// The key a request carried, valid or not. Logout needs it to throw the
/// session away rather than merely forget the cookie.
pub struct PresentedKey(pub Option<String>);

fn cookie(parts: &Parts, name: &str) -> Option<String> {
    parts
        .headers
        .get_all(axum::http::header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|header| header.split(';'))
        .filter_map(|pair| pair.split_once('='))
        .find(|(k, _)| k.trim() == name)
        .map(|(_, v)| v.trim().to_string())
}

/// The key a request presents, header first, cookie second.
fn presented_key(parts: &Parts) -> Option<String> {
    let header = parts
        .headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_string());
    header.or_else(|| cookie(parts, COOKIE_NAME))
}

async fn authenticate(parts: &Parts, state: &Arc<AppState>) -> Option<i32> {
    let key = presented_key(parts)?;
    if !plausible_key(&key) {
        return None;
    }
    db::user_for_key(&state.db, &key).await.ok().flatten()
}

impl FromRequestParts<Arc<AppState>> for Auth {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        match authenticate(parts, state).await {
            Some(user_id) => Ok(Auth { user_id }),
            None => Err(StatusCode::UNAUTHORIZED),
        }
    }
}

impl FromRequestParts<Arc<AppState>> for WebAuth {
    type Rejection = Redirect;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        match authenticate(parts, state).await {
            Some(user_id) => Ok(WebAuth { user_id }),
            None => Err(Redirect::to("/login")),
        }
    }
}

impl FromRequestParts<Arc<AppState>> for MaybeAuth {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        Ok(MaybeAuth(authenticate(parts, state).await))
    }
}

impl FromRequestParts<Arc<AppState>> for PresentedKey {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        _: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        Ok(PresentedKey(presented_key(parts)))
    }
}

/// Cheap sanity check before we go near the database, and it keeps anything
/// that could not survive a `Set-Cookie` round trip out of the cookie.
pub fn plausible_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 256
        && key
            .chars()
            .all(|c| c.is_ascii_graphic() && c != ';' && c != ',' && c != '"' && c != '\\')
}

pub fn login_cookie(key: &str, secure: bool) -> String {
    format!(
        "{COOKIE_NAME}={key}; Path=/; HttpOnly; SameSite=Lax; Max-Age={COOKIE_MAX_AGE}{}",
        if secure { "; Secure" } else { "" }
    )
}

pub fn logout_cookie(secure: bool) -> String {
    format!(
        "{COOKIE_NAME}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{}",
        if secure { "; Secure" } else { "" }
    )
}

/// How long a session minted by an email login lasts.
pub const SESSION_DAYS: i64 = 30;

/// A fresh, unguessable access key. Hex so it survives a cookie untouched.
pub fn new_session_key() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Argon2id, on the blocking pool: hashing a password is deliberately slow and
/// would otherwise stall the runtime.
pub async fn hash_password(password: String) -> Option<String> {
    tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .ok()
            .map(|hash| hash.to_string())
    })
    .await
    .ok()
    .flatten()
}

/// False for anything that is not a password this hash was made from, which
/// includes the empty `passhash` the placeholder user carries.
pub async fn verify_password(password: String, hash: String) -> bool {
    tokio::task::spawn_blocking(move || match PasswordHash::new(&hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    })
    .await
    .unwrap_or(false)
}
