# Forge Indexer - Code-Specific Embedding Updates

## Recent Changes

This update significantly improves the code indexing system by:

### 🤖 Code-Specific Embedding Models
- **OpenAI**: Upgraded from `text-embedding-3-small` to `text-embedding-3-large` for better code understanding
- **Local**: Changed from generic `sentence-transformers/all-MiniLM-L6-v2` to `microsoft/codebert-base` for superior code semantics
- **Enhanced Preprocessing**: Added language-specific preprocessing that preserves code structure and adds semantic markers

### 🧩 Improved Chunking Strategy
- **AST-Aware Chunking**: Enhanced chunking that respects code structure (functions, classes, methods)
- **Context Preservation**: Includes relevant comments, docstrings, and decorators with code chunks
- **Smart Fallback**: Intelligent line-based chunking with natural break points when AST parsing isn't available
- **Better Coverage**: Multi-pass chunking ensures comprehensive code coverage

### 📏 Dynamic Vector Dimensions
- **Automatic Detection**: Vector dimensions are automatically determined from the embedding model
- **Compatibility Checking**: Automatically detects and handles dimension mismatches in Qdrant collections

## ⚠️ Important: Reset Required

Since the embedding models and dimensions have changed, you **must reset your Qdrant collection** before using the updated indexer:

### Reset the Collection

```bash
# For local embedder (CodeBERT - 768 dimensions)
cargo run --bin forge-indexer -- reset --embedder local

# For OpenAI embedder (text-embedding-3-large - 3072 dimensions) 
cargo run --bin forge-indexer -- reset --embedder openai
```

### Start Indexing

After resetting, you can start the indexer normally:

```bash
# Start indexing with local embedder
cargo run --bin forge-indexer -- index --embedder local

# Start indexing with OpenAI embedder
cargo run --bin forge-indexer -- index --embedder openai
```

## New CLI Commands

The indexer now supports subcommands:

- `index`: Start the indexing service (replaces the old default behavior)
- `reset`: Reset the Qdrant collection to handle model/dimension changes

## Environment Variables

- `QDRANT_URL`: Qdrant server URL (default: `http://localhost:6334`)
- `QDRANT_COLLECTION`: Collection name (default: `forge-indexer`)
- `QDRANT_NAMESPACE_PREFIX`: Optional namespace prefix for isolation
- `OPENAI_API_KEY`: Required for OpenAI embedder
- `OPENAI_EMBEDDING_MODEL`: Override default OpenAI model

## Benefits

1. **Better Code Understanding**: Code-specific models understand programming constructs better than generic text models
2. **Improved Search Accuracy**: Enhanced chunking preserves code context and structure
3. **Language-Aware Processing**: Tailored preprocessing for different programming languages
4. **Automatic Compatibility**: Handles model changes gracefully with automatic dimension detection

## Migration

If you have existing indexed data:

1. Run the reset command for your embedder type
2. Restart the indexer to rebuild the index with the new models
3. Enjoy improved search accuracy!