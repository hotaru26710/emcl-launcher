#![allow(clippy::module_name_repetitions)]

use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use crate::db::models::{now_rfc3339, AccountSummary, StoredAccount};
use crate::db::schema::{get_setting, set_setting};

const ACCOUNT_COLUMNS: &str = "id, username, uuid, account_type, access_token,
    refresh_token, expires_at, created_at";

pub fn offline_uuid(username: &str) -> Uuid {
    let digest = md5::compute(format!("OfflinePlayer:{username}").as_bytes());
    let mut bytes = digest.0;
    bytes[6] = (bytes[6] & 0x0f) | 0x30;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

pub fn create_offline_account(connection: &Connection, username: &str) -> Result<StoredAccount> {
    let username = username.trim();
    if username.is_empty() {
        bail!("用户名不能为空");
    }
    if username.chars().count() > 16 {
        bail!("离线用户名不能超过 16 个字符");
    }
    if username.chars().any(char::is_whitespace) {
        bail!("离线用户名不能包含空白字符");
    }

    if let Some(existing) = get_account_by_username(connection, username, "offline")? {
        set_setting(connection, "active_account_id", &existing.id)?;
        return Ok(existing);
    }

    let id = Uuid::new_v4().to_string();
    let uuid = offline_uuid(username).to_string();
    let created_at = now_rfc3339();
    connection.execute(
        "INSERT INTO accounts (
            id, username, uuid, account_type, access_token, refresh_token,
            expires_at, created_at
         ) VALUES (?1, ?2, ?3, 'offline', NULL, NULL, NULL, ?4)",
        params![id, username, uuid, created_at],
    )?;
    set_setting(connection, "active_account_id", &id)?;
    get_account(connection, &id)?.context("离线账户创建后无法读取")
}

pub fn list_accounts(connection: &Connection) -> Result<Vec<AccountSummary>> {
    let active_id = get_setting(connection, "active_account_id")?;
    let mut statement = connection.prepare(&format!(
        "SELECT {ACCOUNT_COLUMNS} FROM accounts ORDER BY created_at DESC"
    ))?;
    let rows = statement.query_map([], row_to_account)?;
    let mut accounts = Vec::new();
    for row in rows {
        let account = row?;
        accounts.push(to_summary(account, active_id.as_deref()));
    }
    Ok(accounts)
}

pub fn get_account(connection: &Connection, id: &str) -> Result<Option<StoredAccount>> {
    connection
        .query_row(
            &format!("SELECT {ACCOUNT_COLUMNS} FROM accounts WHERE id = ?1"),
            params![id],
            row_to_account,
        )
        .optional()
        .context("读取账户失败")
}

pub fn get_active_account(connection: &Connection) -> Result<Option<StoredAccount>> {
    let Some(active_id) = get_setting(connection, "active_account_id")? else {
        return Ok(None);
    };
    get_account(connection, &active_id)
}

pub fn set_active_account(connection: &Connection, id: &str) -> Result<()> {
    if get_account(connection, id)?.is_none() {
        bail!("账户不存在");
    }
    set_setting(connection, "active_account_id", id)?;
    Ok(())
}

pub fn delete_account(connection: &Connection, id: &str) -> Result<()> {
    connection.execute("DELETE FROM accounts WHERE id = ?1", params![id])?;
    if get_setting(connection, "active_account_id")?.as_deref() == Some(id) {
        connection.execute("DELETE FROM settings WHERE key = 'active_account_id'", [])?;
    }
    Ok(())
}

fn get_account_by_username(
    connection: &Connection,
    username: &str,
    account_type: &str,
) -> Result<Option<StoredAccount>> {
    connection
        .query_row(
            &format!(
                "SELECT {ACCOUNT_COLUMNS} FROM accounts
                 WHERE username = ?1 AND account_type = ?2 LIMIT 1"
            ),
            params![username, account_type],
            row_to_account,
        )
        .optional()
        .context("读取账户失败")
}

fn row_to_account(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredAccount> {
    Ok(StoredAccount {
        id: row.get(0)?,
        username: row.get(1)?,
        uuid: row.get(2)?,
        account_type: row.get(3)?,
        access_token: row.get(4)?,
        refresh_token: row.get(5)?,
        expires_at: row.get(6)?,
        created_at: row.get(7)?,
    })
}

fn to_summary(account: StoredAccount, active_id: Option<&str>) -> AccountSummary {
    AccountSummary {
        active: active_id == Some(account.id.as_str()),
        id: account.id,
        username: account.username,
        uuid: account.uuid,
        account_type: account.account_type,
        expires_at: account.expires_at,
        created_at: account.created_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_minecraft_offline_uuid() {
        assert_eq!(
            offline_uuid("Notch").to_string(),
            "b50ad385-829d-3141-a216-7e7d7539ba7f"
        );
        assert_eq!(
            offline_uuid("Steve").to_string(),
            "5627dd98-e6be-3c21-b8a8-e92344183641"
        );
    }
}
