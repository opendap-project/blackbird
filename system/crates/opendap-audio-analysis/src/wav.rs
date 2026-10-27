//! RIFF WAVE, read and written: integer PCM (format tag 1) and IEEE float (3), and
//! WAVE_FORMAT_EXTENSIBLE (0xFFFE) for more than two channels or more than 16 bits, as Microsoft's
//! WAVEFORMATEXTENSIBLE specification requires. 8-bit WAVE is unsigned; wider integers are
//! signed; all little-endian. A capture streams its data after [`write_header`] and calls
//! [`finalize`] with the final size.

use std::io::{self, Read, Seek, SeekFrom, Write};

use crate::Format;

const PCM: u16 = 1;
const FLOAT: u16 = 3;
const EXTENSIBLE: u16 = 0xFFFE;
/// The subformat GUID's tail after its first two bytes (the format tag): KSDATAFORMAT_SUBTYPE_*.
const GUID_TAIL: [u8; 14] = [
    0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71,
];

fn extensible(f: &Format) -> bool {
    f.channels > 2 || f.bits > 16
}

/// The header's length for `format`: 44 bytes, or 68 with the extensible format chunk.
pub fn header_len(format: &Format) -> u64 {
    if extensible(format) { 68 } else { 44 }
}

fn invalid(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.to_string())
}

/// The header for `data_bytes` of `format` (signed little-endian integers above 8 bits, unsigned
/// 8-bit, or little-endian float).
pub fn write_header(w: &mut impl Write, format: &Format, data_bytes: u32) -> io::Result<()> {
    let valid =
        format.bits > 8 && format.signed || format.bits == 8 && !format.signed || format.float;
    if !valid || format.big_endian || !format.is_supported() {
        return Err(invalid(
            "WAVE carries little-endian signed integers (unsigned at 8 bits) or floats",
        ));
    }
    let ext = extensible(format);
    let tag = if format.float { FLOAT } else { PCM };
    let block = format.bytes_per_frame() as u16;
    let fmt_len: u32 = if ext { 40 } else { 16 };
    let riff = 4 + 8 + fmt_len + 8 + data_bytes;
    w.write_all(b"RIFF")?;
    w.write_all(&riff.to_le_bytes())?;
    w.write_all(b"WAVEfmt ")?;
    w.write_all(&fmt_len.to_le_bytes())?;
    w.write_all(&(if ext { EXTENSIBLE } else { tag }).to_le_bytes())?;
    w.write_all(&u16::from(format.channels).to_le_bytes())?;
    w.write_all(&format.rate.to_le_bytes())?;
    w.write_all(&(format.rate * u32::from(block)).to_le_bytes())?;
    w.write_all(&block.to_le_bytes())?;
    w.write_all(&u16::from(format.bits).to_le_bytes())?;
    if ext {
        w.write_all(&22u16.to_le_bytes())?;
        w.write_all(&u16::from(format.bits).to_le_bytes())?; // valid bits
        w.write_all(&0u32.to_le_bytes())?; // channel mask: unspecified
        w.write_all(&tag.to_le_bytes())?;
        w.write_all(&GUID_TAIL)?;
    }
    w.write_all(b"data")?;
    w.write_all(&data_bytes.to_le_bytes())
}

/// Rewrites the header of a streamed file with its final data size, and returns to its end.
pub fn finalize(
    file: &mut (impl Write + Seek),
    format: &Format,
    data_bytes: u32,
) -> io::Result<()> {
    file.seek(SeekFrom::Start(0))?;
    write_header(file, format, data_bytes)?;
    file.seek(SeekFrom::End(0))?;
    Ok(())
}

fn u16_at(b: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([b[i], b[i + 1]])
}

fn u32_at(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
}

/// A WAVE file's format and data. Chunks other than `fmt ` and `data` are skipped.
pub fn read(r: &mut impl Read) -> io::Result<(Format, Vec<u8>)> {
    let mut all = Vec::new();
    r.read_to_end(&mut all)?;
    if all.len() < 12 || &all[..4] != b"RIFF" || &all[8..12] != b"WAVE" {
        return Err(invalid("not a RIFF WAVE file"));
    }
    let (mut at, mut format, mut data) = (12usize, None, None);
    // Checked: a chunk claiming the most a size can say leaves `at` saturated, never wrapped.
    while at.checked_add(8).is_some_and(|end| end <= all.len()) {
        let (id, len) = (&all[at..at + 4], u32_at(&all, at + 4) as usize);
        let body = at + 8;
        // A chunk may claim more than the file holds: only what is there is read.
        let end = body.saturating_add(len).min(all.len());
        match id {
            b"fmt " => {
                let b = &all[body..end];
                if b.len() < 16 {
                    return Err(invalid("a truncated fmt chunk"));
                }
                let mut tag = u16_at(b, 0);
                if tag == EXTENSIBLE && b.len() >= 26 {
                    tag = u16_at(b, 24);
                }
                let bits = u16_at(b, 14) as u8;
                let float = match tag {
                    PCM => false,
                    FLOAT => true,
                    _ => return Err(invalid("a WAVE format other than PCM or float")),
                };
                let channels =
                    u8::try_from(u16_at(b, 2)).map_err(|_| invalid("more than 255 channels"))?;
                format = Some(Format {
                    rate: u32_at(b, 4),
                    channels,
                    bits,
                    signed: float || bits > 8,
                    float,
                    big_endian: false,
                });
            }
            b"data" => data = Some(all[body..end].to_vec()),
            _ => {}
        }
        at = body.saturating_add(len).saturating_add(len & 1);
    }
    match (format, data) {
        (Some(f), _) if !f.is_supported() => {
            Err(invalid("a WAVE sample format this crate does not read"))
        }
        (Some(f), Some(d)) => Ok((f, d)),
        _ => Err(invalid("a WAVE file without its format or data")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(format: Format, data: Vec<u8>) {
        let mut file = Vec::new();
        write_header(&mut file, &format, data.len() as u32).unwrap();
        assert_eq!(file.len() as u64, header_len(&format));
        file.extend(&data);
        let (f, d) = read(&mut file.as_slice()).unwrap();
        assert_eq!((f, d), (format, data));
    }

    #[test]
    fn plain_and_extensible_round_trip() {
        round_trip(Format::int(44_100, 2, 16), vec![1, 2, 3, 4]);
        round_trip(Format::int(96_000, 2, 24), vec![1, 2, 3, 4, 5, 6]);
        round_trip(Format::int(48_000, 6, 32), vec![0; 24]);
        round_trip(Format::f32(48_000, 2), vec![0; 8]);
        round_trip(
            Format {
                rate: 8000,
                channels: 1,
                bits: 8,
                signed: false,
                float: false,
                big_endian: false,
            },
            vec![128],
        );
    }

    #[test]
    fn a_streamed_file_is_finalized() {
        let f = Format::int(48_000, 2, 16);
        let mut file = io::Cursor::new(Vec::new());
        write_header(&mut file, &f, 0).unwrap();
        file.write_all(&[7; 400]).unwrap();
        finalize(&mut file, &f, 400).unwrap();
        let bytes = file.into_inner();
        assert_eq!(u32_at(&bytes, 4), 36 + 400);
        assert_eq!(read(&mut bytes.as_slice()).unwrap().1.len(), 400);
    }

    #[test]
    fn what_wave_cannot_carry_is_refused() {
        let mut out = Vec::new();
        assert!(
            write_header(
                &mut out,
                &Format {
                    big_endian: true,
                    ..Format::int(48_000, 2, 16)
                },
                0
            )
            .is_err()
        );
        assert!(read(&mut &b"RIFX0000WAVE"[..]).is_err());
    }

    #[test]
    fn malformed_files_are_errors_not_panics() {
        let mut file = Vec::new();
        write_header(&mut file, &Format::int(48_000, 2, 16), 4).unwrap();
        // Every truncation of a valid file, and a fmt chunk that claims more than it holds.
        for n in 0..file.len() {
            let _ = read(&mut &file[..n]);
        }
        let mut short = b"RIFF\x10\x00\x00\x00WAVEfmt \x10\x00\x00\x00\x01\x00".to_vec();
        short.truncate(22);
        assert!(read(&mut short.as_slice()).is_err());
        let mut huge = file.clone();
        huge[40..44].copy_from_slice(&u32::MAX.to_le_bytes()); // data claims 4 GiB
        assert_eq!(read(&mut huge.as_slice()).unwrap().1.len(), 0);
        // A chunk after it whose size would carry the position past usize::MAX on 32-bit targets.
        let mut odd = file.clone();
        odd.extend(b"junk\xff\xff\xff\xff");
        assert!(read(&mut odd.as_slice()).is_ok());
    }
}
