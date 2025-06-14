use clap::Parser;
use spiegel_crawler::extract_article;

#[derive(Parser)]
pub struct Args {
    #[arg(long)]
    url: String,
    #[arg(name = "accessInfo", long)]
    access_info: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let client = reqwest::blocking::Client::new();
    extract_article(&client, &args.url, &args.access_info)?;
    Ok(())
}
