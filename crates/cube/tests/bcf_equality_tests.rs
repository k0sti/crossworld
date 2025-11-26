//! BCF structural equality tests
//!
//! Verifies that:
//! 1. Serialization is deterministic (same input → same bytes)
//! 2. Equal cubes produce equal binary (cube1 == cube2 → serialize(cube1) == serialize(cube2))
//! 3. Different cubes produce different binary (no collisions)

use cube::io::bcf::serialize_bcf;
use cube::Cube;
use std::rc::Rc;

/// Helper: Create octree with 8 solid children
fn create_octa_leaves(values: [u8; 8]) -> Cube<u8> {
    Cube::Cubes(Box::new([
        Rc::new(Cube::Solid(values[0])),
        Rc::new(Cube::Solid(values[1])),
        Rc::new(Cube::Solid(values[2])),
        Rc::new(Cube::Solid(values[3])),
        Rc::new(Cube::Solid(values[4])),
        Rc::new(Cube::Solid(values[5])),
        Rc::new(Cube::Solid(values[6])),
        Rc::new(Cube::Solid(values[7])),
    ]))
}

#[test]
fn test_determinism_solid_inline() {
    let cube = Cube::Solid(42u8);

    let bytes1 = serialize_bcf(&cube);
    let bytes2 = serialize_bcf(&cube);
    let bytes3 = serialize_bcf(&cube);

    assert_eq!(bytes1, bytes2, "Serialization must be deterministic");
    assert_eq!(bytes2, bytes3, "Serialization must be deterministic");
}

#[test]
fn test_determinism_solid_extended() {
    let cube = Cube::Solid(200u8);

    let bytes1 = serialize_bcf(&cube);
    let bytes2 = serialize_bcf(&cube);

    assert_eq!(bytes1, bytes2, "Serialization must be deterministic");
}

#[test]
fn test_determinism_octa_leaves() {
    let cube = create_octa_leaves([1, 2, 3, 4, 5, 6, 7, 8]);

    let bytes1 = serialize_bcf(&cube);
    let bytes2 = serialize_bcf(&cube);
    let bytes3 = serialize_bcf(&cube);

    assert_eq!(bytes1, bytes2, "Serialization must be deterministic");
    assert_eq!(bytes2, bytes3, "Serialization must be deterministic");
}

#[test]
fn test_determinism_complex_tree() {
    let pattern = create_octa_leaves([10, 20, 30, 40, 50, 60, 70, 80]);
    let cube = Cube::Cubes(Box::new([
        Rc::new(pattern.clone()),
        Rc::new(Cube::Solid(1)),
        Rc::new(pattern.clone()),
        Rc::new(Cube::Solid(2)),
        Rc::new(pattern.clone()),
        Rc::new(Cube::Solid(3)),
        Rc::new(pattern.clone()),
        Rc::new(Cube::Solid(4)),
    ]));

    let bytes1 = serialize_bcf(&cube);
    let bytes2 = serialize_bcf(&cube);

    assert_eq!(
        bytes1, bytes2,
        "Complex tree serialization must be deterministic"
    );
}

#[test]
fn test_equal_cubes_produce_equal_binary() {
    // Two separately constructed but equal cubes
    let cube1 = Cube::Solid(42u8);
    let cube2 = Cube::Solid(42u8);

    assert_eq!(cube1, cube2, "Cubes should be equal");

    let bytes1 = serialize_bcf(&cube1);
    let bytes2 = serialize_bcf(&cube2);

    assert_eq!(bytes1, bytes2, "Equal cubes must produce equal binary");
}

#[test]
fn test_equal_octas_produce_equal_binary() {
    let cube1 = create_octa_leaves([1, 2, 3, 4, 5, 6, 7, 8]);
    let cube2 = create_octa_leaves([1, 2, 3, 4, 5, 6, 7, 8]);

    assert_eq!(cube1, cube2, "Octas should be equal");

    let bytes1 = serialize_bcf(&cube1);
    let bytes2 = serialize_bcf(&cube2);

    assert_eq!(bytes1, bytes2, "Equal octas must produce equal binary");
}

#[test]
fn test_different_values_produce_different_binary() {
    let cube1 = Cube::Solid(42u8);
    let cube2 = Cube::Solid(43u8);

    assert_ne!(cube1, cube2, "Cubes should be different");

    let bytes1 = serialize_bcf(&cube1);
    let bytes2 = serialize_bcf(&cube2);

    assert_ne!(
        bytes1, bytes2,
        "Different cubes must produce different binary"
    );
}

#[test]
fn test_inline_vs_extended_different_binary() {
    // Value 127 (inline) vs 128 (extended) - different encoding
    let cube1 = Cube::Solid(127u8);
    let cube2 = Cube::Solid(128u8);

    assert_ne!(cube1, cube2);

    let bytes1 = serialize_bcf(&cube1);
    let bytes2 = serialize_bcf(&cube2);

    assert_ne!(bytes1, bytes2, "Inline vs extended must differ");

    // Also verify different sizes
    assert_eq!(bytes1.len(), 13, "Inline leaf: 1 byte data + 12 header");
    assert_eq!(bytes2.len(), 14, "Extended leaf: 2 bytes data + 12 header");
}

#[test]
fn test_different_octa_patterns_produce_different_binary() {
    let cube1 = create_octa_leaves([1, 2, 3, 4, 5, 6, 7, 8]);
    let cube2 = create_octa_leaves([8, 7, 6, 5, 4, 3, 2, 1]); // Reversed

    assert_ne!(cube1, cube2);

    let bytes1 = serialize_bcf(&cube1);
    let bytes2 = serialize_bcf(&cube2);

    assert_ne!(
        bytes1, bytes2,
        "Different octa patterns must produce different binary"
    );
}

#[test]
fn test_solid_vs_octa_different_binary() {
    // Even if logically similar, structure differs
    let solid = Cube::Solid(5u8);
    let octa = create_octa_leaves([5, 5, 5, 5, 5, 5, 5, 5]);

    assert_ne!(solid, octa, "Solid vs octa should differ structurally");

    let bytes_solid = serialize_bcf(&solid);
    let bytes_octa = serialize_bcf(&octa);

    assert_ne!(
        bytes_solid, bytes_octa,
        "Solid vs octa must produce different binary"
    );
}

#[test]
fn test_octa_leaves_vs_octa_pointers_optimization() {
    // Octa with all solid children (should use octa-leaves encoding)
    let octa_leaves = create_octa_leaves([1, 2, 3, 4, 5, 6, 7, 8]);
    let bytes_leaves = serialize_bcf(&octa_leaves);

    // Octa with complex children (should use octa-pointers encoding)
    let pattern = create_octa_leaves([10, 20, 30, 40, 50, 60, 70, 80]);
    let octa_pointers = Cube::Cubes(Box::new([
        Rc::new(pattern.clone()),
        Rc::new(Cube::Solid(1)),
        Rc::new(pattern.clone()),
        Rc::new(Cube::Solid(2)),
        Rc::new(pattern.clone()),
        Rc::new(Cube::Solid(3)),
        Rc::new(pattern.clone()),
        Rc::new(Cube::Solid(4)),
    ]));
    let bytes_pointers = serialize_bcf(&octa_pointers);

    // Octa-leaves encoding is more compact
    assert!(
        bytes_leaves.len() < bytes_pointers.len(),
        "Octa-leaves should be more compact than octa-pointers"
    );

    // Octa-leaves: 9 bytes data (1 type + 8 values) + 12 header = 21 bytes
    assert_eq!(bytes_leaves.len(), 21, "Octa-leaves should be 21 bytes");
}

#[test]
fn test_zero_vs_nonzero_values() {
    let cube_zero = Cube::Solid(0u8);
    let cube_nonzero = Cube::Solid(1u8);

    assert_ne!(cube_zero, cube_nonzero);

    let bytes_zero = serialize_bcf(&cube_zero);
    let bytes_nonzero = serialize_bcf(&cube_nonzero);

    assert_ne!(
        bytes_zero, bytes_nonzero,
        "Zero vs non-zero must produce different binary"
    );
}

#[test]
fn test_boundary_values_distinct() {
    // Test boundary values produce distinct binary
    let values = vec![0u8, 1, 127, 128, 254, 255];

    let mut all_bytes = Vec::new();
    for &val in &values {
        let cube = Cube::Solid(val);
        let bytes = serialize_bcf(&cube);
        all_bytes.push(bytes);
    }

    // Verify all binary outputs are distinct
    for i in 0..all_bytes.len() {
        for j in (i + 1)..all_bytes.len() {
            assert_ne!(
                all_bytes[i], all_bytes[j],
                "Boundary values {} and {} must produce different binary",
                values[i], values[j]
            );
        }
    }
}

#[test]
fn test_structural_equality_after_cloning() {
    let original = create_octa_leaves([1, 2, 3, 4, 5, 6, 7, 8]);
    let cloned = original.clone();

    assert_eq!(original, cloned, "Clone should equal original");

    let bytes_original = serialize_bcf(&original);
    let bytes_cloned = serialize_bcf(&cloned);

    assert_eq!(
        bytes_original, bytes_cloned,
        "Cloned cube must produce identical binary"
    );
}

#[test]
fn test_consistent_hashing_via_binary() {
    // If we use binary output for hashing/caching, it must be consistent
    let cube = create_octa_leaves([10, 20, 30, 40, 50, 60, 70, 80]);

    // Serialize multiple times and verify hash consistency
    let hashes: Vec<u64> = (0..5)
        .map(|_| {
            let bytes = serialize_bcf(&cube);
            // Simple hash for testing (not cryptographic)
            bytes
                .iter()
                .fold(0u64, |acc, &b| acc.wrapping_mul(31).wrapping_add(b as u64))
        })
        .collect();

    // All hashes should be identical
    for i in 1..hashes.len() {
        assert_eq!(
            hashes[0], hashes[i],
            "Binary output hash must be consistent across serializations"
        );
    }
}
