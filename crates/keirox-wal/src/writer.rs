//! Segment WAL engine implementations per `KEI-DES-030`.

use keirox_core::error::Result;
use keirox_core::model::{Offset, StreamId};
use keirox_core::traits::StorageEngine;
use std::collections::HashMap;

/// In-memory WAL engine implementation for tests and prototype verification.
#[derive(Debug, Default)]
pub struct InMemoryWalEngine {
    streams: HashMap<StreamId, Vec<u8>>,
    offsets: HashMap<StreamId, Offset>,
}

impl InMemoryWalEngine {
    /// Create a new in-memory WAL engine.
    pub fn new() -> Self {
        Self::default()
    }
}

impl StorageEngine for InMemoryWalEngine {
    fn append_batch(&mut self, stream_id: StreamId, batch: &[u8]) -> Result<Offset> {
        let entry = self.streams.entry(stream_id).or_default();
        entry.extend_from_slice(batch);

        let current_offset = self.offsets.entry(stream_id).or_insert(0);
        let assigned_offset = *current_offset;
        *current_offset += 1;

        Ok(assigned_offset)
    }

    fn read_records(
        &self,
        stream_id: StreamId,
        _start_offset: Offset,
        _max_records: usize,
    ) -> Result<Vec<u8>> {
        let bytes = self.streams.get(&stream_id).cloned().unwrap_or_default();
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framing::BatchHeader;

    #[test]
    fn test_in_memory_wal_engine_append_and_read() {
        let mut engine = InMemoryWalEngine::new();
        let stream = StreamId([0xEE; 16]);

        let batch_header = BatchHeader::new(0, 128, 1, 0, 0, 1000, 0);
        let header_bytes = batch_header.to_bytes();

        let offset = engine.append_batch(stream, &header_bytes).unwrap();
        assert_eq!(offset, 0);

        let _read_bytes = engine.read_records(stream, 0, 10).unwrap();
    }
}

#[cfg(target_os = "linux")]
pub use uring_engine::IoUringWalEngine;

#[cfg(target_os = "linux")]
pub mod uring_engine {
    use io_uring::{opcode, IoUring};
    use keirox_core::error::{KeiroxError, Result};
    use keirox_core::model::{Offset, StreamId};
    use keirox_core::traits::StorageEngine;
    use rustix::fs::{open, Mode, OFlags};
    use std::collections::HashMap;
    use std::os::fd::{AsRawFd, OwnedFd};
    use std::path::PathBuf;

    /// io_uring WAL engine implementation with O_DIRECT
    pub struct IoUringWalEngine {
        dir: PathBuf,
        ring: IoUring,
        files: HashMap<StreamId, OwnedFd>,
        offsets: HashMap<StreamId, Offset>,
    }

    impl IoUringWalEngine {
        /// Create a new io_uring WAL engine
        pub fn new(dir: PathBuf) -> Result<Self> {
            std::fs::create_dir_all(&dir).map_err(|e| KeiroxError::StorageIo(e))?;
            let ring = IoUring::new(256).map_err(|e| KeiroxError::StorageIo(e))?;
            Ok(Self {
                dir,
                ring,
                files: HashMap::new(),
                offsets: HashMap::new(),
            })
        }

        fn get_or_open_file(&mut self, stream_id: StreamId) -> Result<&OwnedFd> {
            if !self.files.contains_key(&stream_id) {
                let mut path = self.dir.clone();
                let filename = format!("{:032x}.wal", u128::from_be_bytes(stream_id.0));
                path.push(filename);

                let flags =
                    OFlags::CREATE | OFlags::RDWR | OFlags::APPEND | OFlags::DIRECT | OFlags::DSYNC;
                let mode = Mode::from_raw_mode(0o644);
                let fd = open(&path, flags, mode)
                    .map_err(|e| KeiroxError::StorageIo(std::io::Error::from(e)))?;

                self.files.insert(stream_id, fd);
            }
            Ok(self.files.get(&stream_id).unwrap())
        }
    }

    impl StorageEngine for IoUringWalEngine {
        fn append_batch(&mut self, stream_id: StreamId, batch: &[u8]) -> Result<Offset> {
            let fd = self.get_or_open_file(stream_id)?.as_raw_fd();

            // Align buffer to 4096 bytes for O_DIRECT
            let layout = std::alloc::Layout::from_size_align(batch.len(), 4096).unwrap();
            let buf_ptr = unsafe { std::alloc::alloc(layout) };
            if buf_ptr.is_null() {
                return Err(KeiroxError::StorageIo(std::io::Error::new(
                    std::io::ErrorKind::OutOfMemory,
                    "Memory allocation failed",
                )));
            }
            unsafe {
                std::ptr::copy_nonoverlapping(batch.as_ptr(), buf_ptr, batch.len());
            }

            let entry = opcode::Write::new(io_uring::types::Fd(fd), buf_ptr, batch.len() as _)
                .offset(-1) // append mode
                .build()
                .user_data(1);

            unsafe {
                self.ring.submission().push(&entry).map_err(|e| {
                    KeiroxError::StorageIo(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e.to_string(),
                    ))
                })?;
            }
            self.ring
                .submit_and_wait(1)
                .map_err(|e| KeiroxError::StorageIo(e))?;

            let cqe = self.ring.completion().next().ok_or_else(|| {
                KeiroxError::StorageIo(std::io::Error::new(std::io::ErrorKind::Other, "No CQE"))
            })?;

            unsafe {
                std::alloc::dealloc(buf_ptr, layout);
            }

            if cqe.result() < 0 {
                return Err(KeiroxError::StorageIo(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("io_uring write failed: {}", cqe.result()),
                )));
            }

            let current_offset = self.offsets.entry(stream_id).or_insert(0);
            let assigned_offset = *current_offset;
            *current_offset += 1;

            Ok(assigned_offset)
        }

        fn read_records(
            &self,
            _stream_id: StreamId,
            _start_offset: Offset,
            _max_records: usize,
        ) -> Result<Vec<u8>> {
            Err(KeiroxError::StorageIo(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "Direct I/O read unimplemented",
            )))
        }
    }
}
