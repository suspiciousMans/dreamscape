use engine::material::{Material, MaterialFile, MaterialVariant};

#[test]
fn material_defaults_and_serialization() {
    let mat = Material {
        base_color: None,
        normal_map: None,
        roughness: 0.5,
        metallic: 0.0,
        variant: MaterialVariant::Lit,
    };
    // defaults round-trip
    let ron = mat.to_ron_string().unwrap();
    let loaded = Material::from_ron_string(&ron).unwrap();
    assert_eq!(loaded.roughness, 0.5);
    assert_eq!(loaded.metallic, 0.0);
    assert_eq!(loaded.variant, MaterialVariant::Lit);
}

#[test]
fn material_file_load_save_roundtrip() {
    let mf = MaterialFile {
        name: "test_mat".into(),
        material: Material {
            base_color: Some("textures/block_color.png".into()),
            normal_map: Some("textures/block_normal.png".into()),
            roughness: 0.35,
            metallic: 0.2,
            variant: MaterialVariant::Lit,
        },
    };
    let ron = mf.to_ron_string().unwrap();
    let loaded = MaterialFile::from_ron_string(&ron).unwrap();
    assert_eq!(loaded.name, "test_mat");
    assert_eq!(loaded.material.roughness, 0.35);
    assert_eq!(
        loaded.material.base_color.as_ref().unwrap().as_str(),
        "textures/block_color.png"
    );
}

#[test]
fn material_variant_tag_selects_shader_branch() {
    let lit = MaterialVariant::Lit;
    let lit_normals = MaterialVariant::LitNormals;
    let unlit = MaterialVariant::Unlit;

    assert!(matches!(lit, MaterialVariant::Lit));
    assert!(matches!(lit_normals, MaterialVariant::LitNormals));
    assert!(matches!(unlit, MaterialVariant::Unlit));

    // The rule we're encoding: LitNormals => use the normal-map shader variant.
    let (v, f) = engine::shader::shader_files_for(&lit_normals);
    assert_eq!(v, "shaders/mesh_normals.vert");
    assert_eq!(f, "shaders/mesh_normals.frag");

    let (v, f) = engine::shader::shader_files_for(&lit);
    assert_eq!(v, "shaders/mesh.vert");
    assert_eq!(f, "shaders/mesh.frag");

    let (v, f) = engine::shader::shader_files_for(&unlit);
    assert_eq!(v, "shaders/mesh.vert");
    assert_eq!(f, "shaders/mesh.frag");
}
