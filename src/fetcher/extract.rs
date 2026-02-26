use anyhow::{Result, anyhow};
use flate2::Decompress;
use flate2::FlushDecompress;
use flate2::Status;
use super::protocol::ZLIB_HEADERS;

/// Zlib段信息
pub struct ZlibSegment {
    pub offset: usize,
    pub compressed_len: u32,
    pub uncompressed_len: u32,
    pub data: Vec<u8>,
}

/// 精确解压Zlib数据
pub fn decompress_exact(data: &[u8]) -> Result<(Vec<u8>, usize)> {
    let mut decompress = Decompress::new(true);
    let mut output = Vec::with_capacity(data.len() * 4);
    let mut temp_buf = vec![0u8; 4096];
    
    let mut total_in = 0;
    let mut total_out = 0;
    
    loop {
        let _before_in = decompress.total_in();
        let before_out = decompress.total_out();
        
        let status = decompress.decompress(
            &data[total_in..],
            &mut temp_buf,
            FlushDecompress::None
        )?;
        
        let produced = (decompress.total_out() - before_out) as usize;
        output.extend_from_slice(&temp_buf[..produced]);
        
        total_in = decompress.total_in() as usize;
        total_out = decompress.total_out() as usize;
        
        match status {
            Status::Ok => continue,
            Status::StreamEnd => break,
            Status::BufError => {
                if produced == 0 {
                    break;
                }
            }
        }
    }
    
    Ok((output, total_in))
}

/// 猜测压缩/未压缩长度对
fn guess_len_pair(block: &[u8], zlib_pos: usize) -> Option<(u32, u32, usize)> {
    // 尝试在zlib头前8或12字节处查找长度对
    for back in &[8, 12] {
        if zlib_pos < *back {
            continue;
        }
        
        let pos = zlib_pos - back;
        if pos + 8 > block.len() {
            continue;
        }
        
        let a = u32::from_le_bytes([
            block[pos], block[pos+1], block[pos+2], block[pos+3]
        ]);
        let c = u32::from_le_bytes([
            block[pos+4], block[pos+5], block[pos+6], block[pos+7]
        ]);
        
        let total_len = block.len() - zlib_pos;
        
        // 方案1: [comp_len][uncomp_len]
        if a >= 8 && a <= total_len as u32 && c >= 16 && c <= 100_000_000 && a < c {
            return Some((a, c, pos));
        }
        
        // 方案2: [uncomp_len][comp_len]
        if c >= 8 && c <= total_len as u32 && a >= 16 && a <= 100_000_000 && c < a {
            return Some((c, a, pos));
        }
    }
    
    None
}

/// 扫描并提取Zlib段
pub fn scan_zlib_segments(block: &[u8]) -> Vec<ZlibSegment> {
    let mut segments = Vec::new();
    let mut search_pos = 0;
    
    while search_pos < block.len() - 2 {
        // 查找zlib头
        let mut zlib_pos = None;
        for i in search_pos..block.len()-1 {
            if ZLIB_HEADERS.iter().any(|h| block[i..].starts_with(h)) {
                zlib_pos = Some(i);
                break;
            }
        }
        
        let Some(pos) = zlib_pos else {
            break;
        };
        
        // 猜测长度
        if let Some((comp_len, _uncomp_len, _len_pos)) = guess_len_pair(block, pos) {
            let end = pos + comp_len as usize;
            if end <= block.len() {
                // 尝试解压
                if let Ok((data, used)) = decompress_exact(&block[pos..end]) {
                    segments.push(ZlibSegment {
                        offset: pos,
                        compressed_len: used as u32,
                        uncompressed_len: data.len() as u32,
                        data,
                    });
                    search_pos = pos + used;
                    continue;
                }
            }
        }
        
        // 无法确定长度，尝试直接解压
        if let Ok((data, used)) = decompress_exact(&block[pos..]) {
            if used > 0 {
                segments.push(ZlibSegment {
                    offset: pos,
                    compressed_len: used as u32,
                    uncompressed_len: data.len() as u32,
                    data,
                });
                search_pos = pos + used;
                continue;
            }
        }
        
        search_pos = pos + 1;
    }
    
    segments
}

/// 提取所有数据块的有效载荷
pub fn extract_payloads(response: &[u8]) -> Result<Vec<Vec<u8>>> {
    let blocks = super::protocol::slice_blocks(response);
    
    if blocks.is_empty() {
        return Err(anyhow!("No magic blocks found"));
    }
    
    let mut payloads = Vec::new();
    
    for block in blocks {
        let segments = scan_zlib_segments(block);
        for segment in segments {
            if !segment.data.is_empty() {
                payloads.push(segment.data);
            }
        }
    }
    
    Ok(payloads)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;
    
    #[test]
    fn test_decompress_exact() {
        let data = b"Hello, World! This is test data.";
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data).unwrap();
        let compressed = encoder.finish().unwrap();
        
        let (decompressed, used) = decompress_exact(&compressed).unwrap();
        assert_eq!(&decompressed, data);
        assert_eq!(used, compressed.len());
    }
}
