/// Phase 4: Chunked Transfer-Encoding Parser
///
/// Parses and redacts chunked HTTP responses without buffering entire response.
/// Handles pattern boundaries via lookahead buffer.
use anyhow::{anyhow, Result};
use scred_redactor::{RedactionStream, StreamingRedactor};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tracing::{debug, warn};

/// Chunk parsing state machine
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChunkState {
    /// Reading chunk size line: "[hex-size][;extensions]\r\n"
    ReadingSize,
    /// Reading exact chunk data: [N bytes]
    ReadingData,
    /// Reading trailer headers after final chunk
    ReadingTrailers,
    /// All chunks consumed
    Complete,
}

/// Statistics from chunk processing
#[derive(Debug, Clone, Default)]
pub struct ChunkStats {
    pub chunks_read: u64,
    pub total_data_bytes: u64,
    pub patterns_found: u64,
    pub lookahead_hits: u64,
}

/// Chunked transfer-encoding parser
///
/// Handles RFC 7230 chunked encoding:
/// ```text
/// chunk-size [ chunk-extension ] CRLF chunk-data CRLF
/// ...
/// 0 [ chunk-extension ] CRLF [ trailer-section ] CRLF
/// ```
pub struct ChunkedParser {
    state: ChunkState,
    current_chunk_size: usize,
    bytes_remaining_in_chunk: usize,
    stream: Option<RedactionStream>,
}

impl ChunkedParser {
    /// Create new chunked parser
    pub fn new() -> Self {
        Self {
            state: ChunkState::ReadingSize,
            current_chunk_size: 0,
            bytes_remaining_in_chunk: 0,
            stream: None,
        }
    }

    /// Initialize the redaction stream (called before first chunk)
    pub fn init_stream(&mut self, engine: Arc<scred_redactor::RedactionEngine>) {
        self.stream = Some(RedactionStream::new(engine));
    }

    /// Parse next chunk from reader
    ///
    /// Returns:
    /// - `data`: Redacted chunk data (empty if final chunk reached)
    /// - `stats`: Redaction statistics
    pub async fn next_chunk<R: AsyncReadExt + Unpin>(
        &mut self,
        reader: &mut BufReader<R>,
        redactor: Arc<StreamingRedactor>,
    ) -> Result<(Vec<u8>, ChunkStats)> {
        let mut stats = ChunkStats::default();

        loop {
            match self.state {
                ChunkState::ReadingSize => {
                    self.handle_reading_size(reader).await?;
                }
                ChunkState::ReadingData => {
                    return self.handle_reading_data(reader, &mut stats).await;
                }
                ChunkState::ReadingTrailers => {
                    self.handle_reading_trailers(reader).await?;
                    return Ok((Vec::new(), stats));
                }
                ChunkState::Complete => {
                    return Ok((Vec::new(), stats));
                }
            }
        }
    }

    /// Handle ReadingSize state: parse chunk size
    async fn handle_reading_size<R: AsyncReadExt + Unpin>(
        &mut self,
        reader: &mut BufReader<R>,
    ) -> Result<()> {
        let mut size_line = String::new();
        reader.read_line(&mut size_line).await?;
        debug!("[chunked] Size line: {:?}", size_line);

        let size_str = size_line.split(';').next().unwrap_or("").trim();
        let chunk_size = usize::from_str_radix(size_str, 16)
            .map_err(|e| anyhow!("Invalid chunk size '{}': {}", size_str, e))?;
        debug!("[chunked] Chunk size: {} bytes", chunk_size);

        if chunk_size == 0 {
            self.state = ChunkState::ReadingTrailers;
        } else {
            self.current_chunk_size = chunk_size;
            self.bytes_remaining_in_chunk = chunk_size;
            self.state = ChunkState::ReadingData;
        }
        Ok(())
    }

    /// Handle ReadingData state: read and redact chunk data
    async fn handle_reading_data<R: AsyncReadExt + Unpin>(
        &mut self,
        reader: &mut BufReader<R>,
        stats: &mut ChunkStats,
    ) -> Result<(Vec<u8>, ChunkStats)> {
        let mut chunk_data = vec![0u8; self.bytes_remaining_in_chunk];
        reader.read_exact(&mut chunk_data).await?;
        debug!("[chunked] Read chunk data: {} bytes", chunk_data.len());

        let stream = self.stream.as_mut()
            .expect("ChunkedParser::init_stream() must be called before next_chunk()");
        let redacted = stream.feed(&chunk_data);

        stats.total_data_bytes += chunk_data.len() as u64;
        stats.chunks_read += 1;
        if !redacted.is_empty() {
            stats.lookahead_hits += 1;
        }

        let mut trailing = [0u8; 2];
        reader.read_exact(&mut trailing).await?;
        if trailing != *b"\r\n" {
            warn!("[chunked] Expected \\r\\n after chunk, got {:?}", trailing);
        }

        self.state = ChunkState::ReadingSize;
        Ok((redacted, stats.clone()))
    }

    /// Handle ReadingTrailers state: read trailers and finalize
    async fn handle_reading_trailers<R: AsyncReadExt + Unpin>(
        &mut self,
        reader: &mut BufReader<R>,
    ) -> Result<()> {
        let mut line = String::new();
        loop {
            line.clear();
            reader.read_line(&mut line).await?;
            if line == "\r\n" || line.is_empty() {
                break;
            }
        }

        if let Some(stream) = self.stream.as_mut() {
            let (final_output, _final_stats) = stream.finalize();
            if !final_output.is_empty() {
                debug!("[chunked] Final lookahead: {} bytes", final_output.len());
            }
        }

        self.state = ChunkState::Complete;
        Ok(())
    }

    /// Check if parsing is complete
    pub fn is_complete(&self) -> bool {
        self.state == ChunkState::Complete
    }
}

impl Default for ChunkedParser {
    fn default() -> Self {
        Self::new()
    }
}
