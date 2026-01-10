//! Management API for ACP Server
//!
//! Provides HTTP endpoints for:
//! - Server status
//! - Plugin management
//! - Credential management
//! - Token management
//! - Activity monitoring

use acp_lib::AgentToken;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{
    async_trait,
    body::Bytes,
    extract::{FromRequestParts, Path, State},
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// API server state
#[derive(Clone)]
pub struct ApiState {
    /// Server start time for uptime calculation
    pub start_time: std::time::Instant,
    /// Proxy port
    pub proxy_port: u16,
    /// API port
    pub api_port: u16,
    /// Password hash (Argon2)
    pub password_hash: Arc<RwLock<Option<String>>>,
    /// Stored agent tokens
    pub tokens: Arc<RwLock<HashMap<String, AgentToken>>>,
    /// Recent activity log
    pub activity: Arc<RwLock<Vec<ActivityEntry>>>,
}

impl ApiState {
    pub fn new(proxy_port: u16, api_port: u16) -> Self {
        Self {
            start_time: std::time::Instant::now(),
            proxy_port,
            api_port,
            password_hash: Arc::new(RwLock::new(None)),
            tokens: Arc::new(RwLock::new(HashMap::new())),
            activity: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn set_password_hash(&self, hash: String) {
        *self.password_hash.write().await = Some(hash);
    }
}

/// Activity log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEntry {
    pub timestamp: DateTime<Utc>,
    pub method: String,
    pub url: String,
    pub agent_id: Option<String>,
    pub status: u16,
}

/// Status response
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct StatusResponse {
    pub version: String,
    pub uptime_seconds: u64,
    pub proxy_port: u16,
    pub api_port: u16,
}

/// Request body containing password_hash for authentication
#[derive(Debug, Deserialize, Clone)]
pub struct AuthenticatedRequest<T> {
    /// SHA512 hash of password (hex encoded)
    pub password_hash: String,
    #[serde(flatten)]
    pub data: T,
}

/// Extractor that validates authentication
pub struct Authenticated<T>(pub T);

#[async_trait]
impl<T> FromRequestParts<ApiState> for Authenticated<T>
where
    T: for<'de> Deserialize<'de> + Send,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        _parts: &mut Parts,
        _state: &ApiState,
    ) -> Result<Self, Self::Rejection> {
        // For now, this is a placeholder - actual auth will be done in handlers
        // that have access to the request body
        Err((
            StatusCode::UNAUTHORIZED,
            "Use request body for authentication".to_string(),
        ))
    }
}

/// Helper function to verify authentication from request body
async fn verify_auth<T>(
    state: &ApiState,
    body: &[u8],
) -> Result<T, (StatusCode, String)>
where
    T: for<'de> Deserialize<'de>,
{
    // Parse as authenticated request
    let auth_req: AuthenticatedRequest<T> =
        serde_json::from_slice(body).map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("Invalid JSON: {}", e),
            )
        })?;

    // Verify password hash
    let stored_hash = state.password_hash.read().await;
    if let Some(ref hash_str) = *stored_hash {
        // Hash the provided SHA512 hash with Argon2 (stored hash is Argon2 of SHA512)
        let parsed_hash = PasswordHash::new(hash_str).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Invalid stored hash: {}", e),
            )
        })?;

        // The client sends SHA512(password), we verify Argon2(SHA512(password))
        Argon2::default()
            .verify_password(auth_req.password_hash.as_bytes(), &parsed_hash)
            .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid credentials".to_string()))?;

        Ok(auth_req.data)
    } else {
        Err((
            StatusCode::UNAUTHORIZED,
            "Server not initialized".to_string(),
        ))
    }
}

/// Plugin list response
#[derive(Debug, Serialize)]
pub struct PluginsResponse {
    pub plugins: Vec<String>,
}

/// Token creation request
#[derive(Debug, Deserialize)]
pub struct CreateTokenRequest {
    pub name: String,
}

/// Token response (includes full token only on creation)
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    pub id: String,
    pub name: String,
    pub prefix: String,
    pub token: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<AgentToken> for TokenResponse {
    fn from(token: AgentToken) -> Self {
        Self {
            id: token.id.clone(),
            name: token.name.clone(),
            prefix: token.prefix.clone(),
            token: None, // Don't expose token by default
            created_at: token.created_at,
        }
    }
}

/// Tokens list response
#[derive(Debug, Serialize)]
pub struct TokensResponse {
    pub tokens: Vec<TokenResponse>,
}

/// Credential set request
#[derive(Debug, Deserialize)]
pub struct SetCredentialRequest {
    pub value: String,
}

/// Activity response
#[derive(Debug, Serialize)]
pub struct ActivityResponse {
    pub entries: Vec<ActivityEntry>,
}

/// Init request
#[derive(Debug, Deserialize)]
pub struct InitRequest {
    pub ca_path: Option<String>,
}

/// Init response
#[derive(Debug, Serialize, Deserialize)]
pub struct InitResponse {
    pub ca_path: String,
}

/// API error response
#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(self)).into_response()
    }
}

/// Create the API router
pub fn create_router(state: ApiState) -> Router {
    Router::new()
        .route("/status", get(get_status))
        .route("/init", post(init))
        .route("/plugins", get(get_plugins).post(post_plugins))
        .route("/tokens", get(list_tokens).post(post_list_tokens))
        .route("/tokens/create", post(create_token))
        .route("/tokens/:id", delete(delete_token))
        .route(
            "/credentials/:plugin/:key",
            post(set_credential).delete(delete_credential),
        )
        .route("/activity", get(get_activity).post(post_activity))
        .with_state(state)
}

/// GET /status - Server status (no auth required)
async fn get_status(State(state): State<ApiState>) -> Json<StatusResponse> {
    let uptime = state.start_time.elapsed().as_secs();

    Json(StatusResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: uptime,
        proxy_port: state.proxy_port,
        api_port: state.api_port,
    })
}

/// POST /init - Initialize server with password and CA (no auth required initially)
async fn init(
    State(state): State<ApiState>,
    body: Bytes,
) -> Result<Json<InitResponse>, (StatusCode, String)> {
    use acp_lib::storage::create_store;
    use acp_lib::tls::CertificateAuthority;
    use argon2::password_hash::{rand_core::OsRng, SaltString};
    use argon2::{Argon2, PasswordHasher};

    // Check if already initialized
    {
        let hash = state.password_hash.read().await;
        if hash.is_some() {
            return Err((StatusCode::CONFLICT, "Server already initialized".to_string()));
        }
    }

    // Parse request
    let req: AuthenticatedRequest<InitRequest> = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid JSON: {}", e)))?;

    // Hash the password_hash with Argon2 (password_hash is already SHA512 from client)
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(req.password_hash.as_bytes(), &salt)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to hash password: {}", e)))?
        .to_string();

    // Store password hash
    state.set_password_hash(password_hash).await;

    // Generate CA
    let ca = CertificateAuthority::generate()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to generate CA: {}", e)))?;

    // Store CA private key in SecretStore
    let store = create_store(None)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create store: {}", e)))?;

    store
        .set("ca:private_key", ca.ca_key_pem().as_bytes())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to store CA key: {}", e)))?;

    // Determine CA certificate path
    let ca_path = if let Some(path) = req.data.ca_path {
        path
    } else {
        // Default to ~/.config/acp/ca.crt
        let home = std::env::var("HOME")
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "HOME env var not set".to_string()))?;
        format!("{}/.config/acp/ca.crt", home)
    };

    // Export CA certificate to filesystem
    let ca_dir = std::path::Path::new(&ca_path).parent()
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Invalid CA path".to_string()))?;

    std::fs::create_dir_all(ca_dir)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create CA directory: {}", e)))?;

    std::fs::write(&ca_path, ca.ca_cert_pem())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to write CA cert: {}", e)))?;

    Ok(Json(InitResponse { ca_path }))
}

/// GET /plugins - List installed plugins (requires auth)
async fn get_plugins(
    State(state): State<ApiState>,
    body: Bytes,
) -> Result<Json<PluginsResponse>, (StatusCode, String)> {
    verify_auth::<serde_json::Value>(&state, &body).await?;

    // TODO: Load from storage in future implementation
    Ok(Json(PluginsResponse {
        plugins: vec![],
    }))
}

/// POST /plugins - List installed plugins (requires auth, same as GET)
async fn post_plugins(
    State(state): State<ApiState>,
    body: Bytes,
) -> Result<Json<PluginsResponse>, (StatusCode, String)> {
    get_plugins(State(state), body).await
}

/// GET /tokens - List agent tokens (requires auth)
async fn list_tokens(
    State(state): State<ApiState>,
    body: Bytes,
) -> Result<Json<TokensResponse>, (StatusCode, String)> {
    verify_auth::<serde_json::Value>(&state, &body).await?;

    let tokens = state.tokens.read().await;
    let token_list: Vec<TokenResponse> = tokens.values().map(|t| t.clone().into()).collect();

    Ok(Json(TokensResponse { tokens: token_list }))
}

/// POST /tokens - List agent tokens (requires auth, same as GET)
async fn post_list_tokens(
    State(state): State<ApiState>,
    body: Bytes,
) -> Result<Json<TokensResponse>, (StatusCode, String)> {
    list_tokens(State(state), body).await
}

/// POST /tokens/create - Create new agent token (requires auth)
async fn create_token(
    State(state): State<ApiState>,
    body: Bytes,
) -> Result<Json<TokenResponse>, (StatusCode, String)> {
    let req: CreateTokenRequest = verify_auth(&state, &body).await?;

    let token = AgentToken::new(&req.name);
    let token_value = token.token.clone();

    // Store token
    let mut tokens = state.tokens.write().await;
    tokens.insert(token.id.clone(), token.clone());

    // Return with full token (only time it's revealed)
    Ok(Json(TokenResponse {
        id: token.id,
        name: token.name,
        prefix: token.prefix,
        token: Some(token_value),
        created_at: token.created_at,
    }))
}

/// DELETE /tokens/:id - Revoke agent token (requires auth)
async fn delete_token(
    State(state): State<ApiState>,
    Path(id): Path<String>,
    body: Bytes,
) -> Result<StatusCode, (StatusCode, String)> {
    verify_auth::<serde_json::Value>(&state, &body).await?;

    let mut tokens = state.tokens.write().await;
    if tokens.remove(&id).is_some() {
        Ok(StatusCode::OK)
    } else {
        Ok(StatusCode::NOT_FOUND)
    }
}

/// POST /credentials/:plugin/:key - Set credential (requires auth)
async fn set_credential(
    State(state): State<ApiState>,
    Path((plugin, key)): Path<(String, String)>,
    body: Bytes,
) -> Result<StatusCode, (StatusCode, String)> {
    let _req: SetCredentialRequest = verify_auth(&state, &body).await?;

    // TODO: Store in SecretStore in future implementation
    tracing::info!("Setting credential {}:{}", plugin, key);
    Ok(StatusCode::OK)
}

/// DELETE /credentials/:plugin/:key - Delete credential (requires auth)
async fn delete_credential(
    State(state): State<ApiState>,
    Path((plugin, key)): Path<(String, String)>,
    body: Bytes,
) -> Result<StatusCode, (StatusCode, String)> {
    verify_auth::<serde_json::Value>(&state, &body).await?;

    // TODO: Delete from SecretStore in future implementation
    tracing::info!("Deleting credential {}:{}", plugin, key);
    Ok(StatusCode::OK)
}

/// GET /activity - Get recent activity (requires auth)
async fn get_activity(
    State(state): State<ApiState>,
    body: Bytes,
) -> Result<Json<ActivityResponse>, (StatusCode, String)> {
    verify_auth::<serde_json::Value>(&state, &body).await?;

    let activity = state.activity.read().await;
    Ok(Json(ActivityResponse {
        entries: activity.clone(),
    }))
}

/// POST /activity - Get recent activity (requires auth, same as GET)
async fn post_activity(
    State(state): State<ApiState>,
    body: Bytes,
) -> Result<Json<ActivityResponse>, (StatusCode, String)> {
    get_activity(State(state), body).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt; // for `oneshot`

    #[tokio::test]
    async fn test_get_status_without_auth() {
        let state = ApiState::new(9443, 9080);
        let app = create_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let status: StatusResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(status.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(status.proxy_port, 9443);
        assert_eq!(status.api_port, 9080);
        assert!(status.uptime_seconds < 10); // Should be very recent
    }

    #[tokio::test]
    async fn test_status_response_serialization() {
        let status = StatusResponse {
            version: "0.1.0".to_string(),
            uptime_seconds: 42,
            proxy_port: 9443,
            api_port: 9080,
        };

        let json = serde_json::to_string(&status).unwrap();
        let deserialized: StatusResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(status, deserialized);
    }

    #[tokio::test]
    async fn test_post_plugins_endpoint() {
        use argon2::password_hash::{rand_core::OsRng, SaltString};
        use argon2::{Argon2, PasswordHasher};

        let state = ApiState::new(9443, 9080);

        // Set up password hash
        let password = "testpass123";
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(password.as_bytes(), &salt).unwrap().to_string();
        state.set_password_hash(password_hash).await;

        let app = create_router(state);

        // Create auth request body
        let body = serde_json::json!({
            "password_hash": password
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/plugins")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_post_tokens_list_endpoint() {
        use argon2::password_hash::{rand_core::OsRng, SaltString};
        use argon2::{Argon2, PasswordHasher};

        let state = ApiState::new(9443, 9080);

        // Set up password hash
        let password = "testpass123";
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(password.as_bytes(), &salt).unwrap().to_string();
        state.set_password_hash(password_hash).await;

        let app = create_router(state);

        // Create auth request body
        let body = serde_json::json!({
            "password_hash": password
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/tokens")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_post_tokens_create_endpoint() {
        use argon2::password_hash::{rand_core::OsRng, SaltString};
        use argon2::{Argon2, PasswordHasher};

        let state = ApiState::new(9443, 9080);

        // Set up password hash
        let password = "testpass123";
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(password.as_bytes(), &salt).unwrap().to_string();
        state.set_password_hash(password_hash).await;

        let app = create_router(state);

        // Create auth request body with name
        let body = serde_json::json!({
            "password_hash": password,
            "name": "test-token"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/tokens/create")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let token_response: TokenResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(token_response.name, "test-token");
        assert!(token_response.token.is_some());
    }

    #[tokio::test]
    async fn test_post_activity_endpoint() {
        use argon2::password_hash::{rand_core::OsRng, SaltString};
        use argon2::{Argon2, PasswordHasher};

        let state = ApiState::new(9443, 9080);

        // Set up password hash
        let password = "testpass123";
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(password.as_bytes(), &salt).unwrap().to_string();
        state.set_password_hash(password_hash).await;

        let app = create_router(state);

        // Create auth request body
        let body = serde_json::json!({
            "password_hash": password
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/activity")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_init_endpoint() {
        let state = ApiState::new(9443, 9080);
        let app = create_router(state.clone());

        let password = "testpass123";

        // Create init request body
        let body = serde_json::json!({
            "password_hash": password
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/init")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let init_response: InitResponse = serde_json::from_slice(&body_bytes).unwrap();

        // Should return a CA path
        assert!(!init_response.ca_path.is_empty());

        // Password hash should be set in state
        let hash = state.password_hash.read().await;
        assert!(hash.is_some());
    }
}
