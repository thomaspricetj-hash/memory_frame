// src/storage/kv_store.rs
//
// KvStore is a crash-safe, corruption-aware, atomic key-value store
// for MemoryFrame persistence. Uses CBOR for frame serialization
// (via serialize_memory_frame / deserialize_frame) and a simple CRC32
// prefix for payload integrity.

use anyhow::{Result, anyhow, Context};
use sled::{Db, IVec};
use uuid::Uuid;

use crate::storage::{serialize_memory_frame, deserialize_frame, frame_record_to_memory_frame};
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
        let db = sled::open(path).context("opening sled database failed")?;
        Ok(Self { db })
    }

    /// Save a frame atomically.
    /// - convert MemoryFrame -> FrameRecord and serialize (CBOR)
    /// - compute checksum
    /// - store as binary blob with 4-byte CRC32 prefix
    pub fn save(&self, frame: &MemoryFrame) -> Result<()> {
        // Serialize MemoryFrame into canonical FrameRecord bytes (CBOR)
        let bytes = serialize_memory_frame(frame).context("serializing MemoryFrame to FrameRecord failed")?;

        // Compute CRC32 checksum over the serialized bytes
        let checksum = crc32fast::hash(&bytes);
        let mut payload = checksum.to_le_bytes().to_vec();
        payload.extend_from_slice(&bytes);

        // Atomic insert
        self.db.insert(frame.id.as_bytes(), IVec::from(payload)).context("sled insert failed")?;
        self.db.flush().context("sled flush failed")?;

        Ok(())
    }

    /// Load a frame by ID.
    /// - fetch binary blob
    /// - verify checksum
    /// - deserialize FrameRecord (CBOR)
    /// - reconstruct MemoryFrame
    pub fn load(&self, id: Uuid) -> Result<MemoryFrame> {
        let raw = self
            .db
            .get(id.as_bytes())
            .context("sled get failed")?
            .ok_or_else(|| anyhow!("KvStore: frame {} not found", id))?;

        let bytes = raw.as_ref();

        // Extract checksum (4 bytes)
        if bytes.len() < 4 {
            return Err(anyhow!("KvStore: corrupted payload (too small)"));
        }

        let stored_checksum = u32::from_le_bytes(bytes[0..4].try_into().map_err(|e| anyhow!(e))?);
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

        // Deserialize FrameRecord (CBOR)
        let record: crate::storage::FrameRecord =
            deserialize_frame(data).context("deserializing FrameRecord from CBOR failed")?;

        // Reconstruct MemoryFrame from FrameRecord
        let mut frame = frame_record_to_memory_frame(record);

        // Ensure the returned MemoryFrame carries the requested id (store key is authoritative)
        frame.id = id;

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
        self.db.flush().context("sled flush failed")?;
        Ok(())
    }
}







