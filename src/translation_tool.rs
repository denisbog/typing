use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Error as E;
use aws_sdk_dynamodb::types::{AttributeValue, AttributeValueUpdate};
use candle_transformers::models::marian;
use clap::Parser;

use candle_core::{DType, Tensor};
use candle_nn::VarBuilder;

use futures::lock::Mutex;
use serde_dynamo::from_items;
use tokenizers::Tokenizer;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    model: Option<String>,

    #[arg(long, default_value = "tokenizer-marian-base-de.json")]
    tokenizer: String,

    #[arg(long, default_value = "tokenizer-marian-base-en.json")]
    tokenizer_dec: String,
}
use candle_core::utils::{cuda_is_available, metal_is_available};
use candle_core::{Device, Result};
use typing::application_types::{Article, Paragraph};

pub fn opus_mt_de_en() -> marian::Config {
    marian::Config {
        activation_function: candle_nn::Activation::Swish,
        d_model: 512,
        decoder_attention_heads: 8,
        decoder_ffn_dim: 2048,
        decoder_layers: 6,
        decoder_start_token_id: 58100,
        decoder_vocab_size: Some(58101),
        encoder_attention_heads: 8,
        encoder_ffn_dim: 2048,
        encoder_layers: 6,
        eos_token_id: 0,
        forced_eos_token_id: 0,
        is_encoder_decoder: true,
        max_position_embeddings: 512,
        pad_token_id: 58100,
        scale_embedding: true,
        share_encoder_decoder_embeddings: true,
        use_cache: true,
        vocab_size: 58101,
    }
}
pub fn device() -> Result<Device> {
    if cuda_is_available() {
        Ok(Device::new_cuda(0)?)
    } else if metal_is_available() {
        Ok(Device::new_metal(0)?)
    } else {
        #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
        {
            println!("Running on CPU, to run on GPU, build this example with `--features cuda`");
        }
        Ok(Device::Cpu)
    }
}

struct Translator {
    device: Device,
    config: marian::Config,
    model: marian::MTModel,
    tokenizer: Tokenizer,
    tokenizer_dec: Tokenizer,
    logits_processor: candle_transformers::generation::LogitsProcessor,
}

impl Translator {
    pub fn init(args: Args) -> anyhow::Result<Self> {
        use hf_hub::api::sync::Api;

        let config = opus_mt_de_en();

        let tokenizer =
            Tokenizer::from_file(&std::path::PathBuf::from(args.tokenizer)).map_err(E::msg)?;

        let tokenizer_dec =
            Tokenizer::from_file(&std::path::PathBuf::from(args.tokenizer_dec)).map_err(E::msg)?;

        let device = device()?;
        let vb = {
            let model = match args.model {
                Some(model) => std::path::PathBuf::from(model),
                None => Api::new()?
                    .repo(hf_hub::Repo::with_revision(
                        "Helsinki-NLP/opus-mt-de-en".to_string(),
                        hf_hub::RepoType::Model,
                        "refs/pr/4".to_string(),
                    ))
                    .get("model.safetensors")?,
            };
            unsafe { VarBuilder::from_mmaped_safetensors(&[&model], DType::F32, &device)? }
        };
        let model = marian::MTModel::new(&config, vb)?;

        let logits_processor =
            candle_transformers::generation::LogitsProcessor::new(1337, None, None);

        Ok(Translator {
            device,
            config,
            model,
            tokenizer,
            tokenizer_dec,
            logits_processor,
        })
    }

    pub fn translate(&mut self, text: String) -> anyhow::Result<String> {
        let encoder_xs = {
            let mut tokens = self
                .tokenizer
                .encode(text, true)
                .map_err(E::msg)?
                .get_ids()
                .to_vec();
            tokens.push(self.config.eos_token_id);
            let tokens = Tensor::new(tokens.as_slice(), &self.device)?.unsqueeze(0)?;
            self.model.encoder().forward(&tokens, 0)?
        };
        let mut token_ids = vec![self.config.decoder_start_token_id];

        for index in 0..1000 {
            let context_size = if index >= 1 { 1 } else { token_ids.len() };
            let start_pos = token_ids.len().saturating_sub(context_size);
            let input_ids = Tensor::new(&token_ids[start_pos..], &self.device)?.unsqueeze(0)?;
            let logits = self.model.decode(&input_ids, &encoder_xs, start_pos)?;
            let logits = logits.squeeze(0)?;
            let logits = logits.get(logits.dim(0)? - 1)?;
            let token = self.logits_processor.sample(&logits)?;
            token_ids.push(token);
            if token == self.config.eos_token_id || token == self.config.forced_eos_token_id {
                break;
            }
        }
        self.model.reset_kv_cache();
        let temp: String = self
            .tokenizer_dec
            .decode(&token_ids[1..token_ids.len() - 1], true)
            .unwrap();
        Ok(temp)
    }
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let translator = Arc::new(Mutex::new(Translator::init(args)?));

    println!("translating");
    let config = aws_config::load_from_env().await;
    let client = aws_sdk_dynamodb::Client::new(&config);
    let response = client
        .query()
        .table_name("translation")
        .index_name("translated-index")
        .key_condition_expression("translated = :translated")
        .expression_attribute_values(":translated", AttributeValue::S("false".to_string()))
        .send()
        .await
        .unwrap();

    let mut items: Vec<Article> = from_items(response.items.unwrap()).unwrap();
    println!("items {:?}", items);
    let items: Vec<&mut Article> = futures::future::join_all(items.iter_mut().map(|item| async {
        let paragraphs: Vec<String> = item
            .paragraphs
            .iter()
            .map(|paragraph| paragraph.original.clone())
            .collect();

        let translations: Vec<String> =
            futures::future::join_all(paragraphs.iter().map(|paragraph| async {
                translator
                    .lock()
                    .await
                    .translate(paragraph.clone())
                    .unwrap()
            }))
            .await;
        item.paragraphs = paragraphs
            .into_iter()
            .zip(translations.into_iter())
            .map(|(original, translation)| Paragraph {
                original,
                translation: Some(translation),
                pairs: None,
            })
            .collect();
        item
    }))
    .await;

    futures::future::join_all(items.into_iter().map(|item| async {
        let mut key = HashMap::new();
        key.insert(
            "user_id".to_string(),
            AttributeValue::S(item.user_id.clone()),
        );
        key.insert(
            "created_at".to_string(),
            AttributeValue::N(item.created_at.to_string()),
        );

        let temp: HashMap<String, AttributeValue> = serde_dynamo::to_item(item).unwrap();

        client
            .update_item()
            .table_name("translation")
            .set_key(Some(key))
            .attribute_updates(
                "translated",
                AttributeValueUpdate::builder()
                    .value(AttributeValue::S("true".to_string()))
                    .build(),
            )
            .attribute_updates(
                "paragraphs",
                AttributeValueUpdate::builder()
                    .value(temp.get("paragraphs").unwrap().clone())
                    .build(),
            )
            .send()
            .await
            .unwrap();
    }))
    .await;
    Ok(())
}
