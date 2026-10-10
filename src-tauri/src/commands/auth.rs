use tauri::State;

use crate::commands::settings::load_settings;
use crate::core::auth;
use crate::db::models::AccountSummary;
use crate::AppState;

#[tauri::command]
pub fn create_offline_account(
    state: State<'_, AppState>,
    username: String,
) -> Result<AccountSummary, String> {
    let connection = state.db.lock().map_err(|error| error.to_string())?;
    auth::create_offline_account(&connection, &username)
        .map(|account| AccountSummary {
            id: account.id,
            username: account.username,
            uuid: account.uuid,
            account_type: account.account_type,
            expires_at: account.expires_at,
            created_at: account.created_at,
            active: true,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_accounts(state: State<'_, AppState>) -> Result<Vec<AccountSummary>, String> {
    let connection = state.db.lock().map_err(|error| error.to_string())?;
    auth::list_accounts(&connection).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_account(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let connection = state.db.lock().map_err(|error| error.to_string())?;
    auth::delete_account(&connection, &id).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn set_active_account(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let connection = state.db.lock().map_err(|error| error.to_string())?;
    auth::set_active_account(&connection, &id).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_active_account(state: State<'_, AppState>) -> Result<Option<AccountSummary>, String> {
    let connection = state.db.lock().map_err(|error| error.to_string())?;
    match auth::get_active_account(&connection).map_err(|error| error.to_string())? {
        Some(account) => Ok(Some(AccountSummary {
            id: account.id,
            username: account.username,
            uuid: account.uuid,
            account_type: account.account_type,
            expires_at: account.expires_at,
            created_at: account.created_at,
            active: true,
        })),
        None => Ok(None),
    }
}

#[tauri::command]
pub async fn start_microsoft_login(
    state: tauri::State<'_, AppState>,
) -> Result<auth::DeviceCodeResponse, String> {
    let settings = load_settings(&state)?;
    let client_id = settings.microsoft_client_id.as_deref().unwrap_or_default();
    auth::start_microsoft_device_login(&state.http, client_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn poll_microsoft_login(
    state: tauri::State<'_, AppState>,
    device_code: String,
) -> Result<auth::MicrosoftPollResult, String> {
    let settings = load_settings(&state)?;
    let client_id = settings.microsoft_client_id.as_deref().unwrap_or_default();
    let outcome = auth::poll_microsoft_device_login(&state.http, client_id, &device_code)
        .await
        .map_err(|error| error.to_string())?;

    match outcome {
        auth::MicrosoftPollOutcome::Pending { status, message } => Ok(auth::MicrosoftPollResult {
            status,
            message,
            account: None,
        }),
        auth::MicrosoftPollOutcome::Success(data) => {
            let account = {
                let connection = state.db.lock().map_err(|error| error.to_string())?;
                auth::save_microsoft_login(&connection, &data).map_err(|error| error.to_string())?
            };
            Ok(auth::MicrosoftPollResult {
                status: "success".to_string(),
                message: format!("已登录为 {}", account.username),
                account: Some(to_summary(account)),
            })
        }
    }
}

#[tauri::command]
pub async fn refresh_microsoft_account(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<AccountSummary, String> {
    let settings = load_settings(&state)?;
    let client_id = settings.microsoft_client_id.as_deref().unwrap_or_default();
    let refresh_token = {
        let connection = state.db.lock().map_err(|error| error.to_string())?;
        auth::get_account(&connection, &id)
            .map_err(|error| error.to_string())?
            .and_then(|account| account.refresh_token)
            .ok_or_else(|| "账户没有刷新令牌".to_string())?
    };
    let data = auth::refresh_microsoft_credentials(&state.http, client_id, &refresh_token)
        .await
        .map_err(|error| error.to_string())?;
    let account = {
        let connection = state.db.lock().map_err(|error| error.to_string())?;
        auth::save_microsoft_login(&connection, &data).map_err(|error| error.to_string())?
    };
    Ok(to_summary(account))
}

fn to_summary(account: crate::db::models::StoredAccount) -> AccountSummary {
    AccountSummary {
        id: account.id,
        username: account.username,
        uuid: account.uuid,
        account_type: account.account_type,
        expires_at: account.expires_at,
        created_at: account.created_at,
        active: true,
    }
}
