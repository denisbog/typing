use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Result;
use aws_sdk_dynamodb::types::AttributeValue;
use clap::Parser;
use serde_json::Value;
use spiegel_crawler::extract_article;
use tokio::task::JoinSet;

#[derive(Parser)]
pub struct Args {
    #[arg(name = "userInfo", long)]
    user_info: String,
    #[arg(name = "accessInfo", long)]
    access_info: String,
    #[arg(name = "userId", long)]
    user_id: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let client = reqwest::blocking::Client::new();
    let response = client.get("https://www.spiegel.de/services/depot/api/v1/bookmarks");
    let response = response
        .header("Cookie", format!("userInfo={}", args.user_info))
        .header(
            "User-Agent",
            "Mozilla/5.0 (X11; Linux x86_64; rv:129.0) Gecko/20100101 Firefox/129.0",
        );
    let response: Vec<String> = response.send()?.json()?;
    let ids = response.join(",");

    let response = client.get(format!(
        "https://www.spiegel.de/services/sitesearch/fetch?ids={}",
        ids
    ));
    let response = response
        .header("Cookie", format!("userInfo={}", args.user_info))
        .header(
            "User-Agent",
            "Mozilla/5.0 (X11; Linux x86_64; rv:129.0) Gecko/20100101 Firefox/129.0",
        );
    let bookmarks: Value = response.send()?.json()?;

    let config = aws_config::load_from_env().await;
    let dynamo_client = aws_sdk_dynamodb::Client::new(&config);
    let tasks: JoinSet<_> = bookmarks
        .get("results")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.get("url").unwrap().as_str().unwrap())
        .map(|url| extract_article(&client, &url.to_string(), &args.access_info))
        .filter(|item| item.is_ok())
        .flatten()
        .map(|article| {
            let mut temp: HashMap<String, AttributeValue> = serde_dynamo::to_item(article).unwrap();
            temp.insert(
                "user_id".to_string(),
                AttributeValue::S(args.user_id.clone()),
            );
            temp.insert(
                "created_at".to_string(),
                AttributeValue::N(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis()
                        .to_string(),
                ),
            );

            let dynamo_client = dynamo_client.clone();
            async move {
                dynamo_client
                    .put_item()
                    .table_name("translation")
                    .set_item(Some(temp))
                    .send()
                    .await
                    .unwrap();
            }
        })
        .collect();
    tasks.join_all().await;
    Ok(())
}
