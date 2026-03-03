use apihop_core::models::DeploymentMode;

pub struct Config {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub encryption_key: [u8; 32],
    pub mode: DeploymentMode,
    pub server_name: String,
    pub jwt_secret: Option<String>,
    pub registration_enabled: bool,
    pub session_duration_secs: u64,
    pub refresh_duration_secs: u64,
}

impl Config {
    pub fn from_env() -> Self {
        let encryption_key = match std::env::var("APIHOP_SECRET_KEY") {
            Ok(hex_str) => {
                let bytes = hex::decode(hex_str.trim())
                    .expect("APIHOP_SECRET_KEY must be valid hex (64 hex chars = 32 bytes)");
                let arr: [u8; 32] = bytes
                    .try_into()
                    .expect("APIHOP_SECRET_KEY must be exactly 32 bytes (64 hex chars)");
                arr
            }
            Err(_) => {
                let mut key = [0u8; 32];
                getrandom::getrandom(&mut key).expect("Failed to generate random encryption key");
                eprintln!("Warning: APIHOP_SECRET_KEY not set, using random key. Auth secrets will not persist across restarts.");
                key
            }
        };

        let mode: DeploymentMode = std::env::var("APIHOP_MODE")
            .unwrap_or_else(|_| "personal".to_string())
            .parse()
            .unwrap_or(DeploymentMode::Personal);

        let jwt_secret = std::env::var("APIHOP_JWT_SECRET").ok();

        if matches!(mode, DeploymentMode::Organization) && jwt_secret.is_none() {
            panic!("APIHOP_JWT_SECRET is required in organization mode");
        }

        Self {
            host: std::env::var("APIHOP_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("APIHOP_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3000),
            database_url: std::env::var("APIHOP_DATABASE_URL")
                .unwrap_or_else(|_| "apihop.db".to_string()),
            encryption_key,
            mode,
            server_name: std::env::var("APIHOP_SERVER_NAME")
                .unwrap_or_else(|_| "apihop".to_string()),
            jwt_secret,
            registration_enabled: std::env::var("APIHOP_REGISTRATION_ENABLED")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(true),
            session_duration_secs: std::env::var("APIHOP_SESSION_DURATION")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3600), // 1 hour
            refresh_duration_secs: std::env::var("APIHOP_REFRESH_DURATION")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(604800), // 7 days
        }
    }
}
