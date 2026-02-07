//! BCF format serialization and deserialization tests

use cube::io::bcf::{parse_bcf, serialize_bcf, BcfError};
use cube::Cube;
use std::rc::Rc;

#[test]
fn test_inline_leaf_encoding() {
    // Test inline leaf (value 0-127)
    let cube = Cube::Solid(42u8);
    let binary = serialize_bcf(&cube);

    // Should be: header (12 bytes) + inline leaf (1 byte) = 13 bytes
    assert_eq!(binary.len(), 13);

    // Parse it back
    let parsed = parse_bcf(&binary).unwrap();
    assert_eq!(cube, parsed);
}

#[test]
fn test_extended_leaf_encoding() {
    // Test extended leaf (value 128-255)
    let cube = Cube::Solid(200u8);
    let binary = serialize_bcf(&cube);

    // Should be: header (12 bytes) + extended leaf (2 bytes) = 14 bytes
    assert_eq!(binary.len(), 14);

    // Parse it back
    let parsed = parse_bcf(&binary).unwrap();
    assert_eq!(cube, parsed);
}

#[test]
fn test_octa_with_leaves() {
    // Create octree with 8 leaf values
    let children: [Rc<Cube<u8>>; 8] = [
        Rc::new(Cube::Solid(1)),
        Rc::new(Cube::Solid(2)),
        Rc::new(Cube::Solid(3)),
        Rc::new(Cube::Solid(4)),
        Rc::new(Cube::Solid(5)),
        Rc::new(Cube::Solid(6)),
        Rc::new(Cube::Solid(7)),
        Rc::new(Cube::Solid(8)),
    ];
    let cube = Cube::Cubes(Box::new(children));

    let binary = serialize_bcf(&cube);

    // Should be: header (12 bytes) + octa-with-leaves (9 bytes) = 21 bytes
    assert_eq!(binary.len(), 21);

    // Parse it back
    let parsed = parse_bcf(&binary).unwrap();
    assert_eq!(cube, parsed);
}

#[test]
fn test_octa_with_pointers() {
    // Create octree with mixed children (some subdivided)
    let leaf1 = Rc::new(Cube::Solid(10));
    let leaf2 = Rc::new(Cube::Solid(20));

    // Create a subdivided octant
    let sub_children: [Rc<Cube<u8>>; 8] = [
        Rc::new(Cube::Solid(1)),
        Rc::new(Cube::Solid(2)),
        Rc::new(Cube::Solid(3)),
        Rc::new(Cube::Solid(4)),
        Rc::new(Cube::Solid(5)),
        Rc::new(Cube::Solid(6)),
        Rc::new(Cube::Solid(7)),
        Rc::new(Cube::Solid(8)),
    ];
    let sub_cube = Rc::new(Cube::Cubes(Box::new(sub_children)));

    let children: [Rc<Cube<u8>>; 8] = [
        sub_cube,
        leaf1.clone(),
        leaf2.clone(),
        leaf1.clone(),
        leaf2.clone(),
        leaf1.clone(),
        leaf2.clone(),
        leaf1.clone(),
    ];

    let cube = Cube::Cubes(Box::new(children));
    let binary = serialize_bcf(&cube);

    // Parse it back
    let parsed = parse_bcf(&binary).unwrap();
    assert_eq!(cube, parsed);
}

#[test]
fn test_round_trip_simple() {
    let cube = Cube::Solid(0u8);
    let binary = serialize_bcf(&cube);
    let parsed = parse_bcf(&binary).unwrap();
    assert_eq!(cube, parsed);
}

#[test]
fn test_round_trip_max_value() {
    let cube = Cube::Solid(255u8);
    let binary = serialize_bcf(&cube);
    let parsed = parse_bcf(&binary).unwrap();
    assert_eq!(cube, parsed);
}

#[test]
fn test_invalid_magic() {
    let bad_data = vec![
        0xFF, 0xFF, 0xFF, 0xFF, 0x01, 0x00, 0x00, 0x00, 0x0C, 0x00, 0x00, 0x00, 0x2A,
    ];
    let result = parse_bcf(&bad_data);
    assert!(matches!(result, Err(BcfError::InvalidMagic { .. })));
}

#[test]
fn test_unsupported_version() {
    // Create valid BCF data but with wrong version
    let cube = Cube::Solid(42u8);
    let mut binary = serialize_bcf(&cube);

    // Corrupt the version byte (byte 4)
    binary[4] = 0xFF;

    let result = parse_bcf(&binary);
    assert!(matches!(result, Err(BcfError::UnsupportedVersion { .. })));
}

#[test]
fn test_truncated_data() {
    let bad_data = vec![0x42, 0x43, 0x46]; // Only 3 bytes, need at least 12
    let result = parse_bcf(&bad_data);
    assert!(matches!(result, Err(BcfError::TruncatedData { .. })));
}
