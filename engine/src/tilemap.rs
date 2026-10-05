use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Tile orientation for the map. Phase 1 only needs orthogonal; keep the
/// enum here so the data model is ready for isometric later without a
/// breaking change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TileOrientation {
    #[serde(rename = "orthogonal")]
    Orthogonal,
}

impl Default for TileOrientation {
    fn default() -> Self {
        TileOrientation::Orthogonal
    }
}

/// Shared tile-set metadata for a tilemap: the size of one tile (in world
/// units) and the base path used to resolve tile texture assets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileSet {
    pub tile_size: f32,
    pub base_path: String,
}

impl Default for TileSet {
    fn default() -> Self {
        Self {
            tile_size: 16.0,
            base_path: String::new(),
        }
    }
}

/// One row of tiles in a tilemap. Phase 1 models each layer as a single row
/// (column-major within the row): `tiles` is indexed left-to-right and
/// position for tile at index `i` is `[i * tile_size, 0.0]` relative to the
/// layer origin. Multi-row layers can be added later by introducing an
/// explicit width field; the serialization and `tiles_by_layer` contract
/// stays the same.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileLayer {
    pub tiles: Vec<Option<usize>>,
    pub visible: bool,
}

impl Default for TileLayer {
    fn default() -> Self {
        Self {
            tiles: Vec::new(),
            visible: true,
        }
    }
}

/// A tilemap: a name, one tile-set, one or more layers, a world-space origin,
/// and an orientation. Serializes to/from RON via serde.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tilemap {
    pub name: String,
    pub tile_set: TileSet,
    pub layers: Vec<TileLayer>,
    pub origin: [f32; 2],
    pub orientation: TileOrientation,
}

impl Default for Tilemap {
    fn default() -> Self {
        Self {
            name: String::new(),
            tile_set: TileSet::default(),
            layers: Vec::new(),
            origin: [0.0, 0.0],
            orientation: TileOrientation::default(),
        }
    }
}

impl Tilemap {
    /// Serialize this tilemap to a RON string.
    pub fn to_ron_string(&self) -> Result<String> {
        ron::to_string(self).context("serialize tilemap")
    }

    /// Deserialize a tilemap from a RON string.
    pub fn from_ron_string(src: &str) -> Result<Self> {
        ron::from_str(src).context("deserialize tilemap")
    }

    /// Iterate over non-empty tiles in the given layer, returning
    /// `(tile_index, world_position)` pairs. `world_position` is relative to
    /// the tilemap origin (add `origin` to get absolute world coords).
    ///
    /// Phase 1 mapping: single-row layers, column-major within the row.
    /// `position = (index * tile_size, 0.0) + origin`.
    pub fn tiles_by_layer(&self, layer_index: usize) -> Vec<(usize, [f32; 2])> {
        let layer = match self.layers.get(layer_index) {
            Some(l) => l,
            None => return Vec::new(),
        };
        let tile_size = self.tile_set.tile_size;
        let origin = self.origin;
        layer
            .tiles
            .iter()
            .enumerate()
            .filter_map(|(i, tile)| tile.map(|_t| (i, [i as f32 * tile_size + origin[0], origin[1]])))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tilemap_default_is_empty() {
        let tm = Tilemap::default();
        assert!(tm.name.is_empty());
        assert!(tm.layers.is_empty());
        assert_eq!(tm.origin, [0.0, 0.0]);
        assert_eq!(tm.orientation, TileOrientation::Orthogonal);
    }

    #[test]
    fn tilemap_roundtrip_via_ron() {
        let tm = Tilemap {
            name: "demo".into(),
            tile_set: TileSet {
                tile_size: 16.0,
                base_path: "tiles/".into(),
            },
            layers: vec![TileLayer {
                tiles: vec![Some(1), Some(2), None, Some(0)],
                visible: true,
            }],
            origin: [0.0, 0.0].into(),
            orientation: TileOrientation::Orthogonal,
        };
        let ron = tm.to_ron_string().unwrap();
        let loaded = Tilemap::from_ron_string(&ron).unwrap();
        assert_eq!(loaded.name, "demo");
        assert_eq!(loaded.layers.len(), 1);
        assert_eq!(loaded.layers[0].tiles.len(), 4);
        assert_eq!(loaded.tile_set.tile_size, 16.0);
    }

    #[test]
    fn tiles_by_layer_skips_empty_slots() {
        let tm = Tilemap {
            name: "x".into(),
            tile_set: TileSet {
                tile_size: 8.0,
                base_path: "".into(),
            },
            layers: vec![TileLayer {
                tiles: vec![Some(3), None, Some(7)],
                visible: true,
            }],
            origin: [0.0, 0.0].into(),
            orientation: TileOrientation::Orthogonal,
        };
        let positions = tm.tiles_by_layer(0);
        assert_eq!(positions.len(), 2);
        assert_eq!(positions[0], (0, [0.0, 0.0]));
        assert_eq!(positions[1], (2, [16.0, 0.0]));
    }

    #[test]
    fn tiles_by_layer_out_of_range_returns_empty() {
        let tm = Tilemap {
            name: "x".into(),
            tile_set: TileSet::default(),
            layers: Vec::new(),
            origin: [0.0, 0.0].into(),
            orientation: TileOrientation::Orthogonal,
        };
        assert!(tm.tiles_by_layer(0).is_empty());
        assert!(tm.tiles_by_layer(99).is_empty());
    }
}
