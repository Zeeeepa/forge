use std::path::Path;

use anyhow::Result;
use forge_indexer::{EmbedderType, PipelineConfig};

use super::cli::IndexArgs;

/// Load configuration from command line arguments
pub fn load_config_from_args(args: &IndexArgs) -> Result<PipelineConfig> {
    let mut config = PipelineConfig::default();
    config.batch_size = args.batch_size;
    config.max_concurrent_files = args.max_concurrent_files;

    // Configure embedder type
    config.embedder_type = match args.embedder.as_str() {
        "openai" => EmbedderType::OpenAI,
        "local" => EmbedderType::Local,
        "hybrid" => EmbedderType::Hybrid,
        _ => {
            return Err(anyhow::anyhow!(
                "Invalid embedder type: {}. Must be openai, local, or hybrid",
                args.embedder
            ));
        }
    };

    // If using OpenAI embedder, automatically use API key from environment if not provided
    if matches!(config.embedder_type, EmbedderType::OpenAI) && config.openai_api_key.is_none() {
        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            config.openai_api_key = Some(api_key);
        }
    };

    // Configure OpenAI API key if using OpenAI embedder
    if matches!(
        config.embedder_type,
        EmbedderType::OpenAI | EmbedderType::Hybrid
    ) {
        // Only override with CLI argument if it's provided
        if args.openai_api_key.is_some() {
            config.openai_api_key = args.openai_api_key.clone();
        }
        if config.openai_api_key.is_none() {
            return Err(anyhow::anyhow!(
                "OPENAI_API_KEY environment variable or --openai-api-key argument required for OpenAI embedder"
            ));
        }
    }

    // Configure local model paths
    if matches!(
        config.embedder_type,
        EmbedderType::Local | EmbedderType::Hybrid
    ) {
        if let Some(model_path) = &args.local_model_path {
            config.local_model_path = Some(Path::new(model_path).to_path_buf());
        }
        if let Some(tokenizer_path) = &args.local_tokenizer_path {
            config.local_tokenizer_path = Some(Path::new(tokenizer_path).to_path_buf());
        }
    }

    Ok(config)
}
