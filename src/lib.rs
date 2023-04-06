use crate::models::{ApiError, ApiResponse, ListOnlineResult, ListZipSearchResult, Status};
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
        if status.code != 0 {
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

#[cfg(test)]
mod tests {
    use super::*;

    const API_KEY: &str = "0c41959bc7104a953feadd70bc0a1c2a";

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
}
