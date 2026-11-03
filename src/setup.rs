use askama::Template;
use axum::{
    Form,
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    AppState,
    error::AppError,
    franchisees::{self, CreateFranchiseeError, FranchiseeOverview},
    stores::{self, CreateStoreError, StoreOverview},
};

#[derive(Debug, Default, Deserialize)]
pub struct NoticeQuery {
    pub notice: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FranchiseeForm {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct StoreForm {
    pub franchisee_id: String,
    pub name: String,
    pub address: String,
}

#[derive(Debug, Clone)]
pub struct FranchiseeChoice {
    pub id: String,
    pub name: String,
    pub selected: bool,
}

#[derive(Template)]
#[template(path = "setup.html")]
pub struct SetupPage {
    pub franchisees: Vec<FranchiseeOverview>,
    pub franchisee_choices: Vec<FranchiseeChoice>,
    pub stores: Vec<StoreOverview>,
    pub franchisee_count: usize,
    pub store_count: usize,
    pub awaiting_store_count: usize,
    pub notice: Option<String>,
    pub franchisee_errors: Vec<String>,
    pub store_errors: Vec<String>,
    pub franchisee_name: String,
    pub franchisee_email: String,
    pub store_name: String,
    pub store_address: String,
    pub store_franchisee_id: String,
}

impl SetupPage {
    async fn load(state: &AppState, notice: Option<String>) -> Result<Self, AppError> {
        let (franchisees, stores) = tokio::try_join!(
            franchisees::list_overview(&state.pool),
            stores::list_overview(&state.pool)
        )?;

        let awaiting_store_count = franchisees
            .iter()
            .filter(|franchisee| franchisee.store_count == 0)
            .count();

        let franchisee_choices = franchisees
            .iter()
            .map(|franchisee| FranchiseeChoice {
                id: franchisee.id.to_string(),
                name: franchisee.name.clone(),
                selected: false,
            })
            .collect();

        Ok(Self {
            franchisee_count: franchisees.len(),
            store_count: stores.len(),
            awaiting_store_count,
            franchisees,
            franchisee_choices,
            stores,
            notice,
            franchisee_errors: Vec::new(),
            store_errors: Vec::new(),
            franchisee_name: String::new(),
            franchisee_email: String::new(),
            store_name: String::new(),
            store_address: String::new(),
            store_franchisee_id: String::new(),
        })
    }

    fn select_franchisee(&mut self, franchisee_id: String) {
        for choice in &mut self.franchisee_choices {
            choice.selected = choice.id == franchisee_id;
        }
        self.store_franchisee_id = franchisee_id;
    }
}

impl IntoResponse for SetupPage {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(page) => Html(page).into_response(),
            Err(error) => AppError::Template(error).into_response(),
        }
    }
}

pub async fn index(
    State(state): State<AppState>,
    Query(query): Query<NoticeQuery>,
) -> Result<SetupPage, AppError> {
    let notice = match query.notice.as_deref() {
        Some("franchisee") => {
            Some("Franchisee registered. Their store can be opened next.".to_owned())
        }
        Some("store") => Some("Store opened and linked to its franchisee.".to_owned()),
        _ => None,
    };

    SetupPage::load(&state, notice).await
}

pub async fn register_franchisee(
    State(state): State<AppState>,
    Form(form): Form<FranchiseeForm>,
) -> Result<Response, AppError> {
    match franchisees::validate(&form.name, &form.email) {
        Ok(input) => match franchisees::create(&state.pool, input).await {
            Ok(_) => Ok(Redirect::to("/?notice=franchisee").into_response()),
            Err(CreateFranchiseeError::EmailTaken) => {
                let mut page = SetupPage::load(&state, None).await?;
                page.franchisee_errors =
                    vec!["A franchisee with that email address already exists.".to_owned()];
                page.franchisee_name = form.name;
                page.franchisee_email = form.email;
                Ok((StatusCode::UNPROCESSABLE_ENTITY, page).into_response())
            }
            Err(CreateFranchiseeError::Database(error)) => Err(AppError::Database(error)),
        },
        Err(errors) => {
            let mut page = SetupPage::load(&state, None).await?;
            page.franchisee_errors = errors;
            page.franchisee_name = form.name;
            page.franchisee_email = form.email;
            Ok((StatusCode::UNPROCESSABLE_ENTITY, page).into_response())
        }
    }
}

pub async fn open_store(
    State(state): State<AppState>,
    Form(form): Form<StoreForm>,
) -> Result<Response, AppError> {
    let franchisee_id = Uuid::parse_str(form.franchisee_id.trim()).ok();

    match stores::validate(&form.name, &form.address, franchisee_id) {
        Ok(input) => match stores::create(&state.pool, input).await {
            Ok(_) => Ok(Redirect::to("/?notice=store").into_response()),
            Err(CreateStoreError::UnknownFranchisee) => {
                let mut page = SetupPage::load(&state, None).await?;
                page.store_errors =
                    vec!["Choose a registered franchisee for this store.".to_owned()];
                page.store_name = form.name;
                page.store_address = form.address;
                page.select_franchisee(form.franchisee_id);
                Ok((StatusCode::UNPROCESSABLE_ENTITY, page).into_response())
            }
            Err(CreateStoreError::Database(error)) => Err(AppError::Database(error)),
        },
        Err(errors) => {
            let mut page = SetupPage::load(&state, None).await?;
            page.store_errors = errors;
            page.store_name = form.name;
            page.store_address = form.address;
            page.select_franchisee(form.franchisee_id);
            Ok((StatusCode::UNPROCESSABLE_ENTITY, page).into_response())
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use super::*;
    use crate::{franchisees::FranchiseeOverview, stores::StoreOverview};

    #[test]
    fn renders_the_setup_page_with_records_and_forms() {
        let franchisee_id = Uuid::new_v4();
        let page = SetupPage {
            franchisees: vec![FranchiseeOverview {
                id: franchisee_id,
                name: "Corner Bakery".to_owned(),
                email: "owner@example.com".to_owned(),
                created_at: Utc::now(),
                store_count: 1,
            }],
            franchisee_choices: vec![FranchiseeChoice {
                id: franchisee_id.to_string(),
                name: "Corner Bakery".to_owned(),
                selected: true,
            }],
            stores: vec![StoreOverview {
                id: Uuid::new_v4(),
                franchisee_id,
                franchisee_name: "Corner Bakery".to_owned(),
                name: "Riverside".to_owned(),
                address: "12 Mill Lane, Bristol".to_owned(),
                created_at: Utc::now(),
            }],
            franchisee_count: 1,
            store_count: 1,
            awaiting_store_count: 0,
            notice: Some("Franchisee registered.".to_owned()),
            franchisee_errors: Vec::new(),
            store_errors: Vec::new(),
            franchisee_name: String::new(),
            franchisee_email: String::new(),
            store_name: String::new(),
            store_address: String::new(),
            store_franchisee_id: franchisee_id.to_string(),
        };

        let html = page.render().expect("the setup page renders");
        assert!(html.contains("Corner Bakery"));
        assert!(html.contains("Riverside"));
        assert!(html.contains("Franchisee registered."));
        assert!(html.contains("selected"));
        assert!(html.contains("Set up your store"));
    }
}
