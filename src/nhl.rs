use reqwest::{
    Client, Method,
    header::{HeaderMap, HeaderValue, USER_AGENT},
};

pub async fn fetch_team_details() -> String {
    // 1. Try to get the network response
    let response = match reqwest::get("https://api-web.nhle.com/v1/roster/VAN/current").await {
        Ok(res) => res,
        Err(e) => return format!("Network Error: {}", e), // Return early on failure
    };

    // 2. Try to extract the raw text
    let raw_string = match response.text().await {
        Ok(text) => text,
        Err(e) => return format!("Failed to read text body: {}", e),
    };

    // 3. Try to parse and prettify the JSON
    let temp_val: serde_json::Value = match serde_json::from_str(&raw_string) {
        Ok(val) => val,
        Err(_) => return raw_string, // If it's not valid JSON, just return the raw text
    };

    serde_json::to_string_pretty(&temp_val).unwrap_or(raw_string)
}

pub async fn fetch_fantasy() -> String {
    let mut headers = HeaderMap::new();
    headers.insert(
            USER_AGENT,
            HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        );

    let client = match Client::builder()
        .default_headers(headers)
        .timeout(std::time::Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(e) => return format!("Client build error: {}", e),
    };

    let league_id = "lodfazipmgiyrlxi";

    // "https://www.fantrax.com/fxea/general/getTeamRosters?leagueId=[League ID]&period=6"

    let response = match client
        .request(
            Method::GET,
            format!(
                "https://www.fantrax.com/fxea/general/getStandings?leagueId={}",
                //"https://www.fantrax.com/fxea/general/getTeamRosters?leagueId={}",
                league_id
            ),
            //"https://www.fantrax.com/fxea/general/getLeagueInfo?leagueId=lodfazipmgiyrlxi",
        )
        //.json(&json!({"excludePlayerInfo": true}))
        .send()
        .await
    {
        Ok(res) => res,
        Err(e) => return format!("Network Error: {}", e),
    };

    let raw_string = match response.text().await {
        Ok(text) => text,
        Err(e) => return format!("Failed to read text body: {}", e),
    };

    // 3. Try to parse and prettify the JSON
    let temp_val: serde_json::Value = match serde_json::from_str(&raw_string) {
        Ok(val) => val,
        Err(_) => return raw_string, // If it's not valid JSON, just return the raw text
    };

    serde_json::to_string_pretty(&temp_val).unwrap_or(raw_string)
}
