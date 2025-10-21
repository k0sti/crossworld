use crate::octree::{Octant, Octree, OctreeBuilder, OctreeNode};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CsmError {
    #[error("Unexpected token at position {pos}: {msg}")]
    UnexpectedToken { pos: usize, msg: String },

    #[error("Invalid octant character: {0}")]
    InvalidOctant(char),

    #[error("Expected {expected} children in array, got {actual}")]
    InvalidChildCount { expected: usize, actual: usize },

    #[error("Path not found: {0}")]
    PathNotFound(String),

    #[error("Invalid syntax: {0}")]
    InvalidSyntax(String),

    #[error("Parse error: {0}")]
    ParseError(String),
}

type Result<T> = std::result::Result<T, CsmError>;

/// Token types for the lexer
#[derive(Debug, Clone, PartialEq)]
enum Token {
    Greater,              // >
    LeftBracket,          // [
    RightBracket,         // ]
    LeftAngle,            // <
    Slash,                // /
    Pipe,                 // |
    Octant(Octant),       // a-h
    Axis(char),           // x, y, z
    Integer(i32),
    Newline,
}

struct Lexer {
    input: Vec<char>,
    pos: usize,
}

impl Lexer {
    fn new(input: &str) -> Self {
        Lexer {
            input: input.chars().collect(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        if self.pos < self.input.len() {
            Some(self.input[self.pos])
        } else {
            None
        }
    }

    fn advance(&mut self) -> Option<char> {
        if self.pos < self.input.len() {
            let ch = self.input[self.pos];
            self.pos += 1;
            Some(ch)
        } else {
            None
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() && ch != '\n' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_comment(&mut self) {
        if self.peek() == Some('#') {
            while let Some(ch) = self.advance() {
                if ch == '\n' {
                    break;
                }
            }
        }
    }

    fn next_token(&mut self) -> Option<Token> {
        loop {
            self.skip_whitespace();

            if self.peek() == Some('#') {
                self.skip_comment();
                continue;
            }

            let ch = self.peek()?;

            return Some(match ch {
                '\n' => {
                    self.advance();
                    Token::Newline
                }
                '>' => {
                    self.advance();
                    Token::Greater
                }
                '[' => {
                    self.advance();
                    Token::LeftBracket
                }
                ']' => {
                    self.advance();
                    Token::RightBracket
                }
                '<' => {
                    self.advance();
                    Token::LeftAngle
                }
                '/' => {
                    self.advance();
                    Token::Slash
                }
                '|' => {
                    self.advance();
                    Token::Pipe
                }
                'a' | 'b' | 'c' | 'd' | 'e' | 'f' | 'g' | 'h' => {
                    self.advance();
                    Token::Octant(Octant::from_char(ch).unwrap())
                }
                'x' | 'y' | 'z' => {
                    self.advance();
                    Token::Axis(ch)
                }
                '-' | '0'..='9' => {
                    let mut num_str = String::new();
                    if ch == '-' {
                        num_str.push(ch);
                        self.advance();
                    }
                    while let Some(digit) = self.peek() {
                        if digit.is_ascii_digit() {
                            num_str.push(digit);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    let num = num_str.parse::<i32>().ok()?;
                    Token::Integer(num)
                }
                '=' => {
                    // Skip equals sign
                    self.advance();
                    continue;
                }
                _ => {
                    self.advance();
                    continue;
                }
            });
        }
    }
}

/// Parser for CSM (Cube Script Model)
struct Parser {
    lexer: Lexer,
    current_token: Option<Token>,
}

impl Parser {
    fn new(input: &str) -> Self {
        let mut lexer = Lexer::new(input);
        let current_token = lexer.next_token();
        Parser {
            lexer,
            current_token,
        }
    }

    fn advance(&mut self) {
        self.current_token = self.lexer.next_token();
    }

    fn skip_newlines(&mut self) {
        while self.current_token == Some(Token::Newline) {
            self.advance();
        }
    }

    fn parse_path(&mut self) -> Result<Vec<Octant>> {
        let mut path = Vec::new();
        while let Some(Token::Octant(octant)) = self.current_token {
            path.push(octant);
            self.advance();
        }
        Ok(path)
    }

    fn parse_transform(&mut self) -> Result<Vec<char>> {
        let mut axes = Vec::new();
        if self.current_token == Some(Token::Slash) {
            self.advance();
            while let Some(Token::Axis(axis)) = self.current_token {
                axes.push(axis);
                self.advance();
            }
        }
        Ok(axes)
    }

    fn parse_cube(&mut self, prev_epoch: &Option<HashMap<Vec<Octant>, OctreeNode>>) -> Result<OctreeNode> {
        self.skip_newlines();

        match &self.current_token {
            Some(Token::Integer(value)) => {
                let value = *value;
                self.advance();
                Ok(OctreeNode::Value(value))
            }
            Some(Token::LeftBracket) => {
                self.advance();
                let mut children = Vec::new();
                for _ in 0..8 {
                    self.skip_newlines();
                    let child = self.parse_cube(prev_epoch)?;
                    children.push(child);
                    self.skip_newlines();
                }
                if self.current_token != Some(Token::RightBracket) {
                    return Err(CsmError::InvalidSyntax(
                        "Expected ] after 8 children".to_string(),
                    ));
                }
                self.advance();

                let len = children.len();
                if len != 8 {
                    return Err(CsmError::InvalidChildCount {
                        expected: 8,
                        actual: len,
                    });
                }

                Ok(OctreeNode::new_children(
                    children.try_into().map_err(|_| {
                        CsmError::InvalidChildCount {
                            expected: 8,
                            actual: len,
                        }
                    })?,
                ))
            }
            Some(Token::LeftAngle) => {
                self.advance();
                let path = self.parse_path()?;

                // Look up the node from previous epoch
                if let Some(prev) = prev_epoch {
                    if let Some(node) = prev.get(&path) {
                        Ok(node.clone())
                    } else {
                        Err(CsmError::PathNotFound(format!(
                            "Path not found: {}",
                            path.iter()
                                .map(|o| o.to_char())
                                .collect::<String>()
                        )))
                    }
                } else {
                    Err(CsmError::ParseError(
                        "Reference used but no previous epoch".to_string(),
                    ))
                }
            }
            Some(Token::Slash) => {
                let axes = self.parse_transform()?;
                let cube = self.parse_cube(prev_epoch)?;
                Ok(cube.apply_transform(&axes))
            }
            _ => Err(CsmError::InvalidSyntax(format!(
                "Expected cube, got {:?}",
                self.current_token
            ))),
        }
    }

    fn parse_statement(&mut self, prev_epoch: &Option<HashMap<Vec<Octant>, OctreeNode>>) -> Result<(Vec<Octant>, OctreeNode)> {
        self.skip_newlines();

        if self.current_token != Some(Token::Greater) {
            return Err(CsmError::InvalidSyntax(
                "Expected > at start of statement".to_string(),
            ));
        }
        self.advance();

        let path = self.parse_path()?;
        let cube = self.parse_cube(prev_epoch)?;

        Ok((path, cube))
    }

    fn parse_epoch(&mut self, prev_epoch: &Option<HashMap<Vec<Octant>, OctreeNode>>) -> Result<HashMap<Vec<Octant>, OctreeNode>> {
        let mut assignments = HashMap::new();

        loop {
            self.skip_newlines();

            match &self.current_token {
                Some(Token::Greater) => {
                    let (path, cube) = self.parse_statement(prev_epoch)?;
                    assignments.insert(path, cube);
                }
                Some(Token::Pipe) => {
                    self.advance();
                    break;
                }
                None => break,
                _ => {
                    self.advance();
                }
            }
        }

        Ok(assignments)
    }

    fn parse_model(&mut self) -> Result<Octree> {
        let mut prev_epoch: Option<HashMap<Vec<Octant>, OctreeNode>> = None;
        let mut current_epoch = HashMap::new();

        loop {
            self.skip_newlines();

            if self.current_token.is_none() {
                break;
            }

            let epoch = self.parse_epoch(&prev_epoch)?;
            if !epoch.is_empty() {
                current_epoch = epoch.clone();
                prev_epoch = Some(epoch);
            }

            if self.current_token.is_none() {
                break;
            }
        }

        // Build octree from final epoch
        let mut builder = OctreeBuilder::new();
        for (path, node) in current_epoch {
            builder.set(path, node);
        }

        Ok(builder.build())
    }
}

/// Parse CSM text into an Octree
pub fn parse_csm(input: &str) -> Result<Octree> {
    let mut parser = Parser::new(input);
    parser.parse_model()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_value() {
        let csm = ">a 42";
        let tree = parse_csm(csm).unwrap();
        let voxels = tree.collect_voxels();
        assert!(!voxels.is_empty());
    }

    #[test]
    fn test_parse_array() {
        let csm = ">a [1 2 3 4 5 6 7 8]";
        let tree = parse_csm(csm).unwrap();
        let voxels = tree.collect_voxels();
        assert_eq!(voxels.len(), 8);
    }

    #[test]
    fn test_parse_nested() {
        let csm = r#"
            >a [1 2 3 4 5 6 7 8]
            >aa [10 11 12 13 14 15 16 17]
        "#;
        let tree = parse_csm(csm).unwrap();
        let voxels = tree.collect_voxels();
        assert!(voxels.len() > 8);
    }

    #[test]
    fn test_parse_reference() {
        let csm = r#"
            >a 100
            | >b <a
        "#;
        let tree = parse_csm(csm).unwrap();
        let voxels = tree.collect_voxels();
        assert!(!voxels.is_empty());
    }

    #[test]
    fn test_parse_transform() {
        let csm = r#"
            >a [1 2 3 4 5 6 7 8]
            | >b /x <a
        "#;
        let tree = parse_csm(csm).unwrap();
        let voxels = tree.collect_voxels();
        assert!(!voxels.is_empty());
    }
}
