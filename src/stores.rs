use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{FromRow, PgPool};
use thiserror::Error;
use uuid::Uuid;

pub const MAX_NAME_LENGTH: usize = 120;
pub const MAX_ADDRESS_LENGTH: usize = 200;

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Store {
    pub id: Uuid,
    pub franchisee_id: Uuid,
    pub name: String,
    pub address: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct StoreOverview {
    pub id: Uuid,
    pub franchisee_id: Uuid,
    pub franchisee_name: String,
    pub name: String,
    pub address: String,
    pub created_at: DateTime<Utc>,
}

impl StoreOverview {
    pub fn opened_on(&self) -> String {
        self.created_at.format("%d %b %Y").to_string()
    }
}

#[derive(Debug, Clone)]
pub struct NewStore {
    pub franchisee_id: Uuid,
    pub name: String,
    pub address: String,
}

#[derive(Debug, Error)]
pub enum CreateStoreError {
    #[error("Choose a franchisee that is registered.")]
    UnknownFranchisee,
    #[error("The store could not be saved.")]
    Database(#[from] sqlx::Error),
}

pub async fn list_overview(pool: &PgPool) -> Result<Vec<StoreOverview>, sqlx::Error> {
    sqlx::query_as::<_, StoreOverview>(
        r#"
        SELECT
            s.id,
            s.franchisee_id,
            f.name AS franchisee_name,
            s.name,
            s.address,
            s.created_at
        FROM stores s
        INNER JOIN franchisees f ON f.id = s.franchisee_id
        ORDER BY s.created_at DESC, s.name ASC
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn create(pool: &PgPool, input: NewStore) -> Result<Store, CreateStoreError> {
    let franchisee_exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM franchisees WHERE id = $1)")
            .bind(input.franchisee_id)
            .fetch_one(pool)
            .await?;

    if !franchisee_exists {
        return Err(CreateStoreError::UnknownFranchisee);
    }

    sqlx::query_as::<_, Store>(
        r#"
        INSERT INTO stores (id, franchisee_id, name, address)
        VALUES ($1, $2, $3, $4)
        RETURNING id, franchisee_id, name, address, created_at
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(input.franchisee_id)
    .bind(input.name)
    .bind(input.address)
    .fetch_one(pool)
    .await
    .map_err(CreateStoreError::Database)
}

pub fn validate(
    name: &str,
    address: &str,
    franchisee_id: Option<Uuid>,
) -> Result<NewStore, Vec<String>> {
    let mut errors = Vec::new();

    let Some(franchisee_id) = franchisee_id else {
        errors.push("Choose a franchisee for this store.".to_owned());
        return Err(errors);
    };

    let name = name.trim().to_owned();
    if name.is_empty() {
        errors.push("Enter the store's name.".to_owned());
    } else if name.chars().count() > MAX_NAME_LENGTH {
        errors.push(format!(
            "The store name must be {MAX_NAME_LENGTH} characters or fewer."
        ));
    }

    let address = address.trim().to_owned();
    if address.is_empty() {
        errors.push("Enter the store's address.".to_owned());
    } else if address.chars().count() > MAX_ADDRESS_LENGTH {
        errors.push(format!(
            "The store address must be {MAX_ADDRESS_LENGTH} characters or fewer."
        ));
    }

    if errors.is_empty() {
        Ok(NewStore {
            franchisee_id,
            name,
            address,
        })
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_the_store_fields() {
        let input = validate("  Riverside ", " 12 Mill Lane ", Some(Uuid::new_v4())).unwrap();
        assert_eq!(input.name, "Riverside");
        assert_eq!(input.address, "12 Mill Lane");
    }

    #[test]
    fn requires_a_franchisee() {
        let errors = validate("Riverside", "12 Mill Lane", None).unwrap_err();
        assert_eq!(errors, ["Choose a franchisee for this store."]);
    }

    #[test]
    fn reports_empty_store_fields_together() {
        let errors = validate("", " ", Some(Uuid::new_v4())).unwrap_err();
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0], "Enter the store's name.");
        assert_eq!(errors[1], "Enter the store's address.");
    }
}
