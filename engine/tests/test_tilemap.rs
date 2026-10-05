use engine::tilemap::{TileLayer, TileOrientation, TileSet, Tilemap};

#[test]
fn tilemap_default_and_serialization() {
    let ts = TileSet {
        tile_size: 16.0,
        base_path: "tiles/".into(),
    };
    let layer = TileLayer {
        tiles: vec![Some(1), Some(2), None, Some(0)],
        visible: true,
    };
    let tm = Tilemap {
        name: "test_map".into(),
        tile_set: ts,
        layers: vec![layer],
        origin: [0.0, 0.0].into(),
        orientation: TileOrientation::Orthogonal,
    };
    let ron = tm.to_ron_string().unwrap();
    let loaded = Tilemap::from_ron_string(&ron).unwrap();
    assert_eq!(loaded.name, "test_map");
    assert_eq!(loaded.layers.len(), 1);
    assert_eq!(loaded.layers[0].tiles.len(), 4);
}

#[test]
fn tilemap_iterates_tiles() {
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
    let positions: Vec<(usize, [f32; 2])> = tm
        .tiles_by_layer(0)
        .iter()
        .map(|(idx, pos)| (*idx, [pos[0], pos[1]]))
        .collect();
    assert_eq!(positions.len(), 2);
    assert_eq!(positions[0], (0, [0.0, 0.0]));
    assert_eq!(positions[1], (2, [16.0, 0.0]));
}
