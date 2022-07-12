use reqwest::header::USER_AGENT;

pub(crate) async fn query(site: &str) -> String {
  let client = reqwest::Client::new();

  let response = client.get(site).header(USER_AGENT, "cultivation").send().await.unwrap();
  return response.text().await.unwrap();
}

#[tauri::command]
pub(crate) async fn valid_url(url: String) -> bool {
  // Check if we get a 200 response
  let client = reqwest::Client::new();

  let response = client.get(url).header(USER_AGENT, "cultivation").send().await.unwrap();

  return response.status().as_str() == "200";
}