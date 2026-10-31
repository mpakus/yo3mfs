//! STEP Part 21 preview through a bounded, interruptible OCCT WebAssembly sandbox.
mod kernel;

use super::{ImportError, Limits, check_cancel};
use crate::{i18n::Key, scene::Scene};
use std::{io::Read, sync::atomic::AtomicBool};

const INPUT_LIMIT: usize = 64 * 1024 * 1024;
// Reserve room for the cached compiler/module, host bookkeeping, and runtime stack.
// This is an import accounting allowance, not an OS process RSS limit.
const HOST_ALLOWANCE: usize = 256 * 1024 * 1024;
const MIN_KERNEL_MEMORY: usize = 128 * 1024 * 1024;
const MAX_KERNEL_MEMORY: usize = 512 * 1024 * 1024;

fn invalid(key: Key) -> ImportError {
    ImportError::Step(key.into())
}

pub(super) fn read<R: Read>(
    mut input: R,
    limits: &Limits,
    cancel: &AtomicBool,
) -> Result<Scene, ImportError> {
    check_cancel(cancel)?;
    if limits.instances == 0 {
        return Err(ImportError::Limit(Key::InstanceCount));
    }
    if limits.memory_bytes < HOST_ALLOWANCE + MIN_KERNEL_MEMORY {
        return Err(ImportError::Limit(Key::StepMemory));
    }
    let cap = (limits.archive_bytes.min(INPUT_LIMIT as u64)) as usize;
    let mut data = Vec::new();
    let mut buffer = [0; 8192];
    loop {
        check_cancel(cancel)?;
        let n = input.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        if data.len().saturating_add(n) > cap {
            return Err(ImportError::Limit(Key::StepInputSize));
        }
        if data.len() + n > data.capacity() {
            let desired = (data.capacity().saturating_mul(2))
                .max(data.len() + n)
                .min(cap);
            // Include old and new allocations during growth, plus the eventual WASM input copy.
            if HOST_ALLOWANCE + MIN_KERNEL_MEMORY + data.capacity() + desired * 2
                > limits.memory_bytes
            {
                return Err(ImportError::Limit(Key::StepMemory));
            }
            data.try_reserve_exact(desired - data.len())
                .map_err(|_| ImportError::Limit(Key::StepMemory))?;
        }
        data.extend_from_slice(&buffer[..n]);
    }
    validate_envelope(&data, cancel)?;
    let kernel_cap = MAX_KERNEL_MEMORY.min(
        limits
            .memory_bytes
            .saturating_sub(HOST_ALLOWANCE + data.capacity()),
    );
    kernel::convert(&data, data.capacity(), kernel_cap, limits, cancel)
}

// Reject truncated/structurally malformed exchanges before OCCT can return a partial result.
// STEP schema interpretation remains OCCT's job; this is a bounded Part 21 envelope check.
fn validate_envelope(data: &[u8], cancel: &AtomicBool) -> Result<(), ImportError> {
    let data = data.trim_ascii();
    if !data.starts_with(b"ISO-10303-21;") || !data.ends_with(b"END-ISO-10303-21;") {
        return Err(invalid(Key::StepFormat));
    }
    let mut pos = 0;
    let mut next_check = 0;
    let mut depth = 0usize;
    let mut quoted = false;
    let mut comment = false;
    let mut start = 0;
    let mut section = 0;
    while pos < data.len() {
        if pos >= next_check {
            check_cancel(cancel)?;
            next_check = pos + 4096;
        }
        let byte = data[pos];
        let next = data.get(pos + 1).copied();
        if comment {
            if byte == b'*' && next == Some(b'/') {
                comment = false;
                pos += 1;
            }
        } else if quoted {
            if byte == b'\'' {
                if next == Some(b'\'') {
                    pos += 1;
                } else {
                    quoted = false;
                }
            }
        } else {
            match byte {
                0 => return Err(invalid(Key::StepFormat)),
                b'/' if next == Some(b'*') => {
                    comment = true;
                    pos += 1;
                }
                b'\'' => quoted = true,
                b'(' => {
                    depth += 1;
                    if depth > 256 {
                        return Err(invalid(Key::StepFormat));
                    }
                }
                b')' => {
                    depth = depth
                        .checked_sub(1)
                        .ok_or_else(|| invalid(Key::StepFormat))?;
                }
                b';' => {
                    if depth != 0 {
                        return Err(invalid(Key::StepFormat));
                    }
                    let mut statement = data[start..pos].trim_ascii();
                    while statement.starts_with(b"/*") {
                        let end = statement
                            .windows(2)
                            .position(|w| w == b"*/")
                            .ok_or_else(|| invalid(Key::StepFormat))?;
                        statement = statement[end + 2..].trim_ascii_start();
                    }
                    section = match (section, statement) {
                        (0, b"ISO-10303-21") => 1,
                        (1, b"HEADER") => 2,
                        (2, b"ENDSEC") => 3,
                        (2, s) if header_record(s) => 2,
                        (3, b"DATA") => 4,
                        (4, b"ENDSEC") => 5,
                        (4, s) if entity_record(s) => 4,
                        (5, b"END-ISO-10303-21") => 6,
                        _ => return Err(invalid(Key::StepFormat)),
                    };
                    start = pos + 1;
                }
                _ => {}
            }
        }
        pos += 1;
    }
    if quoted || comment || depth != 0 || section != 6 || !data[start..].trim_ascii().is_empty() {
        return Err(invalid(Key::StepFormat));
    }
    Ok(())
}

fn header_record(statement: &[u8]) -> bool {
    let end = statement
        .iter()
        .position(|b| !b.is_ascii_uppercase() && *b != b'_')
        .unwrap_or(statement.len());
    matches!(
        &statement[..end],
        b"FILE_DESCRIPTION" | b"FILE_NAME" | b"FILE_SCHEMA"
    ) && statement[end..].trim_ascii_start().starts_with(b"(")
        && statement.ends_with(b")")
}

fn entity_record(statement: &[u8]) -> bool {
    let Some(rest) = statement.strip_prefix(b"#") else {
        return false;
    };
    let digits = rest.iter().take_while(|b| b.is_ascii_digit()).count();
    if digits == 0 || digits > 10 {
        return false;
    }
    let Some(body) = rest[digits..].trim_ascii_start().strip_prefix(b"=") else {
        return false;
    };
    let body = body.trim_ascii();
    if !body.ends_with(b")") {
        return false;
    }
    // Complex entity instances begin with a parenthesis; simple instances need a type name.
    let body = body.strip_prefix(b"(").unwrap_or(body).trim_ascii_start();
    let name = body
        .iter()
        .take_while(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || **b == b'_')
        .count();
    name > 0 && body[0].is_ascii_uppercase() && body[name..].trim_ascii_start().starts_with(b"(")
}
