use serde::de::{Deserializer, Error, Unexpected};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone)]
pub enum ApiError {
    RequestError(Status),
    StatusError(u16),
}

impl From<u16> for ApiError {
    fn from(status: u16) -> Self {
        ApiError::StatusError(status)
    }
}

impl From<Status> for ApiError {
    fn from(status: Status) -> Self {
        ApiError::RequestError(status)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Status {
    pub code: u64,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApiResponse<T> {
    pub status: Status,
    pub result: T,
}

fn empty_string_as_none<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    if s.is_empty() {
        Ok(None)
    } else {
        Ok(Some(s))
    }
}
fn blacklist_field<'de, D>(deserializer: D) -> Result<Option<Vec<BlacklistInfo>>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum BlacklistField {
        False(bool),
        Blacklist(Vec<BlacklistInfo>),
    }

    let blacklist_field = BlacklistField::deserialize(deserializer)?;

    match blacklist_field {
        BlacklistField::False(_) => Ok(None),
        BlacklistField::Blacklist(blacklist) => Ok(Some(blacklist)),
    }
}

fn zipcode_field<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;

    if s == "-" {
        Ok(None)
    } else {
        Ok(Some(s))
    }
}

fn ip_field<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;

    match value {
        Value::Bool(false) => Ok(None),
        Value::String(ip) => Ok(Some(ip)),
        _ => Err(Error::invalid_type(
            Unexpected::Other("boolean or string"),
            &"IP field expected to be a boolean or string",
        )),
    }
}

fn connect_info_field<'de, D>(deserializer: D) -> Result<Option<ConnectInfo>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum ConnectInfoField {
        False(bool),
        ConnectInfo(ConnectInfo),
    }

    let connect_info_field = ConnectInfoField::deserialize(deserializer)?;

    match connect_info_field {
        ConnectInfoField::False(_) => Ok(None),
        ConnectInfoField::ConnectInfo(connect_info) => Ok(Some(connect_info)),
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum BlacklistType {
    #[serde(rename = "Open Proxy")]
    OpenProxy,
    #[serde(rename = "Web Abuse")]
    WebAbuse,
    #[serde(rename = "Email Spam")]
    EmailSpam,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BlacklistInfo {
    #[serde(rename = "ID")]
    pub id: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Type")]
    pub blacklist_type: BlacklistType,
    #[serde(rename = "Desc")]
    pub desc: String,
    // Link to official blacklist documentation
    #[serde(rename = "Link", deserialize_with = "empty_string_as_none")]
    pub link: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub enum ConnectionType {
    Mobile,
    DSL,
    Hosting,
    Unknown,
    #[serde(rename = "N/A")]
    NotAvailable,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProxyInfo {
    #[serde(rename = "ProxyID")]
    pub proxy_id: u32,
    #[serde(rename = "CostBuy")]
    pub rent_cost: u32,
    #[serde(rename = "CostRent")]
    pub private_rent_cost: u32,
    #[serde(rename = "IsFresh")]
    pub is_fresh: bool,
    #[serde(rename = "IP", deserialize_with = "ip_field")]
    pub ip: Option<String>,
    #[serde(rename = "Hostname")]
    pub hostname: String,
    #[serde(rename = "ISP")]
    pub isp: String,
    #[serde(rename = "CountryCode")]
    pub country_code: String,
    #[serde(rename = "Country")]
    pub country: String,
    #[serde(rename = "Region")]
    pub region: String,
    #[serde(rename = "City")]
    pub city: String,
    #[serde(rename = "ZipCode", deserialize_with = "zipcode_field")]
    pub zip_code: Option<String>,
    #[serde(rename = "Timezone")]
    pub timezone: String,
    #[serde(rename = "Connect")]
    pub connection_type: ConnectionType,
    #[serde(rename = "Ping")]
    pub ping: f64,
    #[serde(rename = "Speed")]
    pub speed: u32,
    #[serde(rename = "UpTimeQuality")]
    pub uptime_quality: u32,
    #[serde(rename = "Blacklist", deserialize_with = "blacklist_field")]
    pub blacklist: Option<Vec<BlacklistInfo>>,
    #[serde(rename = "Distance")]
    pub distance: Option<f64>,
}

impl ProxyInfo {
    pub fn get_formatted_speed(&self) -> String {
        const KILOBYTE: f64 = 1024.0;
        const MEGABYTE: f64 = KILOBYTE * 1024.0;
        const GIGABYTE: f64 = MEGABYTE * 1024.0;

        let speed_f64 = self.speed as f64;

        if speed_f64 >= GIGABYTE {
            format!("{:.2} GB/s", speed_f64 / GIGABYTE)
        } else if speed_f64 >= MEGABYTE {
            format!("{:.2} MB/s", speed_f64 / MEGABYTE)
        } else if speed_f64 >= KILOBYTE {
            format!("{:.2} KB/s", speed_f64 / KILOBYTE)
        } else {
            format!("{} B/s", self.speed)
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConnectInfo {
    #[serde(rename = "ConnectIP")]
    pub connect_ip: String,
    #[serde(rename = "ConnectPort")]
    pub connect_port: u16,
    #[serde(rename = "ConnectSessionID")]
    pub connect_session_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ListInfo {
    #[serde(rename = "HistoryID")]
    pub history_id: u64,
    #[serde(rename = "ConnectInfo", deserialize_with = "connect_info_field")]
    pub connect_info: Option<ConnectInfo>,
    #[serde(rename = "ProxyInfo")]
    pub proxy_info: ProxyInfo,
    #[serde(rename = "LastBought")]
    pub last_bought: u64,
    #[serde(rename = "RemainingTime")]
    pub remaining_time: u64,
    #[serde(rename = "IsOnline")]
    pub is_online: bool,
    #[serde(rename = "IsFresh")]
    pub is_fresh: bool,
    #[serde(rename = "IsRented")]
    pub is_rented: bool,
    #[serde(rename = "RefundAvailable")]
    pub refund_available: bool,
    #[serde(rename = "RenewEnabled")]
    pub renew_enabled: bool,
    #[serde(rename = "RenewCountRemaining")]
    pub renew_count_remaining: u64,
    #[serde(rename = "IPHasChanged")]
    pub ip_has_changed: bool,
    #[serde(rename = "Note", deserialize_with = "empty_string_as_none")]
    pub note: Option<String>,
}

impl ListInfo {
    #[allow(dead_code)]
    fn formatted_remaining_time(&self) -> String {
        let hours = self.remaining_time / 3600;
        let minutes = (self.remaining_time % 3600) / 60;
        let seconds = self.remaining_time % 60;

        if hours > 0 {
            format!("{} Hours {} Minutes {} Seconds", hours, minutes, seconds)
        } else if minutes > 0 {
            format!("{} Minutes {} Seconds", minutes, seconds)
        } else {
            format!("{} Seconds", seconds)
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ListOnlineResult {
    #[serde(rename = "LastUpdate")]
    pub last_update: u64,
    #[serde(rename = "ProxyCount")]
    pub proxy_count: u32,
    #[serde(rename = "ProxyList")]
    pub proxy_list: Vec<ProxyInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ListZipSearchResult {
    #[serde(rename = "ServerTime")]
    pub server_time: u64,
    #[serde(rename = "SearchCountryCode")]
    pub search_country_code: String,
    #[serde(rename = "SearchUnits")]
    pub search_units: String,
    #[serde(rename = "SearchRange")]
    pub search_range: u32,
    #[serde(rename = "SearchZipCode")]
    pub search_zip_code: String,
    #[serde(rename = "ProxyCount")]
    pub proxy_count: u32,
    #[serde(rename = "ProxyList")]
    pub proxy_list: Vec<ProxyInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ListHistoryResult {
    #[serde(rename = "ServerTime")]
    pub server_time: u64,
    #[serde(rename = "HistoryCount")]
    pub history_count: u32,
    #[serde(rename = "HistoryEntriesPerPage")]
    pub history_entries_per_page: u32,
    #[serde(rename = "HistoryCurrentPage")]
    pub history_current_page: u32,
    #[serde(rename = "HistoryMaxPages")]
    pub history_max_pages: u32,
    #[serde(rename = "HistoryList")]
    pub history_list: Vec<ListInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PurchaseResult {
    #[serde(rename = "ServerTime")]
    pub server_time: Option<u64>,
    #[serde(rename = "CreditsLeft")]
    pub credits_left: Option<u32>,
    #[serde(rename = "HistoryEntry")]
    pub history_entry: Option<ListInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProxyCheckResult {
    pub tests_passed: u32,
    pub tests_total: u32,
    #[serde(rename = "tests_result")]
    pub test_result: String,
    #[serde(rename = "tests_result_str")]
    pub test_result_long: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TestAndRefundResult {
    pub tests_passed: u32,
    pub tests_total: u32,
    #[serde(rename = "tests_result")]
    pub test_result: String,
    #[serde(rename = "tests_result_str")]
    pub test_result_long: String,
    pub refund_result: String,
    #[serde(rename = "refund_result_str")]
    pub refund_result_long: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EnableProxyRenewalResult {
    #[serde(rename = "HistoryID")]
    pub history_id: u32,
    #[serde(rename = "Enabled")]
    pub enabled: bool,
    #[serde(rename = "CreditsLeft")]
    pub credits_left: u32,
    #[serde(rename = "Cost")]
    pub cost: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DisableProxyRenewalResult {
    #[serde(rename = "HistoryID")]
    pub history_id: u32,
    #[serde(rename = "Enabled")]
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AccountStatusResult {
    // account creation unix timestamp in milliseconds
    #[serde(rename = "Created")]
    pub created: u64,
    #[serde(rename = "UserID")]
    pub user_id: String,
    #[serde(rename = "Email")]
    pub email: String,
    #[serde(rename = "Active")]
    pub active: bool,
    #[serde(rename = "Plan")]
    pub plan: String,
    // credits expiration unix timestamp in milliseconds
    #[serde(rename = "Expires")]
    pub expires: u64,
    // Credits left in account
    #[serde(rename = "Credits")]
    pub credits: u32,
}
