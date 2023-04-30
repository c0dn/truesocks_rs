use crate::models::{
    AccountStatusResult, ApiError, ApiResponse, DisableProxyRenewalResult,
    EnableProxyRenewalResult, ListHistoryResult, ListOnlineResult, ListZipSearchResult,
    ProxyCheckResult, ProxyInfo, PurchaseResult, Status, TestAndRefundResult,
};
use reqwest::header::{HeaderValue, ACCEPT_ENCODING};
use reqwest_middleware::ClientBuilder;
use reqwest_retry::policies::ExponentialBackoff;
use reqwest_retry::RetryTransientMiddleware;
use serde::de::DeserializeOwned;
use serde_json::{json, Map, Value};
use std::collections::HashMap;

pub mod models;

fn merge_values(mut params1: Value, params2: Value) -> Value {
    let params2_object = params2.as_object().expect("params2 must be an object");

    let params1_object = params1.as_object_mut().expect("params1 must be an object");

    for (key, value) in params2_object {
        params1_object.insert(key.clone(), value.clone());
    }

    params1
}

// Send requests to the API, 418 is when deserialization fails for unknown reason / Unable to send request
async fn execute_command<T: DeserializeOwned>(
    command: &str,
    api_key: String,
    additional_params: Option<Value>,
) -> Result<ApiResponse<T>, ApiError> {
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        ACCEPT_ENCODING,
        HeaderValue::from_static("gzip, deflate, br"),
    );
    let request_params = json!({
        "key": api_key,
        "cmd": command,
    });
    let merged_params = merge_values(request_params, additional_params.unwrap_or(json!({})));
    let builder = reqwest::Client::builder()
        .gzip(true)
        .connect_timeout(std::time::Duration::from_millis(3000))
        .default_headers(headers);
    let client = ClientBuilder::new(builder.build().unwrap())
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build();
    let map: Map<String, Value> = merged_params.as_object().unwrap().clone();
    let params: Vec<(String, String)> = map
        .into_iter()
        .map(|(k, v)| (k, v.as_str().unwrap().to_owned()))
        .collect();

    let url = "https://api.truesocks.net/";
    let url = reqwest::Url::parse_with_params(url, &params).unwrap();
    let res = client.get(url).send().await.map_err(|_| 418_u16)?;
    if !res.status().is_success() {
        return Err(ApiError::from(res.status().as_u16()));
    }
    let value: Value = res.json().await.map_err(|_| 418_u16)?;
    if let Ok(status) = serde_json::from_value::<Status>(value["status"].clone()) {
        if status.code != 0 && status.code != 209 {
            return Err(ApiError::from(status));
        }
    }
    let api_response = serde_json::from_value::<ApiResponse<T>>(value).map_err(|_| 418_u16)?;
    Ok(api_response)
}

pub async fn ping(api_key: String) -> Result<bool, ApiError> {
    execute_command::<bool>("Ping", api_key, None)
        .await
        .map(|_| true)
}

pub async fn list_online_proxies(api_key: String) -> Result<ListOnlineResult, ApiError> {
    execute_command::<ListOnlineResult>("ListOnline", api_key, None)
        .await
        .map(|res| res.result)
}

pub async fn list_zip_search(
    api_key: String,
    country_code: &str,
    zip_code: &str,
    units: Option<&str>,
    range: Option<u32>,
) -> Result<ListZipSearchResult, ApiError> {
    let mut params: HashMap<&str, String> = HashMap::new();
    params.insert("countrycode", country_code.parse().unwrap());
    params.insert("zipcode", zip_code.parse().unwrap());

    if let Some(units_value) = units {
        params.insert("units", units_value.parse().unwrap());
    }

    if let Some(range_value) = range {
        let range_string = range_value.clone().to_string();
        params.insert("range", range_string);
    }

    execute_command::<ListZipSearchResult>(
        "ListZipSearch",
        api_key,
        Some(serde_json::to_value(params).unwrap()),
    )
    .await
    .map(|res| res.result)
}

pub async fn list_history(
    api_key: String,
    only_active: Option<u32>,
    page: Option<u32>,
) -> Result<ListHistoryResult, ApiError> {
    let mut params: HashMap<String, String> = HashMap::new();

    if let Some(only_active_value) = only_active {
        params.insert("onlyactive".to_string(), only_active_value.to_string());
    }

    if let Some(page_value) = page {
        params.insert("page".to_string(), page_value.to_string());
    }

    execute_command::<ListHistoryResult>(
        "ListHistory",
        api_key,
        Some(serde_json::to_value(params).unwrap()),
    )
    .await
    .map(|res| res.result)
}

pub async fn regular_proxy_rent(
    api_key: String,
    proxy_info: &ProxyInfo,
) -> Result<PurchaseResult, ApiError> {
    if !proxy_info.is_fresh {
        let mut params: HashMap<&str, String> = HashMap::new();
        params.insert("proxyid", proxy_info.proxy_id.to_string());

        execute_command::<PurchaseResult>(
            "RegularProxyBuy",
            api_key,
            Some(serde_json::to_value(params).unwrap()),
        )
        .await
        .map(|res| res.result)
    } else {
        Err(ApiError::from(400_u16))
    }
}

pub async fn regular_proxy_private_rent(
    api_key: String,
    proxy_info: &ProxyInfo,
) -> Result<PurchaseResult, ApiError> {
    if !proxy_info.is_fresh && proxy_info.private_rent_cost > 0 {
        let mut params: HashMap<&str, String> = HashMap::new();
        params.insert("proxyid", proxy_info.proxy_id.to_string());

        execute_command::<PurchaseResult>(
            "RegularProxyRent",
            api_key,
            Some(serde_json::to_value(params).unwrap()),
        )
        .await
        .map(|res| res.result)
    } else {
        Err(ApiError::from(400_u16))
    }
}

pub async fn fresh_proxy_rent(
    api_key: String,
    proxy_info: &ProxyInfo,
) -> Result<PurchaseResult, ApiError> {
    if proxy_info.is_fresh {
        let mut params: HashMap<&str, String> = HashMap::new();
        params.insert("proxyid", proxy_info.proxy_id.to_string());

        execute_command::<PurchaseResult>(
            "FreshProxyBuy",
            api_key,
            Some(serde_json::to_value(params).unwrap()),
        )
        .await
        .map(|res| res.result)
    } else {
        Err(ApiError::from(400_u16))
    }
}

pub async fn fresh_proxy_private_rent(
    api_key: String,
    proxy_info: &ProxyInfo,
) -> Result<PurchaseResult, ApiError> {
    if proxy_info.is_fresh && proxy_info.private_rent_cost > 0 {
        let mut params: HashMap<&str, String> = HashMap::new();
        params.insert("proxyid", proxy_info.proxy_id.to_string());

        execute_command::<PurchaseResult>(
            "FreshProxyRent",
            api_key,
            Some(serde_json::to_value(params).unwrap()),
        )
        .await
        .map(|res| res.result)
    } else {
        Err(ApiError::from(400_u16))
    }
}

pub async fn check_purchased_proxy(
    api_key: String,
    proxy_info: &ProxyInfo,
) -> Result<ProxyCheckResult, ApiError> {
    let mut params: HashMap<&str, String> = HashMap::new();
    params.insert("proxyid", proxy_info.proxy_id.to_string());

    execute_command::<ProxyCheckResult>(
        "BoughtProxyCheck",
        api_key,
        Some(serde_json::to_value(params).unwrap()),
    )
    .await
    .map(|res| res.result)
}

pub async fn refund_purchased_proxy(
    api_key: String,
    proxy_info: &ProxyInfo,
) -> Result<TestAndRefundResult, ApiError> {
    let mut params: HashMap<&str, String> = HashMap::new();
    params.insert("proxyid", proxy_info.proxy_id.to_string());

    execute_command::<TestAndRefundResult>(
        "BoughtProxyRefund",
        api_key,
        Some(serde_json::to_value(params).unwrap()),
    )
    .await
    .map(|res| res.result)
}

pub async fn bought_proxy_renew_enable(
    api_key: String,
    history_id: u32,
) -> Result<EnableProxyRenewalResult, ApiError> {
    let params: HashMap<&str, String> = [("historyid", history_id.to_string())]
        .iter()
        .cloned()
        .collect();
    execute_command::<EnableProxyRenewalResult>(
        "BoughtProxyRenewEnable",
        api_key,
        Some(serde_json::to_value(params).unwrap()),
    )
    .await
    .map(|res| res.result)
}

pub async fn bought_proxy_renew_disable(
    api_key: String,
    history_id: u32,
) -> Result<DisableProxyRenewalResult, ApiError> {
    let params: HashMap<&str, String> = [("historyid", history_id.to_string())]
        .iter()
        .cloned()
        .collect();
    execute_command::<DisableProxyRenewalResult>(
        "BoughtProxyRenewDisable",
        api_key,
        Some(serde_json::to_value(params).unwrap()),
    )
    .await
    .map(|res| res.result)
}

// Keep note as None if you want to set it to empty string/remove it
// Returns Ok(()) if successful
pub async fn history_entry_change_note(
    api_key: String,
    history_id: u64,
    note: Option<&str>,
) -> Result<(), ApiError> {
    let mut params: HashMap<&str, String> = [("historyid", history_id.to_string())]
        .iter()
        .cloned()
        .collect();

    if let Some(note_value) = note {
        params.insert("note", note_value.to_string());
    }

    execute_command::<Option<bool>>(
        "HistoryEntryChangeNote",
        api_key,
        Some(serde_json::to_value(params).unwrap()),
    )
    .await?;
    Ok(())
}

pub async fn get_account_status(api_key: String) -> Result<AccountStatusResult, ApiError> {
    execute_command::<AccountStatusResult>("AccountStatus", api_key, None)
        .await
        .map(|res| res.result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lazy_static::lazy_static;
    use std::env;

    lazy_static! {
        static ref API_KEY: String = env::var("API_KEY").unwrap();
    }

    #[tokio::test]
    async fn test_ping() {
        let res = ping(API_KEY.to_string()).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_list_online_proxies() {
        let res = list_online_proxies(API_KEY.to_string()).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_list_zip_search() {
        let res = list_zip_search(API_KEY.to_string(), "US", "10001", None, None).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_list_history() {
        let res = list_history(API_KEY.to_string(), None, None).await;
        assert!(res.is_ok());
        println!("{:?}", res.unwrap());
    }

    #[tokio::test]
    async fn test_get_account_status() {
        let res = get_account_status(API_KEY.to_string()).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_list_note_change() {
        let res = history_entry_change_note(API_KEY.to_string(), 1254511, Some("share_lol")).await;
        assert!(res.is_ok());
        let res = history_entry_change_note(API_KEY.to_string(), 1254511, None).await;
        assert!(res.is_ok());
    }
}
