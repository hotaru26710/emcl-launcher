#![allow(clippy::module_name_repetitions)]

use anyhow::{bail, Context, Result};
use reqwest::Client;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
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

const MICROSOFT_DEVICE_CODE_URL: &str =
    "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const MICROSOFT_TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const XBOX_USER_AUTH_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XSTS_AUTHORIZE_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
const MINECRAFT_LOGIN_URL: &str =
    "https://api.minecraftservices.com/authentication/login_with_xbox";
const MINECRAFT_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MicrosoftPollResult {
    pub status: String,
    pub message: String,
    pub account: Option<AccountSummary>,
}

#[derive(Debug, Clone)]
pub struct MicrosoftLoginData {
    pub username: String,
    pub profile_id: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone)]
pub enum MicrosoftPollOutcome {
    Pending { status: String, message: String },
    Success(MicrosoftLoginData),
}

#[derive(Debug, Deserialize)]
struct OAuthTokenResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct XboxResponse {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: DisplayClaims,
}

#[derive(Debug, Deserialize)]
struct DisplayClaims {
    xui: Vec<XboxUserHash>,
}

#[derive(Debug, Deserialize)]
struct XboxUserHash {
    uhs: String,
}

#[derive(Debug, Deserialize)]
struct MinecraftLoginResponse {
    access_token: String,
    expires_in: u64,
}

#[derive(Debug, Deserialize)]
struct MinecraftProfile {
    id: String,
    name: String,
}

pub async fn start_microsoft_device_login(
    client: &Client,
    client_id: &str,
) -> Result<DeviceCodeResponse> {
    if client_id.trim().is_empty() {
        bail!("请先在设置页填写 Microsoft Client ID");
    }

    let response = client
        .post(MICROSOFT_DEVICE_CODE_URL)
        .form(&[
            ("client_id", client_id.trim()),
            ("scope", "XboxLive.signin offline_access"),
        ])
        .send()
        .await
        .context("无法连接 Microsoft 登录服务")?;
    let status = response.status();
    let text = response.text().await?;
    if !status.is_success() {
        bail!("Microsoft 设备码请求失败: {}", compact_error(&text));
    }
    serde_json::from_str(&text).context("无法解析 Microsoft 设备码响应")
}

pub async fn poll_microsoft_device_login(
    client: &Client,
    client_id: &str,
    device_code: &str,
) -> Result<MicrosoftPollOutcome> {
    let response = client
        .post(MICROSOFT_TOKEN_URL)
        .form(&[
            ("client_id", client_id.trim()),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ("device_code", device_code),
        ])
        .send()
        .await
        .context("无法轮询 Microsoft 登录状态")?;
    let status = response.status();
    let text = response.text().await?;
    let payload: OAuthTokenResponse =
        serde_json::from_str(&text).context("无法解析 Microsoft 登录响应")?;

    if let Some(error) = payload.error {
        let message = payload.error_description.unwrap_or_else(|| error.clone());
        return Ok(MicrosoftPollOutcome::Pending {
            status: error,
            message,
        });
    }

    if !status.is_success() {
        bail!("Microsoft 登录失败: {}", compact_error(&text));
    }

    let microsoft_access_token = payload.access_token.context("Microsoft 没有返回访问令牌")?;
    let refresh_token = payload.refresh_token;
    let (minecraft_token, profile, expires_at) =
        complete_microsoft_login(client, &microsoft_access_token, payload.expires_in).await?;

    Ok(MicrosoftPollOutcome::Success(MicrosoftLoginData {
        username: profile.name,
        profile_id: profile.id,
        access_token: minecraft_token,
        refresh_token,
        expires_at,
    }))
}

pub async fn refresh_microsoft_credentials(
    client: &Client,
    client_id: &str,
    refresh_token: &str,
) -> Result<MicrosoftLoginData> {
    let response = client
        .post(MICROSOFT_TOKEN_URL)
        .form(&[
            ("client_id", client_id.trim()),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
        ])
        .send()
        .await
        .context("无法刷新 Microsoft 令牌")?;
    let status = response.status();
    let text = response.text().await?;
    let payload: OAuthTokenResponse = serde_json::from_str(&text)?;
    if let Some(error) = payload.error {
        bail!("刷新令牌失败: {}", error);
    }
    if !status.is_success() {
        bail!("刷新令牌失败: {}", compact_error(&text));
    }

    let ms_token = payload.access_token.context("刷新响应缺少访问令牌")?;
    let (minecraft_token, profile, expires_at) =
        complete_microsoft_login(client, &ms_token, payload.expires_in).await?;
    Ok(MicrosoftLoginData {
        username: profile.name,
        profile_id: profile.id,
        access_token: minecraft_token,
        refresh_token: payload
            .refresh_token
            .or_else(|| Some(refresh_token.to_string())),
        expires_at,
    })
}

pub fn save_microsoft_login(
    connection: &Connection,
    data: &MicrosoftLoginData,
) -> Result<StoredAccount> {
    upsert_microsoft_account(
        connection,
        &data.username,
        &data.profile_id,
        &data.access_token,
        data.refresh_token.as_deref(),
        data.expires_at.as_deref(),
    )
}

async fn complete_microsoft_login(
    client: &Client,
    microsoft_access_token: &str,
    _microsoft_expires_in: Option<u64>,
) -> Result<(String, MinecraftProfile, Option<String>)> {
    let xbox: XboxResponse = post_json(
        client,
        XBOX_USER_AUTH_URL,
        serde_json::json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={microsoft_access_token}")
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        }),
        None,
    )
    .await?;
    let uhs = xbox
        .display_claims
        .xui
        .first()
        .context("Xbox 响应缺少用户标识")?
        .uhs
        .clone();

    let xsts: XboxResponse = post_json(
        client,
        XSTS_AUTHORIZE_URL,
        serde_json::json!({
            "Properties": {
                "SandboxId": "RETAIL",
                "UserTokens": [xbox.token]
            },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT"
        }),
        None,
    )
    .await?;

    let minecraft_login: MinecraftLoginResponse = post_json(
        client,
        MINECRAFT_LOGIN_URL,
        serde_json::json!({
            "identityToken": format!("XBL3.0 x={uhs};{}", xsts.token)
        }),
        None,
    )
    .await?;

    let profile: MinecraftProfile = get_json(
        client,
        MINECRAFT_PROFILE_URL,
        Some(&minecraft_login.access_token),
    )
    .await?;
    let expires_at = Some(
        (chrono::Utc::now() + chrono::Duration::seconds(minecraft_login.expires_in as i64))
            .to_rfc3339(),
    );
    Ok((minecraft_login.access_token, profile, expires_at))
}

async fn post_json<T: DeserializeOwned>(
    client: &Client,
    url: &str,
    body: serde_json::Value,
    bearer: Option<&str>,
) -> Result<T> {
    let mut request = client.post(url).json(&body);
    if let Some(token) = bearer {
        request = request.bearer_auth(token);
    }
    let response = request
        .send()
        .await
        .with_context(|| format!("请求失败: {url}"))?;
    let status = response.status();
    let text = response.text().await?;
    if !status.is_success() {
        bail!("{} 返回错误: {}", url, compact_error(&text));
    }
    serde_json::from_str(&text).with_context(|| format!("无法解析 {url} 响应"))
}

async fn get_json<T: DeserializeOwned>(
    client: &Client,
    url: &str,
    bearer: Option<&str>,
) -> Result<T> {
    let mut request = client.get(url);
    if let Some(token) = bearer {
        request = request.bearer_auth(token);
    }
    let response = request
        .send()
        .await
        .with_context(|| format!("请求失败: {url}"))?;
    let status = response.status();
    let text = response.text().await?;
    if !status.is_success() {
        bail!("{} 返回错误: {}", url, compact_error(&text));
    }
    serde_json::from_str(&text).with_context(|| format!("无法解析 {url} 响应"))
}

fn compact_error(text: &str) -> String {
    serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .and_then(|value| {
            value
                .get("error_description")
                .or_else(|| value.get("error"))
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| text.chars().take(300).collect())
}

fn upsert_microsoft_account(
    connection: &Connection,
    username: &str,
    profile_id: &str,
    access_token: &str,
    refresh_token: Option<&str>,
    expires_at: Option<&str>,
) -> Result<StoredAccount> {
    let uuid = format_profile_uuid(profile_id);
    let existing = connection
        .query_row(
            &format!(
                "SELECT {ACCOUNT_COLUMNS} FROM accounts
                 WHERE account_type = 'microsoft' AND uuid = ?1 LIMIT 1"
            ),
            params![uuid],
            row_to_account,
        )
        .optional()?;

    let account = if let Some(existing) = existing {
        connection.execute(
            "UPDATE accounts
             SET username = ?1, access_token = ?2, refresh_token = ?3, expires_at = ?4
             WHERE id = ?5",
            params![
                username,
                access_token,
                refresh_token,
                expires_at,
                existing.id
            ],
        )?;
        get_account(connection, &existing.id)?.context("微软账户更新后无法读取")?
    } else {
        let id = Uuid::new_v4().to_string();
        let created_at = now_rfc3339();
        connection.execute(
            "INSERT INTO accounts (
                id, username, uuid, account_type, access_token, refresh_token,
                expires_at, created_at
             ) VALUES (?1, ?2, ?3, 'microsoft', ?4, ?5, ?6, ?7)",
            params![
                id,
                username,
                uuid,
                access_token,
                refresh_token,
                expires_at,
                created_at
            ],
        )?;
        get_account(connection, &id)?.context("微软账户创建后无法读取")?
    };

    set_setting(connection, "active_account_id", &account.id)?;
    Ok(account)
}

fn format_profile_uuid(profile_id: &str) -> String {
    if profile_id.len() == 32 && profile_id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        format!(
            "{}-{}-{}-{}-{}",
            &profile_id[0..8],
            &profile_id[8..12],
            &profile_id[12..16],
            &profile_id[16..20],
            &profile_id[20..32]
        )
    } else {
        profile_id.to_string()
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
