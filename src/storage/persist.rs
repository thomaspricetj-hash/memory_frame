// src/storage/persist.rs
//
// Persistence utilities for the hybrid cognitive memory engine.
// Provides atomic save/load for:
// - CognitiveFrame
// - DeltaHistory
// - AnchorFrame
//
// Uses JSON for readability and CBOR for compact binary storage.
// You can expand this later with your folding layer.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Serialize, Deserialize};

use crate::frame::cognitive_frame::CognitiveFrame;
use crate::storage::versioning::history::DeltaHistory;
use crate::storage::versioning::anchor::AnchorFrame;

/// Errors for persistence operations.
#[derive(Debug)]
pub enum PersistError {
    Io(std::io::Error),
    SerdeJson(serde_json::Error),
    Cbor(serde_cbor::Error),
    MissingFile(PathBuf),
}

impl From<std::io::Error> for PersistError {
    fn from(e: std::io::Error) -> Self {
        PersistError::Io(e)
    }
}

impl From<serde_json::Error> for PersistError {
    fn from(e: serde_json::Error) -> Self {
        PersistError::SerdeJson(e)
    }
}

impl From<serde_cbor::Error> for PersistError {
    fn from(e: serde_cbor::Error) -> Self {
        PersistError::Cbor(e)
    }
}

/// Ensure a directory exists.
fn ensure_dir(path: &Path) -> Result<(), PersistError> {
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
}

/// Atomic write helper.
fn atomic_write(path: &Path, data: &[u8]) -> Result<(), PersistError> {
    let tmp = path.with_extension("tmp");
    {
        let mut f = File::create(&tmp)?;
        f.write_all(data)?;
        f.flush()?;
    }
    fs::rename(tmp, path)?;
    Ok(())
}

/// Save a CognitiveFrame as JSON.
pub fn save_frame_json(frame: &CognitiveFrame, path: impl AsRef<Path>) -> Result<(), PersistError> {
    let path = path.as_ref();
    ensure_dir(path.parent().unwrap())?;

    let json = serde_json::to_vec_pretty(frame)?;
    atomic_write(path, &json)?;
    Ok(())
}

/// Load a CognitiveFrame from JSON.
pub fn load_frame_json(path: impl AsRef<Path>) -> Result<CognitiveFrame, PersistError> {
    let path = path.as_ref();

    if !path.exists() {
        return Err(PersistError::MissingFile(path.to_path_buf()));
    }

    let mut buf = Vec::new();
    File::open(path)?.read_to_end(&mut buf)?;
    let frame: CognitiveFrame = serde_json::from_slice(&buf)?;
    Ok(frame)
}

/// Save a DeltaHistory as binary (CBOR).
pub fn save_history_bin(history: &DeltaHistory, path: impl AsRef<Path>) -> Result<(), PersistError> {
    let path = path.as_ref();
    ensure_dir(path.parent().unwrap())?;

    let bin = serde_cbor::to_vec(history)?;
    atomic_write(path, &bin)?;
    Ok(())
}

/// Load a DeltaHistory from binary (CBOR).
pub fn load_history_bin(path: impl AsRef<Path>) -> Result<DeltaHistory, PersistError> {
    let path = path.as_ref();

    if !path.exists() {
        return Err(PersistError::MissingFile(path.to_path_buf()));
    }

    let mut buf = Vec::new();
    File::open(path)?.read_to_end(&mut buf)?;
    let history: DeltaHistory = serde_cbor::from_slice(&buf)?;
    Ok(history)
}

/// Save an AnchorFrame as JSON.
pub fn save_anchor_json(anchor: &AnchorFrame, path: impl AsRef<Path>) -> Result<(), PersistError> {
    let path = path.as_ref();
    ensure_dir(path.parent().unwrap())?;

    let json = serde_json::to_vec_pretty(anchor)?;
    atomic_write(path, &json)?;
    Ok(())
}

/// Load an AnchorFrame from JSON.
pub fn load_anchor_json(path: impl AsRef<Path>) -> Result<AnchorFrame, PersistError> {
    let path = path.as_ref();

    if !path.exists() {
        return Err(PersistError::MissingFile(path.to_path_buf()));
    }

    let mut buf = Vec::new();
    File::open(path)?.read_to_end(&mut buf)?;
    let anchor: AnchorFrame = serde_json::from_slice(&buf)?;
    Ok(anchor)
}




