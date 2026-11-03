use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{FromRow, PgPool};
use thiserror::Error;
use uuid::Uuid;

pub const MAX_NAME_LENGTH: usize = 120;
pub const MAX_EMAIL_LENGTH: usize = 254;

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Franchisee {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct FranchiseeOverview {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub store_count: i64,
}

impl FranchiseeOverview {
    pub fn registered_on(&self) -> String {
        self.created_at.format("%d %b %Y").to_string()
    }
}

#[derive(Debug, Clone)]
pub struct NewFranchisee {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Error)]
pub enum CreateFranchiseeError {
    #[error("A franchisee with that email address already exists.")]
    EmailTaken,
    #[error("The franchisee could not be saved.")]
    Database(#[from] sqlx::Error),
}

pub async fn list(pool: &PgPool) -> Result<Vec<Franchisee>, sqlx::Error> {
    sqlx::query_as::<_, Franchisee>(
        "SELECT id, name, email, created_at FROM franchisees ORDER BY created_at DESC, name ASC",
    )
    .fetch_all(pool)
    .await
}

pub async fn list_overview(pool: &PgPool) -> Result<Vec<FranchiseeOverview>, sqlx::Error> {
    sqlx::query_as::<_, FranchiseeOverview>(
        r#"
        SELECT
            f.id,
            f.name,
            f.email,
            f.created_at,
            COUNT(s.id)::bigint AS store_count
        FROM franchisees f
        LEFT JOIN stores s ON s.franchisee_id = f.id
        GROUP BY f.id, f.name, f.email, f.created_at
        ORDER BY f.created_at DESC, f.name ASC
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn create(
    pool: &PgPool,
    input: NewFranchisee,
) -> Result<Franchisee, CreateFranchiseeError> {
    let result = sqlx::query_as::<_, Franchisee>(
        r#"
        INSERT INTO franchisees (id, name, email)
        VALUES ($1, $2, $3)
        RETURNING id, name, email, created_at
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(input.name)
    .bind(input.email)
    .fetch_one(pool)
    .await;

    match result {
        Ok(franchisee) => Ok(franchisee),
        Err(sqlx::Error::Database(error)) if error.code().as_deref() == Some("23505") => {
            Err(CreateFranchiseeError::EmailTaken)
        }
        Err(error) => Err(CreateFranchiseeError::Database(error)),
    }
}

pub fn validate(name: &str, email: &str) -> Result<NewFranchisee, Vec<String>> {
    let mut errors = Vec::new();

    let name = name.trim().to_owned();
    if name.is_empty() {
        errors.push("Enter the franchisee's name.".to_owned());
    } else if name.chars().count() > MAX_NAME_LENGTH {
        errors.push(format!(
            "The franchisee name must be {MAX_NAME_LENGTH} characters or fewer."
        ));
    }

    let email = email.trim().to_lowercase();
    if email.is_empty() {
        errors.push("Enter the franchisee's email address.".to_owned());
    } else if email.chars().count() > MAX_EMAIL_LENGTH {
        errors.push(format!(
            "The email address must be {MAX_EMAIL_LENGTH} characters or fewer."
        ));
    } else if !is_valid_email(&email) {
        errors.push("Enter a valid email address.".to_owned());
    }

    if errors.is_empty() {
        Ok(NewFranchisee { name, email })
    } else {
        Err(errors)
    }
}

fn is_valid_email(email: &str) -> bool {
    let Some((local, domain)) = email.rsplit_once('@') else {
        return false;
    };

    !local.is_empty()
        && !local.contains('@')
        && !domain.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !email.chars().any(char::is_whitespace)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_the_name_and_normalises_the_email() {
        let input = validate("  Corner Bakery ", " Owner@Example.COM ").unwrap();
        assert_eq!(input.name, "Corner Bakery");
        assert_eq!(input.email, "owner@example.com");
    }

    #[test]
    fn reports_empty_fields_together() {
        let errors = validate("   ", "").unwrap_err();
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0], "Enter the franchisee's name.");
        assert_eq!(errors[1], "Enter the franchisee's email address.");
    }

    #[test]
    fn rejects_an_email_without_a_domain_suffix() {
        let errors = validate("Corner Bakery", "owner@example").unwrap_err();
        assert_eq!(errors, ["Enter a valid email address."]);
    }

    #[test]
    fn enforces_the_name_limit() {
        let errors = validate(&"a".repeat(MAX_NAME_LENGTH + 1), "owner@example.com").unwrap_err();
        assert_eq!(
            errors,
            [format!(
                "The franchisee name must be {MAX_NAME_LENGTH} characters or fewer."
            )]
        );
    }
}
