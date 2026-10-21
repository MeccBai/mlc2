//! Preserve NeverStepInto rows, which LLVM's CodeViewDebug filters from DILocation.
//! Only the line-number field in C13 DEBUG_S_LINES records is changed.
use std::path::Path;
const PLACEHOLDER: u32 = 0xefffff;
const NEVER_STEP_INTO: u32 = 0xf00f00;
fn u32_at(bytes: &[u8], at: usize) -> Result<u32, String> {
    Ok(u32::from_le_bytes(
        bytes
            .get(at..at + 4)
            .ok_or("Truncated CodeView record")?
            .try_into()
            .unwrap(),
    ))
}
fn rewrite(bytes: &mut [u8]) -> Result<bool, String> {
    if bytes.len() < 20 {
        return Err("Truncated COFF header".into());
    }
    let sections = u16::from_le_bytes([bytes[2], bytes[3]]) as usize;
    let optional = u16::from_le_bytes([bytes[16], bytes[17]]) as usize;
    let mut changed = false;
    for index in 0..sections {
        let header = 20 + optional + index * 40;
        let name = bytes
            .get(header..header + 8)
            .ok_or("Truncated COFF section table")?;
        if name != b".debug$S" {
            continue;
        }
        let size = u32_at(bytes, header + 16)? as usize;
        let start = u32_at(bytes, header + 20)? as usize;
        let end = start
            .checked_add(size)
            .filter(|end| *end <= bytes.len())
            .ok_or("Invalid CodeView section range")?;
        if size < 4 || u32_at(bytes, start)? != 4 {
            continue;
        }
        let mut subsection = start + 4;
        while subsection + 8 <= end {
            let kind = u32_at(bytes, subsection)? & 0x7fff_ffff;
            let length = u32_at(bytes, subsection + 4)? as usize;
            let begin = subsection + 8;
            let limit = begin
                .checked_add(length)
                .filter(|limit| *limit <= end)
                .ok_or("Invalid CodeView subsection range")?;
            if kind == 0xf2 {
                if length < 12 {
                    return Err("Truncated CodeView lines header".into());
                }
                let mut block = begin + 12;
                while block < limit {
                    if block + 12 > limit {
                        return Err("Truncated CodeView file block".into());
                    }
                    let count = u32_at(bytes, block + 4)? as usize;
                    let block_size = u32_at(bytes, block + 8)? as usize;
                    let block_end = block
                        .checked_add(block_size)
                        .filter(|end| *end <= limit && block_size >= 12)
                        .ok_or("Invalid CodeView file block size")?;
                    let lines_end = count
                        .checked_mul(8)
                        .and_then(|n| (block + 12).checked_add(n))
                        .filter(|end| *end <= block_end)
                        .ok_or("Invalid CodeView line count")?;
                    for at in (block + 16..lines_end).step_by(8) {
                        let value = u32_at(bytes, at)?;
                        if value & 0x00ff_ffff == PLACEHOLDER {
                            bytes[at..at + 4].copy_from_slice(
                                &((value & 0xff00_0000) | NEVER_STEP_INTO).to_le_bytes(),
                            );
                            changed = true;
                        }
                    }
                    block = block_end;
                }
            }
            subsection = start + ((limit - start + 3) & !3);
        }
    }
    Ok(changed)
}
pub(super) fn preserve_step_over(path: &Path) -> Result<(), String> {
    let mut bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    if rewrite(&mut bytes)? {
        std::fs::write(path, bytes).map_err(|e| e.to_string())?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_reserved_line_numbers_are_rewritten_in_unaligned_sections() {
        let mut bytes = vec![0u8; 107];
        bytes[2..4].copy_from_slice(&1u16.to_le_bytes());
        bytes[20..28].copy_from_slice(b".debug$S");
        bytes[36..40].copy_from_slice(&44u32.to_le_bytes());
        bytes[40..44].copy_from_slice(&63u32.to_le_bytes());
        for (at, value) in [
            (63, 4u32),
            (67, 0xf2),
            (71, 32),
            (91, 1),
            (95, 20),
            (103, 0x8000_0000 | PLACEHOLDER),
        ] {
            bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
        let original = bytes.clone();
        assert!(rewrite(&mut bytes).unwrap());
        assert_eq!(u32_at(&bytes, 103).unwrap(), 0x8000_0000 | NEVER_STEP_INTO);
        bytes[103..107].copy_from_slice(&original[103..107]);
        assert_eq!(bytes, original);
        bytes[91..95].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(rewrite(&mut bytes).is_err());
    }
    #[test]
    fn malformed_objects_are_rejected() {
        assert!(rewrite(&mut []).is_err());
    }
}
