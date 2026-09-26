use serde::Deserialize;

const BASE_URL: &str = "https://www.fantrax.com/fxea/general";
//const LEAGUE_ID: &str = "lodfazipmgiyrlxi";
const LEAGUE_ID: &str = "kby42l6cmudgs9gb";
const USER_AGENT: &str = "bonehead/0.1";

#[derive(Debug, Deserialize)]
pub struct FantasyTeam {
    #[serde(rename = "teamName")]
    pub team_name: String,
    pub rank: i32,
    pub points: String,
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
        println!("{:?}", result);

        match result {
            Ok(body) => {
                assert!(!body.is_empty());
            }
            Err(e) => panic!("Request failed: {e}"),
        }
    }
}
