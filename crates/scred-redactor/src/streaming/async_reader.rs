use super::*;

pub struct AsyncRedactionReader<R> {
    inner: Option<R>,
    stream: RedactionStream,
    read_buf: Vec<u8>,
    output_buf: Vec<u8>,
    output_pos: usize,
}

impl<R> AsyncRedactionReader<R> {
    /// Create a new AsyncRedactionReader wrapping an AsyncRead source.
    pub fn new(inner: R, engine: Arc<RedactionEngine>) -> Self {
        Self {
            inner: Some(inner),
            stream: RedactionStream::new(engine),
            read_buf: vec![0u8; CHUNK_SIZE],
            output_buf: Vec::new(),
            output_pos: 0,
        }
    }

    /// Get a reference to the inner reader.
    pub fn inner(&self) -> &R {
        match self.inner.as_ref() {
            Some(r) => r,
            None => unreachable!("AsyncRedactionReader inner should always be Some"),
        }
    }

    /// Get a mutable reference to the inner reader.
    pub fn inner_mut(&mut self) -> &mut R {
        match self.inner.as_mut() {
            Some(r) => r,
            None => unreachable!("AsyncRedactionReader inner should always be Some"),
        }
    }

    /// Consume the reader and return the inner reader and any unread redacted data.
    pub fn into_inner(mut self) -> (R, Vec<u8>) {
        let inner = match self.inner.take() {
            Some(r) => r,
            None => unreachable!("AsyncRedactionReader inner should always be Some"),
        };
        // Drain remaining output buffer
        let mut remaining = Vec::new();
        if self.output_pos < self.output_buf.len() {
            remaining.extend_from_slice(&self.output_buf[self.output_pos..]);
        }
        // Finalize to get lookahead
        let (lookahead, _) = self.stream.finalize();
        remaining.extend_from_slice(&lookahead);
        (inner, remaining)
    }
}

impl<R: tokio::io::AsyncRead + Unpin> tokio::io::AsyncRead for AsyncRedactionReader<R> {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        use std::pin::Pin;
        use std::task::Poll;

        let this = self.get_mut();

        // Phase 1: Drain buffered output from previous calls
        if this.output_pos < this.output_buf.len() {
            let available = this.output_buf.len() - this.output_pos;
            let to_copy = std::cmp::min(available, buf.remaining());
            buf.put_slice(&this.output_buf[this.output_pos..this.output_pos + to_copy]);
            this.output_pos += to_copy;
            if this.output_pos == this.output_buf.len() {
                this.output_buf.clear();
                this.output_pos = 0;
            }
            return Poll::Ready(Ok(()));
        }

        // Phase 2: Read from inner until we have output or inner is exhausted
        let mut iterations = 0u32;
        loop {
            iterations += 1;
            if iterations > MAX_ITERATIONS_PER_POLL {
                // Safety valve: yield to executor.
                // Prevents starvation if inner always has data but
                // the stream hasn't accumulated enough for output.
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }

            let mut inner_buf = tokio::io::ReadBuf::new(&mut this.read_buf);
            let inner_pin = match this.inner.as_mut() {
                Some(r) => r,
                None => unreachable!("AsyncRedactionReader inner should always be Some"),
            };
            match Pin::new(inner_pin).poll_read(cx, &mut inner_buf) {
                Poll::Ready(Ok(())) => {
                    let n = inner_buf.filled().len();
                    if n == 0 {
                        // EOF — finalize the stream, flush lookahead
                        let (final_output, _stats) = this.stream.finalize();
                        return Self::put_output(this, buf, final_output);
                    }
                    let output = this.stream.feed(&this.read_buf[..n]);
                    if !output.is_empty() {
                        return Self::put_output(this, buf, output);
                    }
                    // No output — all held as lookahead. Loop to read more.
                }
                Poll::Pending => {
                    // Inner has no data. Waker is registered — we'll be
                    // polled again when data arrives.
                    return Poll::Pending;
                }
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
            }
        }
    }
}

impl<R> AsyncRedactionReader<R> {
    /// Place output into the consumer's buffer, buffering any overflow.
    fn put_output(
        this: &mut Self,
        buf: &mut tokio::io::ReadBuf<'_>,
        output: Vec<u8>,
    ) -> std::task::Poll<std::io::Result<()>> {
        use std::task::Poll;

        if output.is_empty() {
            return Poll::Ready(Ok(()));
        }
        let to_copy = std::cmp::min(output.len(), buf.remaining());
        buf.put_slice(&output[..to_copy]);
        if to_copy < output.len() {
            this.output_buf = output;
            this.output_pos = to_copy;
        }
        Poll::Ready(Ok(()))
    }
}

// ============================================================================
// Pipe — read → redact → write
// ============================================================================

impl RedactionStream {
    /// Read from an AsyncRead source, redact through this stream, and write to an AsyncWrite destination.
    /// Returns stats when the source is exhausted.
    pub async fn pipe<R, W>(
        engine: Arc<RedactionEngine>,
        reader: &mut R,
        writer: &mut W,
    ) -> std::io::Result<StreamingStats>
    where
        R: tokio::io::AsyncRead + Unpin,
        W: tokio::io::AsyncWrite + Unpin,
    {
        use tokio::io::AsyncReadExt;
        use tokio::io::AsyncWriteExt;

        let mut stream = Self::new(engine);
        let mut buf = vec![0u8; CHUNK_SIZE];
        let mut stats = StreamingStats::default();

        loop {
            let n = reader.read(&mut buf).await?;
            if n == 0 {
                let (output, final_stats) = stream.finalize();
                if !output.is_empty() {
                    writer.write_all(&output).await?;
                }
                stats.merge(final_stats);
                return Ok(stats);
            }
            let output = stream.feed(&buf[..n]);
            stats.bytes_read += n as u64;
            stats.bytes_written += output.len() as u64;
            if !output.is_empty() {
                writer.write_all(&output).await?;
            }
            stats.chunks_processed += 1;
        }
    }
}

// ============================================================================
// Legacy API — kept for backward compatibility
// ============================================================================

/// Configuration for streaming redaction
#[derive(Debug, Clone)]
pub struct StreamingConfig {
    pub chunk_size: usize,
    pub lookahead_size: usize,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            chunk_size: CHUNK_SIZE,
            lookahead_size: LOOKAHEAD_SIZE,
        }
    }
}
