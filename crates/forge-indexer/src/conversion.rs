use forge_domain::CodeChunk;
use forge_indexer::proto::Chunk;

/// Convert domain CodeChunk to proto Chunk for serialization
pub fn code_chunk_to_proto(chunk: CodeChunk) -> Chunk {
    Chunk {
        id: chunk.id,
        path: chunk.path,
        lang: chunk.lang,
        symbol: chunk.symbol,
        rev: chunk.rev,
        size: chunk.size as u64,
        code: chunk.content,
        summary: chunk.summary,
        embedding: None, // Don't include embedding in proto
    }
}

/// Convert proto Chunk to domain CodeChunk
pub fn proto_to_code_chunk(chunk: Chunk) -> CodeChunk {
    CodeChunk::new(
        chunk.id,
        chunk.path,
        chunk.lang,
        chunk.rev,
        chunk.code,
        0, // start_line not in proto
        0, // end_line not in proto
    )
    .symbol(chunk.symbol)
    .summary(chunk.summary)
}

/// Placeholder for RetrievedChunk - should be defined in the appropriate module
pub struct RetrievedChunk {
    pub code: String,
    pub path: String,
    pub score: f32,
    pub chunk_hash: String,
}

/// Convert domain CodeChunk to RetrievedChunk for API response
pub fn code_chunk_to_retrieved(chunk: CodeChunk, score: f32) -> RetrievedChunk {
    RetrievedChunk {
        code: chunk.content,
        path: chunk.path,
        score,
        chunk_hash: chunk.id,
    }
}