//! Debug test to examine cube structure

use cube::Cube;
use renderer::scenes::create_octa_cube;

#[test]
fn test_examine_octa_cube_structure() {
    let cube = create_octa_cube();

    println!("Octa cube structure:");
    match cube.as_ref() {
        Cube::Solid(val) => {
            println!("  ERROR: Root is Solid({}), expected Cubes!", val);
            panic!("Octa cube should be Cubes, not Solid!");
        }
        Cube::Cubes(children) => {
            println!("  Root is Cubes with {} children", children.len());
            for (i, child) in children.iter().enumerate() {
                match child.as_ref() {
                    Cube::Solid(val) => println!("    Child {}: Solid({})", i, val),
                    Cube::Cubes(_) => println!("    Child {}: Cubes(...)", i),
                    _ => println!("    Child {}: Other", i),
                }
            }
        }
        _ => {
            println!("  ERROR: Unexpected cube type!");
            panic!("Unexpected cube type!");
        }
    }

    // Also check what the default CpuCubeTracer gets
    use renderer::cpu_tracer::CpuCubeTracer;
    let tracer = CpuCubeTracer::new();

    // Can't directly access the cube, but we can create one with the known scene
    println!("\n Default tracer should use octa cube (created via new())");
}
