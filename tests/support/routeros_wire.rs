//! The RouterOS API wire format for test servers: length-prefixed words and sentences.
//! Shared by `tests/mock_router.rs` and `examples/fake_routeros.rs`.
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

/// Encode one API word with the RouterOS length prefix.
pub fn word(w: &str) -> Vec<u8> {
    let b = w.as_bytes();
    let n = b.len();
    let mut out = match n {
        0..=0x7f => vec![n as u8],
        0x80..=0x3fff => ((n as u16) | 0x8000).to_be_bytes().to_vec(),
        0x4000..=0x1f_ffff => ((n as u32) | 0xc0_0000).to_be_bytes()[1..].to_vec(),
        _ => ((n as u32) | 0xe000_0000).to_be_bytes().to_vec(),
    };
    out.extend_from_slice(b);
    out
}

pub fn sentence(words: &[String]) -> Vec<u8> {
    let mut out: Vec<u8> = words.iter().flat_map(|w| word(w)).collect();
    out.push(0);
    out
}

pub async fn read_len(s: &mut TcpStream) -> std::io::Result<usize> {
    let b0 = s.read_u8().await? as usize;
    Ok(match b0 {
        0..=0x7f => b0,
        0x80..=0xbf => ((b0 & 0x3f) << 8) | s.read_u8().await? as usize,
        0xc0..=0xdf => ((b0 & 0x1f) << 16) | (s.read_u16().await? as usize),
        _ => {
            let mut b = [0u8; 3];
            s.read_exact(&mut b).await?;
            ((b0 & 0x0f) << 24) | ((b[0] as usize) << 16) | ((b[1] as usize) << 8) | b[2] as usize
        }
    })
}

pub async fn read_sentence(s: &mut TcpStream) -> std::io::Result<Vec<String>> {
    let mut words = Vec::new();
    loop {
        let n = read_len(s).await?;
        if n == 0 {
            return Ok(words);
        }
        let mut buf = vec![0u8; n];
        s.read_exact(&mut buf).await?;
        words.push(String::from_utf8_lossy(&buf).into_owned());
    }
}
