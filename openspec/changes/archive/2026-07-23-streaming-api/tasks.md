## 1. RedactionStream

- [x] 1.1 Implement RedactionStream with internal lookahead management
- [x] 1.2 Implement finalize(&mut self) that returns (Vec<u8>, Stats)
- [x] 1.3 Implement Drop warning for unfinalized streams
- [x] 1.4 Keep old process_chunk / process_chunk_bytes for backward compat

## 2. DetectionStream

- [x] 2.1 Implement DetectionStream with internal lookahead management
- [x] 2.2 Implement finalize(&mut self) that returns (Vec<Match>, Stats)

## 3. AsyncRedactionReader

- [x] 3.1 Implement AsyncRedactionReader<R: AsyncRead>
- [x] 3.2 Implement poll_read with iteration cap and wake_by_ref pattern
- [x] 3.3 Implement Drop warning for cancelled futures

## 4. Streaming Pipe

- [x] 4.1 Implement RedactionStream::pipe()

## 5. HTTP Proxy Integration

- [x] 5.1 Update streaming_request.rs to use RedactionStream
- [x] 5.2 Update streaming_response.rs to use RedactionStream
- [x] 5.3 Update chunked_parser.rs to use RedactionStream

## 6. Tests

- [x] 6.1 Unit tests for RedactionStream (empty, single chunk, spanning, boundary, multiple chunks)
- [x] 6.2 Unit tests for DetectionStream (empty, single chunk, spanning, boundary)
- [x] 6.3 Async tests for AsyncRedactionReader (partial reads, slow inner, EOF, cancellation)
- [x] 6.4 Integration test for pipe()
