use anyhow::{Result, anyhow};
use sled::{Db, IVec};
use uuid::Uuid;

use crate::storage::{FrameRecord, serialize_frame, deserialize_frame};
use crate::frame::MemoryFrame;

/// KvStore is a crash‑safe, corruption‑aware, atomic key‑value store
/// for MemoryFrame persistence.
pub struct KvStore {
    db: Db,
}

impl KvStore {
    /// Open or create the KV store at the given path.
    /// sled guarantees crash‑safe durability.
    pub fn open(path: &str) -> Result<Self> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    /// Save a frame atomically.
    /// - serialize frame
    /// - validate record
    /// - compute checksum
    /// - store as binary blob
    pub fn save(&self, frame: &MemoryFrame) -> Result<()> {
        let record = serialize_frame(frame)?;

        // Integrity check: record.id must match frame.id
        if record.id != frame.id {
            return Err(anyhow!(
                "KvStore: record/frame ID mismatch ({} != {})",
                record.id,
                frame.id
            ));
        }

        // Serialize to binary
        let bytes = bincode::serialize(&record)?;

        // Optional: add checksum prefix (future‑proof)
        let checksum = crc32fast::hash(&bytes);
        let mut payload = checksum.to_le_bytes().to_vec();
        payload.extend_from_slice(&bytes);

        // Atomic insert
        self.db.insert(frame.id.as_bytes(), IVec::from(payload))?;
        self.db.flush()?; // ensure durability

        Ok(())
    }

    /// Load a frame by ID.
    /// - fetch binary blob
    /// - verify checksum
    /// - deserialize FrameRecord
    /// - reconstruct MemoryFrame
    pub fn load(&self, id: Uuid) -> Result<MemoryFrame> {
        let raw = self
            .db
            .get(id.as_bytes())?
            .ok_or_else(|| anyhow!("KvStore: frame {} not found", id))?;

        let bytes = raw.as_ref();

        // Extract checksum
        if bytes.len() < 4 {
            return Err(anyhow!("KvStore: corrupted payload (too small)"));
        }

        let stored_checksum = u32::from_le_bytes(bytes[0..4].try_into()?);
        let data = &bytes[4..];

        let computed_checksum = crc32fast::hash(data);
        if stored_checksum != computed_checksum {
            return Err(anyhow!(
                "KvStore: checksum mismatch for frame {} (stored={}, computed={})",
                id,
                stored_checksum,
                computed_checksum
            ));
        }

        // Deserialize FrameRecord
        let record: FrameRecord = bincode::deserialize(data)?;

        // Reconstruct MemoryFrame
        let frame = deserialize_frame(record)?;

        // Final integrity check
        if frame.id != id {
            return Err(anyhow!(
                "KvStore: loaded frame has mismatched ID ({} != {})",
                frame.id,
                id
            ));
        }

        Ok(frame)
    }

    /// Check if a frame exists.
    pub fn exists(&self, id: Uuid) -> bool {
        self.db.contains_key(id.as_bytes())
    }

    /// Delete a frame safely.
    pub fn delete(&self, id: Uuid) -> bool {
        self.db.remove(id.as_bytes()).is_some()
    }

    /// Flush all pending writes.
    pub fn flush(&self) -> Result<()> {
        self.db.flush()?;
        Ok(())
    }
}
