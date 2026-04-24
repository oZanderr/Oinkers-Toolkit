//! WAV (RIFF) parser and canonical WAV writer.

use super::pcm::{ConvertError, Result};

pub(super) struct WavInfo {
    pub channels: u16,
    pub sample_rate: u32,
    pub bits_per_sample: u16,
    /// Byte offset where the raw PCM data begins.
    pub data_offset: usize,
    /// Size of the raw PCM data in bytes.
    pub data_size: u32,
}

#[inline]
fn read_u16_le(b: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([b[off], b[off + 1]])
}

#[inline]
fn read_u32_le(b: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

/// Walks the RIFF/WAV chunk structure and extracts format + data info.
///
/// Used for both real WAV files and Wwise WEM containers, which share the
/// RIFF chunk layout (fmt + data sub-chunks).
pub(super) fn parse_riff(data: &[u8]) -> Result<WavInfo> {
    if data.len() < 12 {
        return Err(ConvertError::InvalidWav(
            "file too small for RIFF header".into(),
        ));
    }
    if &data[0..4] != b"RIFF" {
        return Err(ConvertError::InvalidWav("missing RIFF tag".into()));
    }
    if &data[8..12] != b"WAVE" {
        return Err(ConvertError::InvalidWav("missing WAVE form type".into()));
    }

    let mut channels: Option<u16> = None;
    let mut sample_rate: Option<u32> = None;
    let mut bits_per_sample: Option<u16> = None;
    let mut data_offset: Option<usize> = None;
    let mut data_size: Option<u32> = None;

    let mut pos = 12usize;

    while pos + 8 <= data.len() {
        let chunk_id = &data[pos..pos + 4];
        let chunk_size = read_u32_le(data, pos + 4) as usize;
        let chunk_data_start = pos + 8;

        if chunk_id == b"fmt " {
            if chunk_size < 16 {
                return Err(ConvertError::InvalidWav("fmt chunk too small".into()));
            }
            let format_tag = read_u16_le(data, chunk_data_start);
            // 1 = PCM, 0xFFFE = WAVE_FORMAT_EXTENSIBLE (PCM sub-type)
            if format_tag != 1 && format_tag != 0xFFFE {
                return Err(ConvertError::UnsupportedFormat(format!(
                    "format tag {format_tag:#06X} is not PCM"
                )));
            }
            channels = Some(read_u16_le(data, chunk_data_start + 2));
            sample_rate = Some(read_u32_le(data, chunk_data_start + 4));
            bits_per_sample = Some(read_u16_le(data, chunk_data_start + 14));
        } else if chunk_id == b"data" {
            data_offset = Some(chunk_data_start);
            data_size = Some(chunk_size as u32);
        }

        // Advance to next chunk (sizes are word-aligned in RIFF)
        pos = chunk_data_start + ((chunk_size + 1) & !1);
    }

    Ok(WavInfo {
        channels: channels.ok_or_else(|| ConvertError::InvalidWav("no fmt chunk found".into()))?,
        sample_rate: sample_rate
            .ok_or_else(|| ConvertError::InvalidWav("no fmt chunk found".into()))?,
        bits_per_sample: bits_per_sample
            .ok_or_else(|| ConvertError::InvalidWav("no fmt chunk found".into()))?,
        data_offset: data_offset
            .ok_or_else(|| ConvertError::InvalidWav("no data chunk found".into()))?,
        data_size: data_size
            .ok_or_else(|| ConvertError::InvalidWav("no data chunk found".into()))?,
    })
}

/// Emit a canonical 44-byte PCM WAV file from raw PCM bytes plus format metadata.
pub(super) fn write_canonical_wav(
    pcm: &[u8],
    channels: u16,
    sample_rate: u32,
    bits_per_sample: u16,
) -> Vec<u8> {
    let bytes_per_sample = bits_per_sample as u32 / 8;
    let byte_rate = sample_rate * channels as u32 * bytes_per_sample;
    let block_align = channels * bytes_per_sample as u16;
    let data_size = pcm.len() as u32;
    let riff_size = 36 + data_size;

    let mut out = Vec::with_capacity(44 + data_size as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&riff_size.to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM format
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&bits_per_sample.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_size.to_le_bytes());
    out.extend_from_slice(pcm);
    out
}
