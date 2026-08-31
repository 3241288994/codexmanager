use serde::Serialize;

pub const ROLE_SYSTEM_ADMIN: &str = "system_admin";
pub const ROLE_ADMIN: &str = "admin";
pub const ROLE_MEMBER: &str = "member";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcActor {
    pub role: String,
    pub user_id: Option<String>,
}

impl RpcActor {
    pub fn system_admin() -> Self {
        Self {
            role: ROLE_SYSTEM_ADMIN.to_string(),
            user_id: None,
        }
    }

    pub fn from_parts(role: Option<&str>, user_id: Option<&str>) -> Self {
        let normalized_role = normalize_role(role);
        Self {
            role: normalized_role.to_string(),
            user_id: user_id
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string),
        }
    }

    pub fn is_admin(&self) -> bool {
        matches!(self.role.as_str(), ROLE_SYSTEM_ADMIN | ROLE_ADMIN)
    }

    pub fn is_member(&self) -> bool {
        self.role == ROLE_MEMBER
    }

    pub fn permissions(&self) -> Vec<&'static str> {
        if self.is_admin() {
            return vec!["system:admin"];
        }
        vec![
            "apikey:self",
            "requestlog:self",
            "models:read",
            "profile:self",
        ]
    }
}

fn normalize_role(role: Option<&str>) -> &'static str {
    match role
        .map(str::trim)
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        ROLE_ADMIN => ROLE_ADMIN,
        ROLE_MEMBER => ROLE_MEMBER,
        ROLE_SYSTEM_ADMIN => ROLE_SYSTEM_ADMIN,
        _ => ROLE_SYSTEM_ADMIN,
    }
}
