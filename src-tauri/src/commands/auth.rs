use tauri::State;

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
