use serde::Deserialize;

const BASE_URL: &str = "https://www.fantrax.com/fxea/general";
const LEAGUE_ID: &str = "lodfazipmgiyrlxi";
const USER_AGENT: &str = "bonehead/0.1";

#[derive(Debug, Deserialize)]
pub struct FantasyTeam {
    #[serde(rename = "teamName")]
    pub team_name: String,

    pub rank: i32,

    #[serde(deserialize_with = "deserialize_f32")]
    pub points: f32,
}

fn deserialize_f32<'de, D>(deserializer: D) -> Result<f32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    value.parse::<f32>().map_err(serde::de::Error::custom)
}

// todo: update box-dyn to typed error
pub async fn get_standings() -> Result<Vec<FantasyTeam>, Box<dyn std::error::Error>> {
    let url = format!("{BASE_URL}/getStandings");

    let client = reqwest::Client::builder().user_agent(USER_AGENT).build()?;

    let response = client
        .get(url)
        .query(&[("leagueId", LEAGUE_ID)])
        .send()
        .await?
        .error_for_status()?;

    let body = response.text().await?;
    let teams: Vec<FantasyTeam> = serde_json::from_str(&body)?;

    Ok(teams)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_standings() {
        let result = get_standings().await;

        match result {
            Ok(body) => {
                assert!(!body.is_empty());
            }
            Err(e) => panic!("Request failed: {e}"),
        }
    }
}
