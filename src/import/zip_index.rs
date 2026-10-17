//! Bound central-directory allocation before the ZIP library builds its index.
use super::{ImportError, Limits, invalid};
use crate::i18n::Key;
use std::io::{Read, Seek, SeekFrom};

pub fn preflight<R: Read + Seek>(input: &mut R, limits: &Limits) -> Result<usize, ImportError> {
    let size = input.seek(SeekFrom::End(0))?;
    let tail_len = size.min(65_557) as usize;
    input.seek(SeekFrom::End(-(tail_len as i64)))?;
    let mut tail = vec![0; tail_len];
    input.read_exact(&mut tail)?;
    let at = (0..tail_len.saturating_sub(21))
        .rev()
        .find(|&i| {
            tail[i..].starts_with(b"PK\x05\x06")
                && i + 22 + u16_at(&tail, i + 20) as usize == tail_len
        })
        .ok_or_else(|| invalid(Key::ZipEnd))?;
    if u16_at(&tail, at + 4) != 0 || u16_at(&tail, at + 6) != 0 {
        return Err(invalid(Key::MultipartZip));
    }
    let mut count = u16_at(&tail, at + 10) as u64;
    if u16_at(&tail, at + 8) as u64 != count {
        return Err(invalid(Key::ZipCounts));
    }
    let mut bytes = u32_at(&tail, at + 12) as u64;
    if count == u16::MAX as u64 || bytes == u32::MAX as u64 || u32_at(&tail, at + 16) == u32::MAX {
        let eocd = size - tail_len as u64 + at as u64;
        input.seek(SeekFrom::Start(
            eocd.checked_sub(20)
                .ok_or_else(|| invalid(Key::Zip64Locator))?,
        ))?;
        let mut locator = [0; 20];
        input.read_exact(&mut locator)?;
        if &locator[..4] != b"PK\x06\x07" || u32_at(&locator, 4) != 0 || u32_at(&locator, 16) != 1 {
            return Err(invalid(Key::Zip64Locator));
        }
        let offset = u64_at(&locator, 8);
        if offset > eocd.saturating_sub(56) {
            return Err(invalid(Key::Zip64Directory));
        }
        input.seek(SeekFrom::Start(offset))?;
        let mut record = [0; 56];
        input.read_exact(&mut record)?;
        if &record[..4] != b"PK\x06\x06" || u32_at(&record, 16) != 0 || u32_at(&record, 20) != 0 {
            return Err(invalid(Key::Zip64Directory));
        }
        count = u64_at(&record, 32);
        if u64_at(&record, 24) != count {
            return Err(invalid(Key::Zip64Counts));
        }
        bytes = u64_at(&record, 40);
    }
    if count > limits.entries as u64 {
        return Err(ImportError::Limit(Key::ZipEntries));
    }
    let estimate = bytes
        .saturating_mul(4)
        .saturating_add(count.saturating_mul(1024));
    if bytes > 64 * 1024 * 1024 || estimate > limits.memory_bytes as u64 {
        return Err(ImportError::Limit(Key::ZipIndexMemory));
    }
    input.seek(SeekFrom::Start(0))?;
    Ok(estimate as usize)
}

// Callers pass only complete fixed-size records, so these indices are structural constants.
fn u16_at(b: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([b[i], b[i + 1]])
}
fn u32_at(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
}
fn u64_at(b: &[u8], i: usize) -> u64 {
    u64::from_le_bytes([
        b[i],
        b[i + 1],
        b[i + 2],
        b[i + 3],
        b[i + 4],
        b[i + 5],
        b[i + 6],
        b[i + 7],
    ])
}
