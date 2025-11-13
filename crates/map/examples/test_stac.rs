use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let client = reqwest::Client::new();

    let query = serde_json::json!({
        "collections": ["sentinel-2-l2a"],
        "bbox": [-17.3, 32.6, -16.65, 32.88],
        "limit": 3
    });

    println!("Querying STAC API for Madeira...\n");

    let response = client
        .post("https://earth-search.aws.element84.com/v1/search")
        .json(&query)
        .send()
        .await?;

    if !response.status().is_success() {
        println!("Error: {}", response.status());
        return Ok(());
    }

    let result: serde_json::Value = response.json().await?;

    println!("Response:\n{}", serde_json::to_string_pretty(&result)?);

    Ok(())
}
