// src/frame/slice.rs

use crate::frame::Grid;
use crate::layers::LayerId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{self, MapAccess, Visitor};
use std::fmt;

pub type SliceId = LayerId;

/// SliceData: externally tagged enum representation: {"kind": "...", "value": ...}
/// We implement custom Serialize/Deserialize so that the Semantic variant
/// (which contains serde_json::Value) roundtrips correctly with bincode.
/// For bincode compatibility we serialize the Semantic inner Value as a JSON string.
#[derive(Debug, Clone, PartialEq)]
pub enum SliceData {
    Visual(Vec<u8>),
    Semantic(serde_json::Value),
    Temporal(DateTime<Utc>),
    Emotional(f32),
    Relational(Vec<f32>),
    Declarative(String),
}

impl Serialize for SliceData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // We'll serialize as an externally tagged map: {"kind": "<Variant>", "value": <...>}
        use serde::ser::SerializeStruct;
        match self {
            SliceData::Visual(v) => {
                let mut st = serializer.serialize_struct("SliceData", 2)?;
                st.serialize_field("kind", "Visual")?;
                st.serialize_field("value", v)?;
                st.end()
            }
            SliceData::Semantic(val) => {
                // For compatibility with bincode (which doesn't implement deserialize_any),
                // serialize the JSON Value as a JSON string.
                let json_str = serde_json::to_string(val).map_err(serde::ser::Error::custom)?;
                let mut st = serializer.serialize_struct("SliceData", 2)?;
                st.serialize_field("kind", "Semantic")?;
                st.serialize_field("value", &json_str)?;
                st.end()
            }
            SliceData::Temporal(dt) => {
                let mut st = serializer.serialize_struct("SliceData", 2)?;
                st.serialize_field("kind", "Temporal")?;
                st.serialize_field("value", dt)?;
                st.end()
            }
            SliceData::Emotional(f) => {
                let mut st = serializer.serialize_struct("SliceData", 2)?;
                st.serialize_field("kind", "Emotional")?;
                st.serialize_field("value", f)?;
                st.end()
            }
            SliceData::Relational(vs) => {
                let mut st = serializer.serialize_struct("SliceData", 2)?;
                st.serialize_field("kind", "Relational")?;
                st.serialize_field("value", vs)?;
                st.end()
            }
            SliceData::Declarative(s) => {
                let mut st = serializer.serialize_struct("SliceData", 2)?;
                st.serialize_field("kind", "Declarative")?;
                st.serialize_field("value", s)?;
                st.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for SliceData {
    fn deserialize<D>(deserializer: D) -> Result<SliceData, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Expect a map with keys "kind" and "value"
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Kind,
            Value,
            // allow unknown fields to be ignored
            #[serde(other)]
            Other,
        }

        struct SliceDataVisitor;

        impl<'de> Visitor<'de> for SliceDataVisitor {
            type Value = SliceData;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct SliceData { kind: String, value: ... }")
            }

            fn visit_map<V>(self, mut map: V) -> Result<SliceData, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut kind_opt: Option<String> = None;
                // We'll capture the raw serde value for "value" and then interpret it
                // depending on the kind. To keep things flexible across formats,
                // we accept different representations:
                // - For Semantic: we expect a string (JSON text) and parse it.
                // - For other variants: we attempt to deserialize into the expected type.
                // To accomplish this, we use serde_json::Value as an intermediate for "value"
                // when necessary, but prefer direct deserialization where possible.

                // Because MapAccess doesn't let us peek keys easily, we read entries
                // and stash the "value" raw bytes via serde's Value when needed.
                // We'll use serde_value crate-like approach by deserializing the "value"
                // into serde_json::Value when we don't know the kind yet.
                //
                // Simpler approach: read both fields in any order; when we see "value",
                // store it as serde_json::Value via serde_json::Value::deserialize.
                let mut raw_value: Option<serde_json::Value> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "kind" => {
                            if kind_opt.is_some() {
                                return Err(de::Error::duplicate_field("kind"));
                            }
                            kind_opt = Some(map.next_value()?);
                        }
                        "value" => {
                            if raw_value.is_some() {
                                return Err(de::Error::duplicate_field("value"));
                            }
                            // Deserialize the "value" into serde_json::Value using the incoming format.
                            // This works for most serde formats (bincode, cbor, msgpack) because
                            // serde_json::Value implements Deserialize for those formats as well.
                            let v = map.next_value::<serde_json::Value>()?;
                            raw_value = Some(v);
                        }
                        _ => {
                            // Unknown field: skip it
                            let _ = map.next_value::<serde_json::Value>()?;
                        }
                    }
                }

                let kind = kind_opt.ok_or_else(|| de::Error::missing_field("kind"))?;
                let raw = raw_value.ok_or_else(|| de::Error::missing_field("value"))?;

                match kind.as_str() {
                    "Visual" => {
                        // Expect raw to be a sequence of bytes; try to deserialize into Vec<u8>
                        let vec: Vec<u8> = serde_json::from_value(raw).map_err(de::Error::custom)?;
                        Ok(SliceData::Visual(vec))
                    }
                    "Semantic" => {
                        // We serialized Semantic as a JSON string for bincode compatibility.
                        // Accept either:
                        //  - a JSON string containing the JSON text (preferred for bincode),
                        //  - or a direct JSON object (if the format supports deserialize_any).
                        match raw {
                            serde_json::Value::String(s) => {
                                let parsed: serde_json::Value =
                                    serde_json::from_str(&s).map_err(de::Error::custom)?;
                                Ok(SliceData::Semantic(parsed))
                            }
                            other => {
                                // If the incoming format provided a direct object, accept it.
                                Ok(SliceData::Semantic(other))
                            }
                        }
                    }
                    "Temporal" => {
                        let dt: DateTime<Utc> =
                            serde_json::from_value(raw).map_err(de::Error::custom)?;
                        Ok(SliceData::Temporal(dt))
                    }
                    "Emotional" => {
                        let f: f32 = serde_json::from_value(raw).map_err(de::Error::custom)?;
                        Ok(SliceData::Emotional(f))
                    }
                    "Relational" => {
                        let v: Vec<f32> = serde_json::from_value(raw).map_err(de::Error::custom)?;
                        Ok(SliceData::Relational(v))
                    }
                    "Declarative" => {
                        let s: String = serde_json::from_value(raw).map_err(de::Error::custom)?;
                        Ok(SliceData::Declarative(s))
                    }
                    other => Err(de::Error::unknown_variant(other, &["Visual","Semantic","Temporal","Emotional","Relational","Declarative"])),
                }
            }
        }

        // Deserialize the incoming map using our visitor
        const FIELDS: &'static [&'static str] = &["kind", "value"];
        deserializer.deserialize_struct("SliceData", FIELDS, SliceDataVisitor)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Slice {
    pub id: SliceId,
    pub grid: Grid,
    pub data: SliceData,
}

impl Slice {
    /// Create a new slice with a default 32×32 grid.
    pub fn new(id: SliceId, data: SliceData) -> Self {
        Self {
            id,
            grid: Grid::new(32, 32),
            data,
        }
    }

    /// Create a slice with a custom grid size.
    pub fn with_size(id: SliceId, data: SliceData, width: usize, height: usize) -> Self {
        Self {
            id,
            grid: Grid::new(width, height),
            data,
        }
    }

    pub fn example_with_id(id: SliceId) -> Self {
        let mut grid = Grid::new(8, 8);
        if let Some(cell_id) = grid.cell_id(1, 1) {
            if let Some(cell) = grid.get_cell_mut(cell_id) {
                cell.confidence = 0.9;
                cell.tags.push("example".to_string());
            }
        }
        if let Some(cell_id) = grid.cell_id(6, 6) {
            if let Some(cell) = grid.get_cell_mut(cell_id) {
                cell.confidence = 0.2;
                cell.tags.push("low".to_string());
            }
        }

        Self {
            id,
            grid,
            data: SliceData::Declarative("example slice".to_string()),
        }
    }

    pub fn example() -> Self {
        if let Some(id) = LayerId::from_str_fast("example") {
            Self::example_with_id(id)
        } else {
            panic!(
                "Slice::example() could not construct a LayerId. \
                 Implement Default for LayerId, provide a constructor, or use \
                 Slice::example_with_id(id) in tests."
            )
        }
    }

    /// Average confidence across all cells in this slice.
    pub fn average_confidence(&self) -> f32 {
        self.grid.average_confidence()
    }

    /// Extract the top tags from this slice.
    pub fn dominant_tags(&self) -> Vec<String> {
        self.grid.dominant_tags()
    }

    /// Count total cells.
    pub fn cell_count(&self) -> usize {
        self.grid.cell_count()
    }

    /// Return all high‑confidence cells above a threshold.
    pub fn high_confidence_cells(&self, threshold: f32) -> Vec<crate::frame::CellId> {
        self.grid.high_confidence_cells(threshold)
    }

    /// Return the strongest cell in the slice.
    pub fn strongest_cell(&self) -> Option<&crate::frame::Cell> {
        self.grid.strongest_cell()
    }

    /// Return the weakest cell in the slice.
    pub fn weakest_cell(&self) -> Option<&crate::frame::Cell> {
        self.grid.weakest_cell()
    }

    /// Return all tags sorted by frequency.
    pub fn tag_histogram(&self) -> Vec<(String, usize)> {
        self.grid.tag_histogram()
    }

    /// Update slice metadata (Semantic, Declarative, etc.)
    pub fn update_data(&mut self, new_data: SliceData) {
        self.data = new_data;
    }

    /// Diagonal slice signature using strongest + weakest cells only.
    pub fn diagonal_signature(&self) -> f32 {
        let w = self.grid_width() as f32;
        let h = self.grid_height() as f32;

        let cx = (w - 1.0) / 2.0;
        let cy = (h - 1.0) / 2.0;

        let mut score = 0.0;
        let mut count = 0.0;

        if let Some(cell) = self.strongest_cell() {
            let dx = (cell.id.x as f32 - cx).abs();
            let dy = (cell.id.y as f32 - cy).abs();
            let diag = 1.0 / (1.0 + (dx - dy).abs());
            score += diag * cell.confidence;
            count += 1.0;
        }

        if let Some(cell) = self.weakest_cell() {
            let dx = (cell.id.x as f32 - cx).abs();
            let dy = (cell.id.y as f32 - cy).abs();
            let diag = 1.0 / (1.0 + (dx - dy).abs());
            score += diag * cell.confidence;
            count += 1.0;
        }

        if count == 0.0 { 0.0 } else { score / count }
    }

    /// Diagonal semantic surface using tag histogram density.
    pub fn diagonal_semantic_surface(&self) -> f32 {
        let tags = self.tag_histogram();
        if tags.is_empty() {
            return 0.0;
        }

        let total_tags: usize = tags.iter().map(|(_, c)| *c).sum();
        let density = total_tags as f32 / self.cell_count().max(1) as f32;

        density.clamp(0.0, 1.5)
    }

    /// Diagonal confidence surface using average confidence and grid aspect ratio.
    pub fn diagonal_confidence_surface(&self) -> f32 {
        let avg = self.average_confidence();
        let w = self.grid_width() as f32;
        let h = self.grid_height() as f32;

        let aspect_diag = 1.0 / (1.0 + (w - h).abs());
        (avg * aspect_diag).clamp(0.0, 1.5)
    }

    /// Diagonal propagation weight: semantic + confidence blend.
    pub fn diagonal_propagation_weight(&self) -> f32 {
        let sem = self.diagonal_semantic_surface();
        let conf = self.diagonal_confidence_surface();
        ((sem + conf) / 2.0).clamp(0.25, 1.5)
    }

    /// Diagonal collapse influence: signature × confidence.
    pub fn diagonal_collapse_influence(&self) -> f32 {
        let sig = self.diagonal_signature();
        (sig * self.average_confidence()).clamp(0.1, 2.0)
    }

    /// Helper: grid width (field access, not method).
    fn grid_width(&self) -> usize {
        self.grid.width
    }

    /// Helper: grid height (field access, not method).
    fn grid_height(&self) -> usize {
        self.grid.height
    }

    /// Compute a compact harmonic signature for this slice using multi-scale pools and tags.
    pub fn harmonic_signature(&self) -> u64 {
        crate::frame::harmonics::harmonic_signature(&self.grid, &self.grid.collect_tags())
    }

    /// Initialize phase channels from legacy confidence values for all cells.
    pub fn init_phases_from_confidence(&mut self) {
        for c in &mut self.grid.cells {
            c.init_phases_from_confidence();
        }
    }
}









