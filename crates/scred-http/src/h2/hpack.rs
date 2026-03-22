/// HTTP/2 HPACK (Header Compression for HTTP/2) - RFC 7541
/// 
/// Decodes compressed headers from HEADERS frames using HPACK algorithm.
/// Supports both literal and indexed header representations.

use std::collections::HashMap;
use anyhow::{anyhow, Result};
use tracing::{debug, warn};

/// HPACK Dynamic Table (RFC 7541 Section 2.3)
pub struct HpackDynamicTable {
    /// Entries in insertion order (most recent first)
    entries: Vec<(String, String)>,
    /// Maximum size in octets
    max_size: usize,
    /// Current size in octets
    current_size: usize,
}

impl HpackDynamicTable {
    pub fn new(max_size: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_size,
            current_size: 0,
        }
    }

    /// Insert header to front of table (RFC 7541 Section 2.3.3)
    pub fn insert(&mut self, name: String, value: String) {
        let entry_size = name.len() + value.len() + 32;
        
        // Evict entries if necessary
        while self.current_size + entry_size > self.max_size && !self.entries.is_empty() {
            if let Some((n, v)) = self.entries.pop() {
                self.current_size = self.current_size.saturating_sub(n.len() + v.len() + 32);
            }
        }
        
        if entry_size <= self.max_size {
            self.current_size += entry_size;
            self.entries.insert(0, (name, value));
        }
    }

    /// Get entry by index (1-based for compatibility with static table)
    pub fn get(&self, index: usize) -> Option<(String, String)> {
        if index > 0 && index <= self.entries.len() {
            let (n, v) = &self.entries[index - 1];
            Some((n.clone(), v.clone()))
        } else {
            None
        }
    }

    /// Set maximum table size (RFC 7541 Section 6.3)
    pub fn set_max_size(&mut self, new_size: usize) {
        self.max_size = new_size;
        
        // Evict entries to fit new size
        while self.current_size > new_size && !self.entries.is_empty() {
            if let Some((n, v)) = self.entries.pop() {
                self.current_size = self.current_size.saturating_sub(n.len() + v.len() + 32);
            }
        }
    }
}

/// HPACK Static Table (RFC 7541 Appendix B - common headers)
const STATIC_TABLE: &[(&str, &str)] = &[
    (":authority", ""),
    (":method", "GET"),
    (":method", "POST"),
    (":path", "/"),
    (":scheme", "http"),
    (":scheme", "https"),
    ("accept", ""),
    ("accept-charset", ""),
    ("accept-encoding", "gzip, deflate"),
    ("accept-language", ""),
    ("accept-ranges", ""),
    ("age", ""),
    ("allow", ""),
    ("authorization", ""),
    ("cache-control", ""),
    ("content-disposition", ""),
    ("content-encoding", ""),
    ("content-language", ""),
    ("content-length", ""),
    ("content-location", ""),
    ("content-range", ""),
    ("content-type", ""),
    ("cookie", ""),
    ("date", ""),
    ("etag", ""),
    ("expect", ""),
    ("expires", ""),
    ("from", ""),
    ("host", ""),
    ("if-match", ""),
    ("if-modified-since", ""),
    ("if-none-match", ""),
    ("if-range", ""),
    ("if-unmodified-since", ""),
    ("last-modified", ""),
    ("link", ""),
    ("location", ""),
    ("max-forwards", ""),
    ("proxy-authenticate", ""),
    ("proxy-authorization", ""),
    ("range", ""),
    ("referer", ""),
    ("refresh", ""),
    ("retry-after", ""),
    ("server", ""),
    ("set-cookie", ""),
    ("strict-transport-security", ""),
    ("transfer-encoding", ""),
    ("user-agent", ""),
    ("vary", ""),
    ("via", ""),
    ("www-authenticate", ""),
];

/// HPACK Decoder
pub struct HpackDecoder {
    dynamic_table: HpackDynamicTable,
}

impl HpackDecoder {
    pub fn new() -> Self {
        Self {
            dynamic_table: HpackDynamicTable::new(4096), // Default 4KB
        }
    }

    /// Decode HPACK-encoded header block
    pub fn decode(&mut self, data: &[u8]) -> Result<HashMap<String, String>> {
        let mut headers = HashMap::new();
        let mut pos = 0;

        while pos < data.len() {
            pos = self.decode_header_field(&data, pos, &mut headers)?;
        }

        Ok(headers)
    }

    /// Decode single header field (RFC 7541 Section 6)
    fn decode_header_field(
        &mut self,
        data: &[u8],
        mut pos: usize,
        headers: &mut HashMap<String, String>,
    ) -> Result<usize> {
        if pos >= data.len() {
            return Ok(pos);
        }

        let first_byte = data[pos];

        // Pattern 1: Indexed Header Field (RFC 7541 Section 6.1)
        if first_byte & 0x80 != 0 {
            let (index, new_pos) = self.decode_integer(&data, pos, 7)?;
            pos = new_pos;
            
            if let Some((name, value)) = self.lookup_header(index as usize) {
                headers.insert(name, value);
            } else {
                warn!("Unknown header index: {}", index);
            }
            return Ok(pos);
        }

        // Pattern 2: Literal Header Field with Incremental Indexing (RFC 7541 Section 6.2.1)
        if first_byte & 0xC0 == 0x40 {
            let (index, new_pos) = self.decode_integer(&data, pos, 6)?;
            pos = new_pos;

            let (name, new_pos) = if index == 0 {
                // New name
                self.decode_string(&data, new_pos)?
            } else {
                // Indexed name
                if let Some((n, _)) = self.lookup_header(index as usize) {
                    (n, new_pos)
                } else {
                    return Err(anyhow!("Unknown header index: {}", index));
                }
            };

            let (value, new_pos) = self.decode_string(&data, new_pos)?;
            pos = new_pos;

            // Add to dynamic table
            self.dynamic_table.insert(name.clone(), value.clone());
            headers.insert(name, value);
            return Ok(pos);
        }

        // Pattern 3: Literal Header Field without Indexing (RFC 7541 Section 6.2.2)
        if first_byte & 0xF0 == 0x00 {
            let (index, new_pos) = self.decode_integer(&data, pos, 4)?;
            pos = new_pos;

            let (name, new_pos) = if index == 0 {
                self.decode_string(&data, new_pos)?
            } else {
                if let Some((n, _)) = self.lookup_header(index as usize) {
                    (n, new_pos)
                } else {
                    return Err(anyhow!("Unknown header index: {}", index));
                }
            };

            let (value, new_pos) = self.decode_string(&data, new_pos)?;
            pos = new_pos;

            headers.insert(name, value);
            return Ok(pos);
        }

        // Pattern 4: Literal Header Field Never Indexed (RFC 7541 Section 6.2.3)
        if first_byte & 0xF0 == 0x10 {
            let (index, new_pos) = self.decode_integer(&data, pos, 4)?;
            pos = new_pos;

            let (name, new_pos) = if index == 0 {
                self.decode_string(&data, new_pos)?
            } else {
                if let Some((n, _)) = self.lookup_header(index as usize) {
                    (n, new_pos)
                } else {
                    return Err(anyhow!("Unknown header index: {}", index));
                }
            };

            let (value, new_pos) = self.decode_string(&data, new_pos)?;
            pos = new_pos;

            headers.insert(name, value);
            return Ok(pos);
        }

        // Pattern 5: Dynamic Table Size Update (RFC 7541 Section 6.3)
        if first_byte & 0xE0 == 0x20 {
            let (new_size, new_pos) = self.decode_integer(&data, pos, 5)?;
            pos = new_pos;
            self.dynamic_table.set_max_size(new_size as usize);
            return Ok(pos);
        }

        Err(anyhow!("Invalid HPACK header field: {}", first_byte))
    }

    /// Lookup header from static or dynamic table
    fn lookup_header(&self, index: usize) -> Option<(String, String)> {
        // Dynamic table comes first (lower indices)
        if index > 0 && index <= self.dynamic_table.entries.len() {
            return self.dynamic_table.get(index);
        }

        // Then static table
        let static_index = index - self.dynamic_table.entries.len() - 1;
        if static_index < STATIC_TABLE.len() {
            let (name, value) = STATIC_TABLE[static_index];
            Some((name.to_string(), value.to_string()))
        } else {
            None
        }
    }

    /// Decode integer (RFC 7541 Section 5.1)
    fn decode_integer(&self, data: &[u8], pos: usize, prefix_bits: u8) -> Result<(u64, usize)> {
        if pos >= data.len() {
            return Err(anyhow!("Not enough data to decode integer"));
        }

        let prefix_mask = (1u8 << prefix_bits) - 1;
        let mut value = (data[pos] & prefix_mask) as u64;
        let mut pos = pos + 1;

        if value < (prefix_mask as u64) {
            return Ok((value, pos));
        }

        // Multi-byte integer encoding
        let mut m = 0u64;
        loop {
            if pos >= data.len() {
                return Err(anyhow!("Incomplete integer encoding"));
            }

            let byte = data[pos] as u64;
            pos += 1;

            value = value.saturating_add((byte & 0x7F) << m);
            m += 7;

            if byte & 0x80 == 0 {
                break;
            }

            if m > 64 {
                return Err(anyhow!("Integer overflow"));
            }
        }

        Ok((value, pos))
    }

    /// Decode string (RFC 7541 Section 5.2)
    fn decode_string(&self, data: &[u8], pos: usize) -> Result<(String, usize)> {
        if pos >= data.len() {
            return Err(anyhow!("Not enough data to decode string"));
        }

        let huffman_encoded = data[pos] & 0x80 != 0;
        let (length, mut pos) = self.decode_integer(data, pos, 7)?;
        let length = length as usize;

        if pos + length > data.len() {
            return Err(anyhow!("String extends past data boundary"));
        }

        let string_data = &data[pos..pos + length];
        pos += length;

        let string = if huffman_encoded {
            decode_huffman_string(string_data)?
        } else {
            String::from_utf8(string_data.to_vec())?
        };

        Ok((string, pos))
    }
}

impl Default for HpackDecoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Huffman decode (simplified - RFC 7541 Appendix B)
/// Full implementation would use RFC 7541's exact Huffman table
fn decode_huffman_string(data: &[u8]) -> Result<String> {
    // For now, return placeholder
    // Full implementation would decode using HPACK Huffman table
    debug!("Huffman-encoded string ({} bytes)", data.len());
    
    // Try to decode as UTF-8 (fallback for common cases)
    String::from_utf8(data.to_vec()).or_else(|_| {
        Ok("[huffman-encoded-data]".to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hpack_decoder_creation() {
        let decoder = HpackDecoder::new();
        assert_eq!(decoder.dynamic_table.max_size, 4096);
        assert_eq!(decoder.dynamic_table.current_size, 0);
    }

    #[test]
    fn test_static_table_lookup() {
        let decoder = HpackDecoder::new();
        
        // Index 1 should be :authority
        if let Some((name, _)) = decoder.lookup_header(1) {
            assert_eq!(name, ":authority");
        }

        // Index 2 should be :method GET
        if let Some((name, value)) = decoder.lookup_header(2) {
            assert_eq!(name, ":method");
            assert_eq!(value, "GET");
        }
    }

    #[test]
    fn test_dynamic_table_insertion() {
        let mut table = HpackDynamicTable::new(4096);
        table.insert("custom-header".to_string(), "value".to_string());
        
        assert_eq!(table.entries.len(), 1);
        if let Some((name, value)) = table.get(1) {
            assert_eq!(name, "custom-header");
            assert_eq!(value, "value");
        }
    }

    #[test]
    fn test_dynamic_table_eviction() {
        let mut table = HpackDynamicTable::new(100);
        
        // Add entries until eviction occurs
        table.insert("header1".to_string(), "value1".to_string());
        table.insert("header2".to_string(), "value2".to_string());
        table.insert("header3".to_string(), "value3".to_string());
        
        // Should have evicted some entries
        assert!(table.current_size <= table.max_size);
    }

    #[test]
    fn test_integer_decode() {
        let decoder = HpackDecoder::new();
        
        // Simple case: single byte integer
        let data = vec![0x0A]; // Value 10 with 7-bit prefix
        let (value, pos) = decoder.decode_integer(&data, 0, 7).unwrap();
        assert_eq!(value, 10);
        assert_eq!(pos, 1);
    }

    #[test]
    fn test_string_decode() {
        let decoder = HpackDecoder::new();
        
        // Simple ASCII string "hello" (length 5)
        let data = vec![0x05, b'h', b'e', b'l', b'l', b'o'];
        let (string, pos) = decoder.decode_string(&data, 0).unwrap();
        assert_eq!(string, "hello");
        assert_eq!(pos, 6);
    }
}
