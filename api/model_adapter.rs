use crate::frame::{MemoryFrame, SliceData};
use crate::layers::*;
use crate::api::ApiError;

pub struct ModelAdapter;

impl ModelAdapter {
    pub fn ingest_temporal(
        frame: &mut MemoryFrame,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), ApiError> {
        let temporal = TemporalLayer::encode(timestamp)?;
        frame.insert_slice(LayerId::Temporal, SliceData::Temporal(temporal));
        Ok(())
    }

    pub fn ingest_dynamic(
        frame: &mut MemoryFrame,
        layer: LayerId,
        payload: serde_json::Value,
    ) -> Result<(), ApiError> {

        let slice = match layer {
            LayerId::Visual => {
                let arr = payload
                    .as_array()
                    .ok_or_else(|| ApiError::AdapterError("visual payload must be array".into()))?;

                let bytes = arr
                    .iter()
                    .map(|v| v.as_u64().unwrap_or(0) as u8)
                    .collect::<Vec<u8>>();

                SliceData::Visual(VisualLayer::encode(bytes)?)
            }

            LayerId::Semantic => {
                SliceData::Semantic(SemanticLayer::encode(payload)?)
            }

            LayerId::Temporal => {
                let ts = payload
                    .as_str()
                    .ok_or_else(|| ApiError::AdapterError("temporal payload must be RFC3339 string".into()))?;

                let dt = ts.parse::<chrono::DateTime<chrono::Utc>>()?;
                SliceData::Temporal(TemporalLayer::encode(dt)?)
            }

            LayerId::Emotional => {
                let score = payload["score"].as_f64().unwrap_or(0.0) as f32;

                // FIX: EmotionalLayer now returns EmotionalOutput
                let out = EmotionalLayer::encode(score)?;

                SliceData::Emotional(out.value)
            }

            LayerId::Relational => {
                let arr = payload
                    .as_array()
                    .ok_or_else(|| ApiError::AdapterError("relational payload must be array".into()))?;

                let embedding = arr
                    .iter()
                    .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                    .collect::<Vec<f32>>();

                SliceData::Relational(RelationalLayer::encode(embedding)?)
            }

            LayerId::Declarative => {
                let text = payload
                    .as_str()
                    .ok_or_else(|| ApiError::AdapterError("declarative payload must be string".into()))?;

                // FIX: DeclarativeLayer now returns DeclarativeOutput
                let encoded = DeclarativeLayer::encode(text.to_string())?;

                SliceData::Declarative(encoded.normalized)
            }
        };

        frame.insert_slice(layer, slice);
        Ok(())
    }
}







