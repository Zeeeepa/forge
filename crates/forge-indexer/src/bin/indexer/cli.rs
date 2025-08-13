use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Parser, Debug)]
pub enum Commands {
    /// Start the indexer service
    Index {
        /// Path to watch for file changes
        #[arg(default_value = ".")]
        path: String,

        /// Embedder type to use (openai, local, hybrid)
        #[arg(long, default_value_t = if std::env::var("OPENAI_API_KEY").is_ok() { "openai".to_string() } else { "local".to_string() })]
        embedder: String,

        /// OpenAI API key (required if using OpenAI embedder)
        #[arg(long)]
        openai_api_key: Option<String>,

        /// Local model path
        #[arg(long)]
        local_model_path: Option<String>,

        /// Local tokenizer path
        #[arg(long)]
        local_tokenizer_path: Option<String>,

        /// Batch size for processing
        #[arg(long, default_value_t = 10)]
        batch_size: usize,

        /// Maximum concurrent files to process
        #[arg(long, default_value_t = 5)]
        max_concurrent_files: usize,
    },
    /// Reset the Qdrant collection (deletes all indexed data)
    Reset {
        /// Embedder type to determine vector dimensions (openai, local, hybrid)
        #[arg(long, default_value_t = if std::env::var("OPENAI_API_KEY").is_ok() { "openai".to_string() } else { "local".to_string() })]
        embedder: String,
    },
}

#[derive(Debug)]
pub struct IndexArgs {
    pub path: String,
    pub embedder: String,
    pub openai_api_key: Option<String>,
    pub local_model_path: Option<String>,
    pub local_tokenizer_path: Option<String>,
    pub batch_size: usize,
    pub max_concurrent_files: usize,
}

impl From<&Commands> for Option<IndexArgs> {
    fn from(commands: &Commands) -> Self {
        match commands {
            Commands::Index {
                path,
                embedder,
                openai_api_key,
                local_model_path,
                local_tokenizer_path,
                batch_size,
                max_concurrent_files,
            } => Some(IndexArgs {
                path: path.clone(),
                embedder: embedder.clone(),
                openai_api_key: openai_api_key.clone(),
                local_model_path: local_model_path.clone(),
                local_tokenizer_path: local_tokenizer_path.clone(),
                batch_size: *batch_size,
                max_concurrent_files: *max_concurrent_files,
            }),
            Commands::Reset { .. } => None,
        }
    }
}
