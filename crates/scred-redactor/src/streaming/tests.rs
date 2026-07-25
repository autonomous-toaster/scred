
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::streaming::*;
    use std::sync::Arc;
    use crate::{RedactionConfig, RedactionEngine};

    fn test_engine() -> Arc<RedactionEngine> {
        Arc::new(RedactionEngine::new(RedactionConfig { enabled: true }))
    }

    // ========================================================================
    // RedactionStream tests
    // ========================================================================

    #[test]
    fn test_redaction_stream_empty() {
        let mut stream = RedactionStream::new(test_engine());
        let out = stream.feed(b"");
        assert!(out.is_empty());
        let (final_out, stats) = stream.finalize();
        assert!(final_out.is_empty());
        assert_eq!(stats.bytes_read, 0);
    }

    #[test]
    fn test_redaction_stream_no_secrets() {
        let mut stream = RedactionStream::new(test_engine());
        // Data < 512B is held in lookahead, returned by finalize
        let out = stream.feed(b"hello world this is plain text");
        assert!(out.is_empty(), "small data held in lookahead");
        let (final_out, stats) = stream.finalize();
        assert_eq!(final_out.len(), 30);
        assert_eq!(stats.patterns_found, 0);
    }

    #[test]
    fn test_redaction_stream_single_secret() {
        let mut stream = RedactionStream::new(test_engine());
        // Data < 512B is held in lookahead
        let out = stream.feed(b"AKIAIOSFODNN7EXAMPLE");
        assert!(out.is_empty(), "small data held in lookahead");
        let (final_out, _) = stream.finalize();
        assert_eq!(String::from_utf8_lossy(&final_out), "AKIAxxxxxxxxxxxxxxxx");
    }

    #[test]
    fn test_redaction_stream_secret_spanning_chunks() {
        let mut stream = RedactionStream::new(test_engine());

        // First chunk: 400 bytes of padding (not 'A' to avoid false prefix match) + prefix
        let mut chunk1 = vec![b'X'; 400];
        chunk1.extend_from_slice(b"some data AKIA");
        let out1 = stream.feed(&chunk1);
        // 413 < 512, all held in lookahead
        assert!(out1.is_empty(), "413B < 512B lookahead");

        // Second chunk: rest of the secret + padding to exceed 512
        let mut chunk2 = b"IOSFODNN7EXAMPLE more data"[..].to_vec();
        chunk2.extend_from_slice(&vec![b'Y'; 200]);
        let out2 = stream.feed(&chunk2);
        // Combined = 413 + 219 = 632 > 512, output is first 120 bytes (all X's)
        // The redacted secret is in the lookahead (bytes 120-631)
        assert_eq!(out2.len(), 128, "output should be 640-512=128 bytes");

        let (final_out, stats) = stream.finalize();
        let final_str = String::from_utf8_lossy(&final_out);
        assert!(final_str.contains("AKIAxxxxxxxxxxxxxxxx"), "final: {}", final_str);
        assert!(stats.patterns_found > 0);
    }

    #[test]
    fn test_redaction_stream_multiple_chunks() {
        let mut stream = RedactionStream::new(test_engine());

        // Feed enough data to exceed lookahead
        let prefix = vec![b'A'; 400]; // 400 bytes of padding
        let out1 = stream.feed(&prefix);
        assert!(out1.is_empty(), "400B < 512B lookahead");

        let out2 = stream.feed(b"AKIAIOSFODNN7EXAMPLE");
        // Combined = 400 + 20 = 420, still < 512
        assert!(out2.is_empty(), "420B < 512B lookahead");

        let out3 = stream.feed(b" and more data here");
        // Combined = 420 + 19 = 439, still < 512
        assert!(out3.is_empty(), "439B < 512B lookahead");

        let out4 = stream.feed(b"X"); // 440 total
        // Still < 512
        assert!(out4.is_empty());

        let (final_out, stats) = stream.finalize();
        let final_str = String::from_utf8_lossy(&final_out);
        assert!(final_str.contains("AKIAxxxxxxxxxxxxxxxx"), "final: {}", final_str);
        assert!(stats.patterns_found > 0);
    }

    #[test]
    fn test_redaction_stream_finalize_empty() {
        let mut stream = RedactionStream::new(test_engine());
        let (out, stats) = stream.finalize();
        assert!(out.is_empty());
        assert_eq!(stats.bytes_read, 0);
    }

    #[test]
    fn test_redaction_stream_large_data() {
        let mut stream = RedactionStream::new(test_engine());
        // Feed enough data to fill the lookahead and produce output
        let data = vec![b'A'; LOOKAHEAD_SIZE + 100];
        let out = stream.feed(&data);
        // Should produce output (data > lookahead)
        assert!(!out.is_empty());
        assert_eq!(out.len(), 100); // 100 bytes past lookahead
        let (final_out, _) = stream.finalize();
        assert_eq!(final_out.len(), LOOKAHEAD_SIZE);
    }

    // ========================================================================
    // DetectionStream tests
    // ========================================================================

    #[test]
    fn test_detection_stream_empty() {
        let mut detector = DetectionStream::new(test_engine());
        let matches = detector.feed(b"");
        assert!(matches.is_empty());
        let (final_matches, _) = detector.finalize();
        assert!(final_matches.is_empty());
    }

    #[test]
    fn test_detection_stream_no_secrets() {
        let mut detector = DetectionStream::new(test_engine());
        let matches = detector.feed(b"hello world");
        assert!(matches.is_empty());
        let (final_matches, _) = detector.finalize();
        assert!(final_matches.is_empty());
    }

    #[test]
    fn test_detection_stream_single_secret() {
        let mut detector = DetectionStream::new(test_engine());
        // Data < 512B is held in lookahead
        let matches = detector.feed(b"AKIAIOSFODNN7EXAMPLE");
        assert!(matches.is_empty(), "small data held in lookahead");
        let (final_matches, _) = detector.finalize();
        assert!(!final_matches.is_empty(), "should detect AWS key on finalize");
    }

    #[test]
    fn test_detection_stream_secret_spanning_chunks() {
        let mut detector = DetectionStream::new(test_engine());

        // First chunk: padding + prefix at the end
        let mut chunk1 = vec![b'X'; 400];
        chunk1.extend_from_slice(b"some data AKIA");
        let matches1 = detector.feed(&chunk1);
        assert!(matches1.is_empty(), "prefix in lookahead should not match yet");

        // Second chunk: rest of the secret
        let mut chunk2 = b"IOSFODNN7EXAMPLE more data"[..].to_vec();
        chunk2.extend_from_slice(&vec![b'Y'; 200]);
        let matches2 = detector.feed(&chunk2);
        // Matches in the output region (first 120 bytes) — none, they're in lookahead
        // The match will be returned by finalize()

        let (final_matches, _) = detector.finalize();
        assert!(!final_matches.is_empty(), "should detect spanning secret on finalize");
    }

    // ========================================================================
    // AsyncRedactionReader tests (sync mock)
    // ========================================================================

    /// A simple sync reader that implements AsyncRead via a cursor
    struct SyncReader {
        data: Vec<u8>,
        pos: usize,
    }

    impl SyncReader {
        fn new(data: &[u8]) -> Self {
            Self {
                data: data.to_vec(),
                pos: 0,
            }
        }
    }

    impl tokio::io::AsyncRead for SyncReader {
        fn poll_read(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            use std::task::Poll;
            let this = self.get_mut();
            let remaining = this.data.len() - this.pos;
            if remaining == 0 {
                return Poll::Ready(Ok(()));
            }
            let to_copy = std::cmp::min(remaining, buf.remaining());
            buf.put_slice(&this.data[this.pos..this.pos + to_copy]);
            this.pos += to_copy;
            Poll::Ready(Ok(()))
        }
    }

    #[tokio::test]
    async fn test_async_reader_basic() {
        let engine = test_engine();
        let data = b"hello AKIAIOSFODNN7EXAMPLE world";
        let reader = SyncReader::new(data);
        let mut redacted_reader = AsyncRedactionReader::new(reader, engine);

        let mut output = Vec::new();
        tokio::io::AsyncReadExt::read_to_end(&mut redacted_reader, &mut output)
            .await
            .unwrap();

        let output_str = String::from_utf8_lossy(&output);
        assert!(output_str.contains("hello "), "output: {}", output_str);
        assert!(output_str.contains("AKIAxxxxxxxxxxxxxxxx"), "output: {}", output_str);
        assert!(output_str.contains(" world"), "output: {}", output_str);
    }

    #[tokio::test]
    async fn test_async_reader_empty() {
        let engine = test_engine();
        let reader = SyncReader::new(b"");
        let mut redacted_reader = AsyncRedactionReader::new(reader, engine);

        let mut output = Vec::new();
        tokio::io::AsyncReadExt::read_to_end(&mut redacted_reader, &mut output)
            .await
            .unwrap();

        assert!(output.is_empty());
    }

    #[tokio::test]
    async fn test_async_reader_partial_reads() {
        let engine = test_engine();
        let data = b"AKIAIOSFODNN7EXAMPLE";
        let reader = SyncReader::new(data);
        let mut redacted_reader = AsyncRedactionReader::new(reader, engine);

        // Read 1 byte at a time
        let mut output = Vec::new();
        let mut buf = [0u8; 1];
        loop {
            let n = tokio::io::AsyncReadExt::read(&mut redacted_reader, &mut buf)
                .await
                .unwrap();
            if n == 0 {
                break;
            }
            output.push(buf[0]);
        }

        let output_str = String::from_utf8_lossy(&output);
        assert_eq!(output_str, "AKIAxxxxxxxxxxxxxxxx");
    }

    #[tokio::test]
    async fn test_async_reader_into_inner() {
        let engine = test_engine();
        let data = b"hello world";
        let reader = SyncReader::new(data);
        let redacted_reader = AsyncRedactionReader::new(reader, engine);

        // into_inner should return the inner reader without panicking
        let (_inner, remaining) = redacted_reader.into_inner();
        // Data was never fed to the stream (no poll_read calls),
        // so remaining should be empty
        assert!(remaining.is_empty());
    }

    // ========================================================================
    // Pipe tests
    // ========================================================================

    #[tokio::test]
    async fn test_pipe_basic() {
        let engine = test_engine();
        let input = b"hello AKIAIOSFODNN7EXAMPLE world";
        let mut reader = &input[..];
        let mut output = Vec::new();
        let stats = RedactionStream::pipe(
            engine,
            &mut reader,
            &mut output,
        )
        .await
        .unwrap();

        let output_str = String::from_utf8_lossy(&output);
        assert!(output_str.contains("AKIAxxxxxxxxxxxxxxxx"), "output: {}", output_str);
        assert!(stats.patterns_found > 0);
        assert!(stats.bytes_read > 0);
    }
}
