use serde::Serialize;

#[derive(Debug)]
pub enum EmailTemplates {
    EmailVerify,
    PasswordReset,
    PasswordChanged,
}

impl std::fmt::Display for EmailTemplates {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<EmailTemplates> for String {
    fn from(value: EmailTemplates) -> Self {
        value.to_string()
    }
}

#[derive(Serialize)]
pub struct EmailVerifyData {
    pub verification_link: String,
    pub code: String,
    pub expire_in_hours: i64,
}

#[derive(Serialize)]
pub struct PasswordResetData {
    pub reset_link: String,
    pub code: String,
    pub expire_in_hours: i64,
}

#[derive(Serialize)]
pub struct PasswordChangedData {
    pub email: String,
}
