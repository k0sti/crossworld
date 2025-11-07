use crate::core::{octant_char_to_index, Axis, Cube, Octree};
use nom::{
    branch::alt,
    bytes::complete::take_while,
    character::complete::{char, i32 as nom_i32, multispace0, one_of},
    combinator::{map, opt, value},
    multi::many0,
    sequence::{delimited, preceded, tuple},
    IResult,
};
use std::collections::HashMap;
use std::rc::Rc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CsmError {
    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Path not found: {0}")]
    PathNotFound(String),

    #[error("Invalid octant character: {0}")]
    InvalidOctant(char),

    #[error("Expected {expected} children, got {actual}")]
    InvalidChildCount { expected: usize, actual: usize },
}

type Result<T> = std::result::Result<T, CsmError>;

// Whitespace and comments
fn comment(input: &str) -> IResult<&str, ()> {
    value(
        (),
        tuple((char('#'), take_while(|c| c != '\n'), opt(char('\n')))),
    )(input)
}

fn ws_or_comment(input: &str) -> IResult<&str, ()> {
    let (input, _) = multispace0(input)?;
    let mut remaining = input;
    while let Ok((input, _)) = comment(remaining) {
        let (input, _) = multispace0(input)?;
        remaining = input;
    }
    Ok((remaining, ()))
}

// Path parsing (octant chars a-h)
fn octant(input: &str) -> IResult<&str, usize> {
    map(one_of("abcdefgh"), |c| octant_char_to_index(c).unwrap())(input)
}

fn path(input: &str) -> IResult<&str, Vec<usize>> {
    many0(octant)(input)
}

// Axis parsing (x, y, z)
fn axis(input: &str) -> IResult<&str, Axis> {
    map(one_of("xyzXYZ"), |c| Axis::from_char(c).unwrap())(input)
}

// Swap parsing (^xyz)
fn swap_axes(input: &str) -> IResult<&str, Vec<Axis>> {
    preceded(char('^'), many0(axis))(input)
}

// Mirror parsing (/xyz)
fn mirror_axes(input: &str) -> IResult<&str, Vec<Axis>> {
    preceded(char('/'), many0(axis))(input)
}

// Cube value parsing
fn cube_value(input: &str) -> IResult<&str, Cube<i32>> {
    map(nom_i32, Cube::solid)(input)
}

// Reference parsing (<path>)
fn cube_reference<'a>(
    input: &'a str,
    prev_epoch: &Option<HashMap<Vec<usize>, Rc<Cube<i32>>>>,
) -> IResult<&'a str, Cube<i32>> {
    let (input, _) = char('<')(input)?;
    let (input, p) = path(input)?;

    if let Some(prev) = prev_epoch {
        if let Some(cube) = prev.get(&p) {
            Ok((input, (**cube).clone()))
        } else {
            Err(nom::Err::Failure(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Verify,
            )))
        }
    } else {
        Err(nom::Err::Failure(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Verify,
        )))
    }
}

// Array parsing [...]
fn cube_array<'a>(
    input: &'a str,
    prev_epoch: &Option<HashMap<Vec<usize>, Rc<Cube<i32>>>>,
) -> IResult<&'a str, Cube<i32>> {
    let (input, _) = char('[')(input)?;
    let (input, _) = ws_or_comment(input)?;

    let mut children = Vec::new();
    let mut remaining = input;

    for _ in 0..8 {
        let (input, _) = ws_or_comment(remaining)?;
        let (input, child) = parse_cube_inner(input, prev_epoch)?;
        children.push(Rc::new(child));
        remaining = input;
    }

    let (input, _) = ws_or_comment(remaining)?;
    let (input, _) = char(']')(input)?;

    if children.len() == 8 {
        Ok((input, Cube::cubes(children.try_into().unwrap())))
    } else {
        Err(nom::Err::Failure(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Count,
        )))
    }
}

// Swap application (^xyz <cube>)
fn cube_swap<'a>(
    input: &'a str,
    prev_epoch: &Option<HashMap<Vec<usize>, Rc<Cube<i32>>>>,
) -> IResult<&'a str, Cube<i32>> {
    let (input, axes) = swap_axes(input)?;
    let (input, _) = ws_or_comment(input)?;
    let (input, cube) = parse_cube_inner(input, prev_epoch)?;
    Ok((input, cube.apply_swap(&axes)))
}

// Mirror application (/xyz <cube>)
fn cube_mirror<'a>(
    input: &'a str,
    prev_epoch: &Option<HashMap<Vec<usize>, Rc<Cube<i32>>>>,
) -> IResult<&'a str, Cube<i32>> {
    let (input, axes) = mirror_axes(input)?;
    let (input, _) = ws_or_comment(input)?;
    let (input, cube) = parse_cube_inner(input, prev_epoch)?;
    Ok((input, cube.apply_mirror(&axes)))
}

// Main cube parser
fn parse_cube_inner<'a>(
    input: &'a str,
    prev_epoch: &Option<HashMap<Vec<usize>, Rc<Cube<i32>>>>,
) -> IResult<&'a str, Cube<i32>> {
    preceded(
        ws_or_comment,
        alt((
            |i| cube_swap(i, prev_epoch),
            |i| cube_mirror(i, prev_epoch),
            cube_value,
            |i| cube_array(i, prev_epoch),
            |i| cube_reference(i, prev_epoch),
        )),
    )(input)
}

// Statement parsing (>path cube)
fn statement<'a>(
    input: &'a str,
    prev_epoch: &Option<HashMap<Vec<usize>, Rc<Cube<i32>>>>,
) -> IResult<&'a str, (Vec<usize>, Cube<i32>)> {
    let (input, _) = ws_or_comment(input)?;
    let (input, _) = char('>')(input)?;
    let (input, p) = path(input)?;
    let (input, _) = ws_or_comment(input)?;
    let (input, cube) = parse_cube_inner(input, prev_epoch)?;
    Ok((input, (p, cube)))
}

// Epoch separator (|)
fn epoch_separator(input: &str) -> IResult<&str, ()> {
    value((), delimited(ws_or_comment, char('|'), ws_or_comment))(input)
}

// Epoch parsing
fn epoch<'a>(
    input: &'a str,
    prev_epoch: &Option<HashMap<Vec<usize>, Rc<Cube<i32>>>>,
) -> IResult<&'a str, HashMap<Vec<usize>, Rc<Cube<i32>>>> {
    let mut assignments = HashMap::new();
    let mut remaining = input;

    loop {
        let (input, _) = ws_or_comment(remaining)?;

        // Check for epoch separator or end
        if input.is_empty() || input.starts_with('|') {
            remaining = input;
            break;
        }

        // Try to parse statement
        match statement(input, prev_epoch) {
            Ok((input, (p, cube))) => {
                assignments.insert(p, Rc::new(cube));
                remaining = input;
            }
            Err(_) => break,
        }
    }

    Ok((remaining, assignments))
}

// Full model parser
fn parse_model(input: &str) -> IResult<&str, Octree> {
    let mut prev_epoch: Option<HashMap<Vec<usize>, Rc<Cube<i32>>>> = None;
    let mut current_epoch = HashMap::new();
    let mut remaining = input;

    loop {
        let (input, _) = ws_or_comment(remaining)?;

        if input.is_empty() {
            break;
        }

        // Parse epoch
        let (input, assignments) = epoch(input, &prev_epoch)?;
        if !assignments.is_empty() {
            current_epoch = assignments.clone();
            prev_epoch = Some(assignments);
        }

        // Check for epoch separator
        if let Ok((input, _)) = epoch_separator(input) {
            remaining = input;
        } else {
            remaining = input;
            break;
        }
    }

    // Build final cube from root assignment or empty
    let root = if let Some(cube) = current_epoch.get(&vec![]) {
        (**cube).clone()
    } else {
        // Build from partial assignments
        build_cube_from_assignments(&current_epoch, &[])
    };

    Ok((remaining, Octree::new(root)))
}

// Build cube from partial assignments
fn build_cube_from_assignments(
    assignments: &HashMap<Vec<usize>, Rc<Cube<i32>>>,
    prefix: &[usize],
) -> Cube<i32> {
    // Check for direct assignment
    if let Some(cube) = assignments.get(prefix) {
        return (**cube).clone();
    }

    // Check if any children exist
    let has_children = assignments
        .keys()
        .any(|path| path.len() > prefix.len() && path[..prefix.len()] == *prefix);

    if !has_children {
        return Cube::Solid(0);
    }

    // Build children
    let children: Vec<Rc<Cube<i32>>> = (0..8)
        .map(|i| {
            let mut child_prefix = prefix.to_vec();
            child_prefix.push(i);
            Rc::new(build_cube_from_assignments(assignments, &child_prefix))
        })
        .collect();

    Cube::cubes(children.try_into().unwrap())
}

/// Parse CSM text into an Octree
pub fn parse_csm(input: &str) -> Result<Octree> {
    match parse_model(input) {
        Ok((_, tree)) => Ok(tree),
        Err(e) => Err(CsmError::ParseError(format!("{:?}", e))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_value() {
        let csm = ">a 42";
        let tree = parse_csm(csm).unwrap();
        // Just verify it parses without error - detailed mesh generation tested elsewhere
        assert!(matches!(tree.root, crate::core::Cube::Cubes(_)));
    }

    #[test]
    fn test_parse_array() {
        let csm = ">a [1 2 3 4 5 6 7 8]";
        let tree = parse_csm(csm).unwrap();
        assert!(matches!(tree.root, crate::core::Cube::Cubes(_)));
    }

    #[test]
    fn test_parse_nested() {
        let csm = r#"
            >a [1 2 3 4 5 6 7 8]
            >aa [10 11 12 13 14 15 16 17]
        "#;
        let tree = parse_csm(csm).unwrap();
        assert!(matches!(tree.root, crate::core::Cube::Cubes(_)));
    }

    #[test]
    fn test_parse_reference() {
        let csm = r#"
            >a 100
            | >b <a
        "#;
        let tree = parse_csm(csm).unwrap();
        assert!(matches!(tree.root, crate::core::Cube::Cubes(_)));
    }

    #[test]
    fn test_parse_swap() {
        let csm = r#">a [1 2 3 4 5 6 7 8]
            | >b ^x <a
        "#;
        let tree = parse_csm(csm).unwrap();
        assert!(matches!(tree.root, crate::core::Cube::Cubes(_)));
    }

    #[test]
    fn test_parse_mirror() {
        let csm = r#"
            >a [1 2 3 4 5 6 7 8]
            | >b /x <a
        "#;
        let tree = parse_csm(csm).unwrap();
        assert!(matches!(tree.root, crate::core::Cube::Cubes(_)));
    }
}
