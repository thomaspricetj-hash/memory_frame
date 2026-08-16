use serde::{Serialize, Deserialize};
use std::fmt;
use std::str::FromStr;

/// Canonical lowercase names for each layer.
pub const VISUAL_NAME: &str = "visual";
pub const SEMANTIC_NAME: &str = "semantic";
pub const TEMPORAL_NAME: &str = "temporal";
pub const EMOTIONAL_NAME: &str = "emotional";
pub const RELATIONAL_NAME: &str = "relational";
pub const DECLARATIVE_NAME: &str = "declarative";

/// Static lookup table for fast reverse mapping.
const LAYER_NAME_PAIRS: &[(&str, LayerId)] = &[
    (VISUAL_NAME, LayerId::Visual),
    (SEMANTIC_NAME, LayerId::Semantic),
    (TEMPORAL_NAME, LayerId::Temporal),
    (EMOTIONAL_NAME, LayerId::Emotional),
    (RELATIONAL_NAME, LayerId::Relational),
    (DECLARATIVE_NAME, LayerId::Declarative),
];

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LayerId {
    Visual,
    Semantic,
    Temporal,
    Emotional,
    Relational,
    Declarative,
}

impl LayerId {
    #[inline]
    pub fn as_str(&self) -> &'static str {
        match self {
            LayerId::Visual => VISUAL_NAME,
            LayerId::Semantic => SEMANTIC_NAME,
            LayerId::Temporal => TEMPORAL_NAME,
            LayerId::Emotional => EMOTIONAL_NAME,
            LayerId::Relational => RELATIONAL_NAME,
            LayerId::Declarative => DECLARATIVE_NAME,
        }
    }

    #[inline]
    pub fn all() -> &'static [LayerId] {
        const ALL: &[LayerId] = &[
            LayerId::Visual,
            LayerId::Semantic,
            LayerId::Temporal,
            LayerId::Emotional,
            LayerId::Relational,
            LayerId::Declarative,
        ];
        ALL
    }

    /// Fast reverse lookup without allocating.
    #[inline]
    pub fn from_str_fast(s: &str) -> Option<Self> {
        let key = s.trim().to_lowercase();
        for (name, id) in LAYER_NAME_PAIRS {
            if key == *name {
                // id: &LayerId â†’ clone to get a LayerId value
                return Some(id.clone());
            }
        }
        None
    }
}

impl fmt::Display for LayerId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for LayerId {
    type Err = ();

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        LayerId::from_str_fast(s).ok_or(())
    }
}








