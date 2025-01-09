use anyhow::Context;
use regex::Regex;
use secrecy::SecretString;
use sqlx::PgPool;
use std::sync::LazyLock;
use uuid::Uuid;

use crate::app::{auth::password::verify_password_hash, error::AppError};

pub static USERNAME_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-zA-Z][a-zA-Z0-9_-]{3,32}$").unwrap());

#[tracing::instrument(name = "Password required API, validating")]
pub async fn validate_password(
    candidate_pw: SecretString,
    user_id: &Uuid,
    pool: &PgPool,
) -> Result<(), AppError> {
    let user_pw_hash = sqlx::query_scalar!(
        r#"
            select password_hash 
            from "user"
            where user_id = $1
        "#,
        user_id
    )
    .fetch_one(pool)
    .await
    .context("failed to retrieve stored credentials.")?;

    verify_password_hash(SecretString::from(user_pw_hash), candidate_pw).await
}

pub static BUSINESS_NAME_SYMBOLS: &str = r#"[&'"-.,;:()#/ ]"#;

pub static BUSINESS_NAME_EN_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r#"^[a-zA-Z0-9{}]{{3,100}}$"#,
        BUSINESS_NAME_SYMBOLS
    ))
    .unwrap()
});

pub static BUSINESS_NAME_MN_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r#"^[а-яА-ЯЁёӨөҮү0-9{}]{{3,100}}$"#,
        BUSINESS_NAME_SYMBOLS
    ))
    .unwrap()
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_username_valid_regex() {
        let valid_usernames = [
            "user_123",
            "User-Name",
            "u1234567890",
            "A-123456",
            "abc_def",
            "longuser_name",
            "test123",
        ];

        for &username in &valid_usernames {
            assert!(
                USERNAME_REGEX.is_match(username),
                "{} should be valid",
                username
            );
        }
    }

    #[test]
    fn test_username_invalid_regex() {
        let invalid_usernames = [
            "123user",
            "user@",
            "u",
            "a",
            "user12345678901234567890123456789012345678901",
            "_user",
            "-username",
            "bleach_nevermind_inutero_incesticide",
        ];

        for &username in &invalid_usernames {
            assert!(
                !USERNAME_REGEX.is_match(username),
                "{} should be invalid",
                username
            );
        }
    }

    #[test]
    fn test_business_name_en_valid() {
        let valid_strings = [
            "abc",
            "hello world",
            "1234567890",
            "valid-string-123",
            "John Doe (123)",
            "a'&\"-,;()# ",
            "123&'()-.,; #",
            "Tech Solutions 123",
            "Acme Corporation",
            "Green Valley Co.",
            "John's Bakery & Cafe",
            "Tech-Giant Inc.",
            "Best Products Ltd.",
            "Global Innovations 2025",
            "Smith & Wesson LLC",
            "Quick Fix Auto Repair",
            "Fashion World #1",
            "Creative Designs, Inc.",
            "Sakura Cafe",
        ];

        for &test_str in &valid_strings {
            assert!(
                BUSINESS_NAME_EN_REGEX.is_match(test_str),
                "{} should be valid",
                test_str
            );
        }
    }

    #[test]
    fn test_business_name_en_invalid() {
        let invalid_strings = [
            "a",                        
            "ab",                       
            "This is a very long string that exceeds one hundred characters in length. This is a test to check if the regex correctly handles long strings.", 
            "invalid@string",           
            "invalid|string~",          
            "1234567890#)(*&^",         
            "!not allowed",             
            "hi!",                      
            "#$%^&*",                   
            "Сакура Кафе"
        ];

        for &test_str in &invalid_strings {
            assert!(
                !BUSINESS_NAME_EN_REGEX.is_match(test_str),
                "{} should be invalid",
                test_str
            );
        }
    }

    #[test]
    fn test_business_name_mn_valid() {
        let valid_strings = [
            "Сакура Кафе",
            "Пийк Ойл",
            "Технологии и решения",
            "СМИ: \"Новости\"",
            "Интернет-магазин 24/7",
            "Группировка \"Сибирь\" & \"Үни\" проект",
            "Өргөө кино театр",
        ];

        for &test_str in &valid_strings {
            assert!(
                BUSINESS_NAME_MN_REGEX.is_match(test_str),
                "{} should be valid",
                test_str
            );
        }
    }
}
