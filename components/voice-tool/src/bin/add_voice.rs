use anyhow::Context;
use base64::Engine;
use clap::Parser;
use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use std::{fs, path::{Path, PathBuf}};
use tokio::fs as async_fs;

const API_URL: &str = "https://api.mistral.ai/v1";
const TTS_MODEL: &str = "voxtral-mini-tts-2603";
const TRANSCRIPTION_MODEL: &str = "voxtral-mini-2602";

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, default_value = "generated")]
    root: String,

    #[arg(long)]
    voice_name: String,

    #[arg(long)]
    voice_id: String,

    #[arg(long, default_value_t = usize::MAX)]
    limit: usize,

    #[arg(long)]
    api_key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TtsResponse {
    audio_data: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ParagraphMetadata {
    original: String,
}

fn is_guardrail_error(err: &anyhow::Error) -> bool {
    let msg = err.to_string();
    msg.contains("guardrail_violation")
        || msg.contains("Request blocked by guardrail policy")
        || msg.contains("403 Forbidden")
}

fn voice_folder_name(name: &str) -> String {
    name.trim().replace(['/', '\\'], "_")
}

fn list_dirs_sorted(path: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut dirs = Vec::new();
    for entry in fs::read_dir(path).with_context(|| format!("failed to read {}", path.display()))? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            dirs.push(entry.path());
        }
    }
    dirs.sort();
    Ok(dirs)
}

async fn tts_and_transcribe(
    client: &reqwest::Client,
    text: &str,
    out_dir: &Path,
    api_key: &str,
    voice_id: &str,
) -> anyhow::Result<()> {
    let speech_res = client
        .post(format!("{API_URL}/audio/speech"))
        .bearer_auth(api_key)
        .json(&serde_json::json!({
            "input": text,
            "model": TTS_MODEL,
            "response_format": "mp3",
            "voice_id": voice_id,
        }))
        .send()
        .await?;

    if !speech_res.status().is_success() {
        anyhow::bail!(
            "TTS failed ({}): {}",
            speech_res.status(),
            speech_res.text().await?
        );
    }

    let speech_json: TtsResponse = speech_res.json().await?;
    let audio_base64 = speech_json
        .audio_data
        .context("TTS response did not include audio_data")?;

    let audio_buffer = base64::engine::general_purpose::STANDARD
        .decode(audio_base64)
        .context("failed to decode audio_data")?;
    async_fs::write(out_dir.join("output.mp3"), &audio_buffer).await?;

    let form = Form::new()
        .text("model", TRANSCRIPTION_MODEL.to_string())
        .text("response_format", "verbose_json")
        .text("timestamp_granularities[]", "word")
        .part(
            "file",
            Part::bytes(audio_buffer)
                .file_name("output.mp3")
                .mime_str("audio/mpeg")?,
        );

    let trans_res = client
        .post(format!("{API_URL}/audio/transcriptions"))
        .bearer_auth(api_key)
        .multipart(form)
        .send()
        .await?;

    if !trans_res.status().is_success() {
        anyhow::bail!(
            "Transcription failed ({}): {}",
            trans_res.status(),
            trans_res.text().await?
        );
    }

    let transcription: serde_json::Value = trans_res.json().await?;
    async_fs::write(
        out_dir.join("transcription.json"),
        serde_json::to_vec_pretty(&transcription)?,
    )
    .await?;

    Ok(())
}

async fn update_article_metadata(article_dir: &Path, voice_name: &str) -> anyhow::Result<()> {
    let metadata_path = article_dir.join("metadata.json");
    let raw = async_fs::read_to_string(&metadata_path).await?;
    let mut metadata: serde_json::Value = serde_json::from_str(&raw)?;

    if metadata.get("voices").and_then(|value| value.as_array()).is_none() {
        metadata["voices"] = serde_json::json!([]);
    }

    let voices = metadata
        .get_mut("voices")
        .and_then(|value| value.as_array_mut())
        .expect("voices array should exist");

    if !voices.iter().any(|value| value.as_str() == Some(voice_name)) {
        voices.push(serde_json::Value::String(voice_name.to_string()));
    }

    async_fs::write(metadata_path, serde_json::to_vec_pretty(&metadata)?).await?;
    Ok(())
}

async fn process_paragraph(
    client: &reqwest::Client,
    api_key: &str,
    voice_id: &str,
    voice_folder: &str,
    paragraph_dir: &Path,
) -> anyhow::Result<()> {
    let metadata: ParagraphMetadata = serde_json::from_str(
        &async_fs::read_to_string(paragraph_dir.join("metadata.json")).await?,
    )?;

    let voice_dir = paragraph_dir.join(voice_folder);
    if async_fs::try_exists(&voice_dir).await? {
        return Ok(());
    }

    async_fs::create_dir_all(&voice_dir).await?;
    async_fs::write(voice_dir.join("text.txt"), &metadata.original).await?;
    async_fs::write(
        voice_dir.join("metadata.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "voiceName": voice_folder,
            "voiceId": voice_id,
            "original": metadata.original,
            "sourceParagraph": paragraph_dir.file_name().and_then(|v| v.to_str()).unwrap_or_default(),
        }))?,
    )
    .await?;

    tts_and_transcribe(client, &metadata.original, &voice_dir, api_key, voice_id).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let api_key = args
        .api_key
        .clone()
        .or_else(|| std::env::var("MISTRAL_API_KEY").ok())
        .context("Missing MISTRAL_API_KEY environment variable")?;

    let root = PathBuf::from(&args.root);
    let voice_folder = voice_folder_name(&args.voice_name);

    let client = reqwest::Client::new();
    let mut processed = 0usize;

    let mut stop = false;

    for article_dir in list_dirs_sorted(&root)? {
        for paragraph_dir in list_dirs_sorted(&article_dir)? {
            if processed >= args.limit {
                stop = true;
                break;
            }

            let text_path = paragraph_dir.join("text.txt");
            let metadata_path = paragraph_dir.join("metadata.json");
            if !text_path.exists() || !metadata_path.exists() {
                continue;
            }

            let voice_dir = paragraph_dir.join(&voice_folder);
            if voice_dir.exists() {
                continue;
            }

            print!("processing {}... ", paragraph_dir.display());
            match process_paragraph(&client, &api_key, &args.voice_id, &voice_folder, &paragraph_dir).await {
                Ok(()) => {
                    processed += 1;
                    println!("done");
                }
                Err(err) if is_guardrail_error(&err) => {
                    eprintln!(
                        "warning: skipping guardrail-blocked paragraph {}: {}",
                        paragraph_dir.display(),
                        err
                    );
                }
                Err(err) => return Err(err),
            }
        }

        update_article_metadata(&article_dir, &voice_folder).await?;

        if stop {
            break;
        }
    }

    println!("Done. Processed {} paragraph(s)", processed);
    Ok(())
}
