//! Physical WAL segment file management and replay per `KEI-DES-030`.

use crate::framing::{BatchHeader, RecordEntry, SegmentFooter, SegmentHeader, BATCH_MAGIC};
use keirox_core::error::{KeiroxError, Result};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::mem::size_of;
use std::path::{Path, PathBuf};

/// Page alignment size in bytes (4096 bytes per `KEI-DES-030` §8.1).
pub const PAGE_SIZE: usize = 4096;

/// Default segment pre-allocation size (64 MB per `KEI-DES-030` §4.1).
pub const DEFAULT_SEGMENT_SIZE: u64 = 64 * 1024 * 1024;

/// Replayed batch containing verified header, record entries, and raw payload bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayedBatch {
    /// Batch frame header.
    pub header: BatchHeader,
    /// Record entries pointing into payload.
    pub records: Vec<RecordEntry>,
    /// Raw uncompressed payload bytes.
    pub payload: Vec<u8>,
}

/// Physical WAL Segment file writer.
pub struct SegmentFile {
    _path: PathBuf,
    file: File,
    header: SegmentHeader,
    write_cursor: u64,
    batch_count: u32,
    record_count: u64,
    sealed: bool,
}

impl SegmentFile {
    /// Create and initialize a new WAL segment file on disk.
    pub fn create<P: AsRef<Path>>(
        path: P,
        segment_id: u64,
        volume_id: u32,
        node_id: u32,
        physical_seq_start: u64,
    ) -> Result<Self> {
        let path_buf = path.as_ref().to_path_buf();
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path_buf)?;

        let header = SegmentHeader::new(segment_id, volume_id, node_id, physical_seq_start);
        // SAFETY: `header` is a valid #[repr(C)] POD struct with known size and memory layout.
        let header_bytes = unsafe {
            std::slice::from_raw_parts(
                &header as *const SegmentHeader as *const u8,
                size_of::<SegmentHeader>(),
            )
        };

        file.write_all(header_bytes)?;
        file.flush()?;

        Ok(Self {
            _path: path_buf,
            file,
            header,
            write_cursor: PAGE_SIZE as u64,
            batch_count: 0,
            record_count: 0,
            sealed: false,
        })
    }

    /// Append a batch frame to the segment file with 4096-byte page alignment padding.
    pub fn append_batch(
        &mut self,
        header: &BatchHeader,
        records: &[RecordEntry],
        payload: &[u8],
    ) -> Result<u64> {
        if self.sealed {
            return Err(KeiroxError::LogMutationViolation(
                "Cannot append to a sealed segment".into(),
            ));
        }

        if !header.is_valid() {
            return Err(KeiroxError::Internal("Invalid batch header CRC".into()));
        }

        self.file.seek(SeekFrom::Start(self.write_cursor))?;
        let batch_start_offset = self.write_cursor;

        // 1. Write 128-byte BatchHeader
        // SAFETY: `header` is a valid #[repr(C)] POD struct with known size.
        let header_bytes = unsafe {
            std::slice::from_raw_parts(
                header as *const BatchHeader as *const u8,
                size_of::<BatchHeader>(),
            )
        };
        self.file.write_all(header_bytes)?;

        // 2. Write RecordEntries
        for record in records {
            if !record.is_valid() {
                return Err(KeiroxError::Internal("Invalid record entry CRC".into()));
            }
            // SAFETY: `record` is a valid #[repr(C, packed)] POD struct with known size.
            let record_bytes = unsafe {
                std::slice::from_raw_parts(
                    record as *const RecordEntry as *const u8,
                    size_of::<RecordEntry>(),
                )
            };
            self.file.write_all(record_bytes)?;
        }

        // 3. Write Payload Block
        self.file.write_all(payload)?;

        // 4. Calculate 4096-byte page padding
        let total_written =
            size_of::<BatchHeader>() + std::mem::size_of_val(records) + payload.len();
        let padded_size = total_written.div_ceil(PAGE_SIZE) * PAGE_SIZE;
        let padding_needed = padded_size - total_written;

        if padding_needed > 0 {
            let zero_pad = vec![0u8; padding_needed];
            self.file.write_all(&zero_pad)?;
        }

        self.file.flush()?;

        self.write_cursor += padded_size as u64;
        self.batch_count += 1;
        self.record_count += records.len() as u64;

        Ok(batch_start_offset)
    }

    /// Seal segment and append 4096-byte SegmentFooter.
    pub fn seal(&mut self, sealed_timestamp_ns: u64) -> Result<()> {
        if self.sealed {
            return Ok(());
        }

        let footer = SegmentFooter::new(
            self.header.segment_id,
            self.header.physical_seq_start + self.record_count,
            self.batch_count,
            self.record_count,
            sealed_timestamp_ns,
        );

        // SAFETY: `footer` is a valid #[repr(C)] POD struct with known size and memory layout.
        let footer_bytes = unsafe {
            std::slice::from_raw_parts(
                &footer as *const SegmentFooter as *const u8,
                size_of::<SegmentFooter>(),
            )
        };

        self.file.seek(SeekFrom::Start(self.write_cursor))?;
        self.file.write_all(footer_bytes)?;
        self.file.flush()?;

        self.sealed = true;
        Ok(())
    }

    /// Return segment identifier.
    pub fn segment_id(&self) -> u64 {
        self.header.segment_id
    }

    /// Return total recorded batches.
    pub fn batch_count(&self) -> u32 {
        self.batch_count
    }

    /// Return total recorded records.
    pub fn record_count(&self) -> u64 {
        self.record_count
    }
}

/// Physical WAL Segment file replay reader.
pub struct SegmentReader {
    file: File,
    header: SegmentHeader,
}

impl SegmentReader {
    /// Open and validate a segment file on disk.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut file = File::open(path)?;
        let mut header_buf = vec![0u8; size_of::<SegmentHeader>()];
        file.read_exact(&mut header_buf)?;

        // SAFETY: `header_buf` has length size_of::<SegmentHeader>().
        let header: SegmentHeader =
            unsafe { std::ptr::read_unaligned(header_buf.as_ptr() as *const SegmentHeader) };

        if !header.is_valid() {
            return Err(KeiroxError::Internal(
                "SegmentHeader failed CRC or magic validation".into(),
            ));
        }

        Ok(Self { file, header })
    }

    /// Return the verified SegmentHeader.
    pub fn header(&self) -> &SegmentHeader {
        &self.header
    }

    /// Replay all valid batches and return them in physical sequence.
    pub fn replay_batches(&mut self) -> Result<Vec<ReplayedBatch>> {
        self.file.seek(SeekFrom::Start(PAGE_SIZE as u64))?;
        let mut batches = Vec::new();

        loop {
            let mut header_buf = vec![0u8; size_of::<BatchHeader>()];
            match self.file.read_exact(&mut header_buf) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e.into()),
            }

            // SAFETY: `header_buf` has length size_of::<BatchHeader>().
            let batch_header: BatchHeader =
                unsafe { std::ptr::read_unaligned(header_buf.as_ptr() as *const BatchHeader) };

            // Check if we hit unwritten zero padding or footer
            if batch_header.magic != BATCH_MAGIC {
                break;
            }

            if !batch_header.is_valid() {
                return Err(KeiroxError::Internal(
                    "BatchHeader failed CRC validation during replay".into(),
                ));
            }

            let mut records = Vec::with_capacity(batch_header.record_count as usize);
            for _ in 0..batch_header.record_count {
                let mut record_buf = vec![0u8; size_of::<RecordEntry>()];
                self.file.read_exact(&mut record_buf)?;
                // SAFETY: `record_buf` has length size_of::<RecordEntry>().
                let record: RecordEntry =
                    unsafe { std::ptr::read_unaligned(record_buf.as_ptr() as *const RecordEntry) };
                if !record.is_valid() {
                    return Err(KeiroxError::Internal(
                        "RecordEntry failed CRC validation during replay".into(),
                    ));
                }
                records.push(record);
            }

            let payload_len = batch_header
                .total_batch_size
                .saturating_sub(size_of::<BatchHeader>() as u32)
                .saturating_sub((records.len() * size_of::<RecordEntry>()) as u32);
            let mut payload = vec![0u8; payload_len as usize];
            if payload_len > 0 {
                self.file.read_exact(&mut payload)?;
            }

            // Skip page padding to next 4096-byte boundary
            let total_read = size_of::<BatchHeader>()
                + std::mem::size_of_val(records.as_slice())
                + payload.len();
            let padded_size = total_read.div_ceil(PAGE_SIZE) * PAGE_SIZE;
            let padding = padded_size - total_read;
            if padding > 0 {
                self.file.seek(SeekFrom::Current(padding as i64))?;
            }

            batches.push(ReplayedBatch {
                header: batch_header,
                records,
                payload,
            });
        }

        Ok(batches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framing::WAL_FORMAT_VERSION;
    use tempfile::tempdir;

    #[test]
    fn test_segment_create_append_replay_and_seal() {
        let dir = tempdir().unwrap();
        let segment_path = dir.path().join("0000000000000001.kwal");

        // 1. Create and append
        {
            let mut segment = SegmentFile::create(&segment_path, 1, 10, 100, 0).unwrap();
            assert_eq!(segment.segment_id(), 1);

            let stream = [0x77; 16];
            let records = vec![
                RecordEntry::new(stream, 0, 0, 16, 0),
                RecordEntry::new(stream, 1, 16, 16, 0),
            ];
            let payload = vec![0xAB; 32];
            let total_batch_size = (size_of::<BatchHeader>()
                + (records.len() * size_of::<RecordEntry>())
                + payload.len()) as u32;

            let header = BatchHeader::new(0, total_batch_size, 2, 0, 1, 1700000000, 0);

            segment
                .append_batch(&header, &records, &payload)
                .expect("Append batch must succeed");
            assert_eq!(segment.batch_count(), 1);
            assert_eq!(segment.record_count(), 2);

            segment.seal(1700000100).expect("Seal must succeed");
        }

        // 2. Replay and verify
        {
            let mut reader = SegmentReader::open(&segment_path).unwrap();
            assert_eq!(reader.header().segment_id, 1);
            assert_eq!(reader.header().format_version, WAL_FORMAT_VERSION);

            let batches = reader.replay_batches().unwrap();
            assert_eq!(batches.len(), 1);
            let batch = &batches[0];
            assert_eq!(batch.header.record_count, 2);
            assert_eq!(batch.records.len(), 2);
            assert_eq!(batch.records[0].logical_offset(), 0);
            assert_eq!(batch.records[1].logical_offset(), 1);
            assert_eq!(batch.payload.len(), 32);
        }
    }
}
