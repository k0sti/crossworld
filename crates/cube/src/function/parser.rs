//! Expression parser using nom
//!
//! Parses the function expression language into the AST representation.
//! Supports:
//! - Arithmetic and comparison operators with proper precedence
//! - Function calls (sin, cos, noise, etc.)
//! - if-then-else conditionals
//! - let bindings
//! - match expressions
//! - Material constants

use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case, take_while, take_while1},
    character::complete::{char, multispace0},
    combinator::{cut, map, opt, recognize, value},
    multi::{many0, separated_list0},
    number::complete::double,
    sequence::{delimited, pair, preceded, separated_pair, terminated},
    IResult, Parser,
};
use std::collections::HashMap;
use thiserror::Error;

use super::ast::{BinOpKind, BuiltinFunc, Expr, MatchPattern, UnaryOpKind, VarId};

/// Parse error with location information
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Parse error at position {position}: {message}")]
    SyntaxError { position: usize, message: String },

    #[error("Unknown function: {0}")]
    UnknownFunction(String),

    #[error("Unknown variable: {0}")]
    UnknownVariable(String),

    #[error("Invalid arity for {func}: expected {expected}, got {actual}")]
    InvalidArity {
        func: String,
        expected: String,
        actual: usize,
    },

    #[error("Unexpected end of input")]
    UnexpectedEof,
}

/// Predefined material constants
fn material_constants() -> HashMap<&'static str, f64> {
    let mut m = HashMap::new();
    // Basic materials (indices match typical voxel engine conventions)
    m.insert("AIR", 0.0);
    m.insert("STONE", 1.0);
    m.insert("GRASS", 2.0);
    m.insert("DIRT", 3.0);
    m.insert("SAND", 4.0);
    m.insert("WATER", 5.0);
    m.insert("WOOD", 6.0);
    m.insert("LEAVES", 7.0);
    m.insert("BRICK", 8.0);
    m.insert("IRON", 9.0);
    m.insert("GOLD", 10.0);
    m.insert("GLASS", 11.0);
    m.insert("SNOW", 12.0);
    m.insert("ICE", 13.0);
    m.insert("LAVA", 14.0);
    m.insert("CLAY", 15.0);
    m.insert("GRAVEL", 16.0);
    m.insert("BEDROCK", 17.0);
    m.insert("OBSIDIAN", 18.0);
    m.insert("COBBLESTONE", 19.0);
    // Mathematical constants
    m.insert("PI", std::f64::consts::PI);
    m.insert("E", std::f64::consts::E);
    m.insert("TAU", std::f64::consts::TAU);
    m
}

/// Whitespace and comments
fn ws(input: &str) -> IResult<&str, ()> {
    let (input, _) = multispace0(input)?;
    // Handle line comments
    let mut remaining = input;
    while remaining.starts_with("//") || remaining.starts_with('#') {
        let (input, _) = take_while(|c| c != '\n').parse(remaining)?;
        let (input, _) = opt(char('\n')).parse(input)?;
        let (input, _) = multispace0(input)?;
        remaining = input;
    }
    Ok((remaining, ()))
}

/// Parse an identifier (starts with letter or _, followed by letters, digits, _)
fn identifier(input: &str) -> IResult<&str, &str> {
    recognize(pair(
        take_while1(|c: char| c.is_alphabetic() || c == '_'),
        take_while(|c: char| c.is_alphanumeric() || c == '_'),
    )).parse(input)
}

/// Parse a number literal
fn number_literal(input: &str) -> IResult<&str, Expr> {
    map(double, Expr::Number).parse(input)
}

/// Parse a variable or constant
fn variable_or_constant(input: &str) -> IResult<&str, Expr> {
    let (input, name) = identifier(input)?;

    // Check for built-in variable
    if let Some(var_id) = VarId::from_name(name) {
        return Ok((input, Expr::Var(var_id)));
    }

    // Check for material constant
    if let Some(&value) = material_constants().get(name) {
        return Ok((input, Expr::Number(value)));
    }

    // Must be a user-defined variable
    Ok((input, Expr::UserVar(name.to_string())))
}

/// Parse a function call: func(arg1, arg2, ...)
fn function_call(input: &str) -> IResult<&str, Expr> {
    let (input, name) = identifier(input)?;

    // Only proceed if next char is '('
    if !input.starts_with('(') {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )));
    }

    let (input, args) = delimited(
        pair(char('('), ws),
        separated_list0((ws, char(','), ws), expr),
        pair(ws, cut(char(')'))),
    ).parse(input)?;

    // Look up the function
    if let Some(func) = BuiltinFunc::from_name(name) {
        // Validate arity
        let (min, max) = func.arity();
        if args.len() < min || args.len() > max {
            // For now, just return an error - we'll improve error handling later
            return Err(nom::Err::Failure(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Verify,
            )));
        }
        Ok((input, Expr::Call { func, args }))
    } else {
        Err(nom::Err::Failure(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Verify,
        )))
    }
}

/// Parse a parenthesized expression: (expr)
fn paren_expr(input: &str) -> IResult<&str, Expr> {
    delimited(
        pair(char('('), ws),
        expr,
        pair(ws, cut(char(')'))),
    ).parse(input)
}

/// Parse an if-then-else expression
fn if_expr(input: &str) -> IResult<&str, Expr> {
    let (input, _) = tag_no_case("if").parse(input)?;
    let (input, _) = ws(input)?;
    let (input, cond) = cut(expr).parse(input)?;
    let (input, _) = ws(input)?;
    let (input, _) = cut(tag_no_case("then")).parse(input)?;
    let (input, _) = ws(input)?;
    let (input, then_expr) = cut(expr).parse(input)?;
    let (input, _) = ws(input)?;
    let (input, _) = cut(tag_no_case("else")).parse(input)?;
    let (input, _) = ws(input)?;
    let (input, else_expr) = cut(expr).parse(input)?;

    Ok((input, Expr::if_then_else(cond, then_expr, else_expr)))
}

/// Parse a let binding: let name = value; body
fn let_expr(input: &str) -> IResult<&str, Expr> {
    let (input, _) = tag_no_case("let").parse(input)?;
    let (input, _) = ws(input)?;
    let (input, name) = cut(identifier).parse(input)?;
    let (input, _) = ws(input)?;
    let (input, _) = cut(char('=')).parse(input)?;
    let (input, _) = ws(input)?;
    let (input, value) = cut(expr).parse(input)?;
    let (input, _) = ws(input)?;
    let (input, _) = cut(char(';')).parse(input)?;
    let (input, _) = ws(input)?;
    let (input, body) = cut(expr).parse(input)?;

    Ok((input, Expr::let_in(name.to_string(), value, body)))
}

/// Parse a match pattern
fn match_pattern(input: &str) -> IResult<&str, MatchPattern> {
    alt((
        // Range pattern: n..m
        map(
            separated_pair(double, (ws, tag(".."), ws), double),
            |(low, high)| MatchPattern::Range { low, high },
        ),
        // Number pattern
        map(double, MatchPattern::Number),
    )).parse(input)
}

/// Parse a match case: pattern => expr
fn match_case(input: &str) -> IResult<&str, (MatchPattern, Expr)> {
    let (input, pattern) = match_pattern(input)?;
    let (input, _) = ws(input)?;
    let (input, _) = tag("=>").parse(input)?;
    let (input, _) = ws(input)?;
    let (input, result) = expr(input)?;
    Ok((input, (pattern, result)))
}

/// Parse a match expression: match expr { cases... }
fn match_expr(input: &str) -> IResult<&str, Expr> {
    let (input, _) = tag_no_case("match").parse(input)?;
    let (input, _) = ws(input)?;
    let (input, match_value) = cut(expr).parse(input)?;
    let (input, _) = ws(input)?;
    let (input, _) = cut(char('{')).parse(input)?;
    let (input, _) = ws(input)?;

    // Parse cases
    let (input, cases) = many0(terminated(
        match_case,
        (ws, char(','), ws),
    )).parse(input)?;

    // Parse default case: _ => expr
    let (input, _) = ws(input)?;
    let (input, _) = cut(char('_')).parse(input)?;
    let (input, _) = ws(input)?;
    let (input, _) = cut(tag("=>")).parse(input)?;
    let (input, _) = ws(input)?;
    let (input, default) = cut(expr).parse(input)?;
    let (input, _) = ws(input)?;
    let (input, _) = opt(char(',')).parse(input)?;
    let (input, _) = ws(input)?;
    let (input, _) = cut(char('}')).parse(input)?;

    Ok((
        input,
        Expr::Match {
            expr: Box::new(match_value),
            cases,
            default: Box::new(default),
        },
    ))
}

/// Parse a primary (atomic) expression
fn primary(input: &str) -> IResult<&str, Expr> {
    preceded(
        ws,
        alt((
            if_expr,
            let_expr,
            match_expr,
            function_call,
            paren_expr,
            number_literal,
            variable_or_constant,
        )),
    ).parse(input)
}

/// Parse a unary expression: -expr, not expr
fn unary(input: &str) -> IResult<&str, Expr> {
    preceded(
        ws,
        alt((
            // Negative: -expr
            map(preceded(char('-'), unary), |e| {
                Expr::unary(UnaryOpKind::Neg, e)
            }),
            // Logical not: not expr
            map(
                preceded((tag_no_case("not"), ws), unary),
                |e| Expr::unary(UnaryOpKind::Not, e),
            ),
            primary,
        )),
    ).parse(input)
}

/// Parse power operator (right-associative)
fn power(input: &str) -> IResult<&str, Expr> {
    let (input, base) = unary(input)?;
    let (input, _) = ws(input)?;

    if let Ok((input, _)) = char::<&str, nom::error::Error<&str>>('^')(input) {
        let (input, _) = ws(input)?;
        let (input, exp) = power(input)?; // Right-associative
        Ok((input, Expr::binop(BinOpKind::Pow, base, exp)))
    } else {
        Ok((input, base))
    }
}

/// Parse multiplication/division/modulo
fn term(input: &str) -> IResult<&str, Expr> {
    let (input, first) = power(input)?;
    let (input, rest) = many0((
        ws,
        alt((
            value(BinOpKind::Mul, char('*')),
            value(BinOpKind::Div, char('/')),
            value(BinOpKind::Mod, char('%')),
        )),
        ws,
        power,
    )).parse(input)?;

    let result = rest
        .into_iter()
        .fold(first, |acc, (_, op, _, rhs)| Expr::binop(op, acc, rhs));

    Ok((input, result))
}

/// Parse addition/subtraction
fn additive(input: &str) -> IResult<&str, Expr> {
    let (input, first) = term(input)?;
    let (input, rest) = many0((
        ws,
        alt((
            value(BinOpKind::Add, char('+')),
            value(BinOpKind::Sub, char('-')),
        )),
        ws,
        term,
    )).parse(input)?;

    let result = rest
        .into_iter()
        .fold(first, |acc, (_, op, _, rhs)| Expr::binop(op, acc, rhs));

    Ok((input, result))
}

/// Parse comparison operators
fn comparison(input: &str) -> IResult<&str, Expr> {
    let (input, first) = additive(input)?;
    let (input, rest) = many0((
        ws,
        alt((
            value(BinOpKind::Le, tag("<=")),
            value(BinOpKind::Ge, tag(">=")),
            value(BinOpKind::Ne, tag("!=")),
            value(BinOpKind::Eq, tag("==")),
            value(BinOpKind::Lt, char('<')),
            value(BinOpKind::Gt, char('>')),
        )),
        ws,
        additive,
    )).parse(input)?;

    let result = rest
        .into_iter()
        .fold(first, |acc, (_, op, _, rhs)| Expr::binop(op, acc, rhs));

    Ok((input, result))
}

/// Parse logical and
fn logical_and(input: &str) -> IResult<&str, Expr> {
    let (input, first) = comparison(input)?;
    let (input, rest) = many0((
        ws,
        value(BinOpKind::And, tag_no_case("and")),
        ws,
        comparison,
    )).parse(input)?;

    let result = rest
        .into_iter()
        .fold(first, |acc, (_, op, _, rhs)| Expr::binop(op, acc, rhs));

    Ok((input, result))
}

/// Parse logical or
fn logical_or(input: &str) -> IResult<&str, Expr> {
    let (input, first) = logical_and(input)?;
    let (input, rest) = many0((
        ws,
        value(BinOpKind::Or, tag_no_case("or")),
        ws,
        logical_and,
    )).parse(input)?;

    let result = rest
        .into_iter()
        .fold(first, |acc, (_, op, _, rhs)| Expr::binop(op, acc, rhs));

    Ok((input, result))
}

/// Parse a complete expression
fn expr(input: &str) -> IResult<&str, Expr> {
    logical_or(input)
}

/// Parse an expression string into an AST
pub fn parse_expr(input: &str) -> Result<Expr, ParseError> {
    let (remaining, result) = preceded(ws, expr).parse(input).map_err(|e| match e {
        nom::Err::Incomplete(_) => ParseError::UnexpectedEof,
        nom::Err::Error(e) | nom::Err::Failure(e) => {
            let position = input.len() - e.input.len();
            ParseError::SyntaxError {
                position,
                message: format!("{:?}", e.code),
            }
        }
    })?;

    // Ensure we consumed all input
    let (remaining, _) = ws(remaining).map_err(|_| ParseError::UnexpectedEof)?;
    if !remaining.is_empty() {
        let position = input.len() - remaining.len();
        return Err(ParseError::SyntaxError {
            position,
            message: format!("Unexpected input: {}", &remaining[..remaining.len().min(20)]),
        });
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_number() {
        let result = parse_expr("42").unwrap();
        assert_eq!(result, Expr::Number(42.0));

        let result = parse_expr("3.14").unwrap();
        assert_eq!(result, Expr::Number(3.14));

        let result = parse_expr("-2.5").unwrap();
        assert_eq!(result, Expr::unary(UnaryOpKind::Neg, Expr::Number(2.5)));
    }

    #[test]
    fn test_parse_variable() {
        let result = parse_expr("x").unwrap();
        assert_eq!(result, Expr::Var(VarId::X));

        let result = parse_expr("time").unwrap();
        assert_eq!(result, Expr::Var(VarId::Time));

        let result = parse_expr("wx").unwrap();
        assert_eq!(result, Expr::Var(VarId::WorldX));
    }

    #[test]
    fn test_parse_constant() {
        let result = parse_expr("PI").unwrap();
        assert_eq!(result, Expr::Number(std::f64::consts::PI));

        let result = parse_expr("STONE").unwrap();
        assert_eq!(result, Expr::Number(1.0));
    }

    #[test]
    fn test_parse_arithmetic() {
        let result = parse_expr("x + 1").unwrap();
        assert_eq!(
            result,
            Expr::binop(BinOpKind::Add, Expr::Var(VarId::X), Expr::Number(1.0))
        );

        let result = parse_expr("x * y + z").unwrap();
        assert_eq!(
            result,
            Expr::binop(
                BinOpKind::Add,
                Expr::binop(BinOpKind::Mul, Expr::Var(VarId::X), Expr::Var(VarId::Y)),
                Expr::Var(VarId::Z)
            )
        );
    }

    #[test]
    fn test_parse_precedence() {
        // Test that * binds tighter than +
        let result = parse_expr("1 + 2 * 3").unwrap();
        assert_eq!(
            result,
            Expr::binop(
                BinOpKind::Add,
                Expr::Number(1.0),
                Expr::binop(BinOpKind::Mul, Expr::Number(2.0), Expr::Number(3.0))
            )
        );

        // Test that ^ binds tighter than *
        let result = parse_expr("2 * 3 ^ 4").unwrap();
        assert_eq!(
            result,
            Expr::binop(
                BinOpKind::Mul,
                Expr::Number(2.0),
                Expr::binop(BinOpKind::Pow, Expr::Number(3.0), Expr::Number(4.0))
            )
        );
    }

    #[test]
    fn test_parse_power_right_assoc() {
        // Test that ^ is right-associative: 2^3^4 = 2^(3^4)
        let result = parse_expr("2 ^ 3 ^ 4").unwrap();
        assert_eq!(
            result,
            Expr::binop(
                BinOpKind::Pow,
                Expr::Number(2.0),
                Expr::binop(BinOpKind::Pow, Expr::Number(3.0), Expr::Number(4.0))
            )
        );
    }

    #[test]
    fn test_parse_comparison() {
        let result = parse_expr("x < 1").unwrap();
        assert_eq!(
            result,
            Expr::binop(BinOpKind::Lt, Expr::Var(VarId::X), Expr::Number(1.0))
        );

        let result = parse_expr("x >= y").unwrap();
        assert_eq!(
            result,
            Expr::binop(BinOpKind::Ge, Expr::Var(VarId::X), Expr::Var(VarId::Y))
        );
    }

    #[test]
    fn test_parse_logical() {
        let result = parse_expr("x > 0 and y > 0").unwrap();
        assert_eq!(
            result,
            Expr::binop(
                BinOpKind::And,
                Expr::binop(BinOpKind::Gt, Expr::Var(VarId::X), Expr::Number(0.0)),
                Expr::binop(BinOpKind::Gt, Expr::Var(VarId::Y), Expr::Number(0.0))
            )
        );
    }

    #[test]
    fn test_parse_function_call() {
        let result = parse_expr("sin(x)").unwrap();
        assert_eq!(
            result,
            Expr::call(BuiltinFunc::Sin, vec![Expr::Var(VarId::X)])
        );

        let result = parse_expr("noise(x, y, z)").unwrap();
        assert_eq!(
            result,
            Expr::call(
                BuiltinFunc::Noise,
                vec![Expr::Var(VarId::X), Expr::Var(VarId::Y), Expr::Var(VarId::Z)]
            )
        );

        let result = parse_expr("clamp(x, 0, 1)").unwrap();
        assert_eq!(
            result,
            Expr::call(
                BuiltinFunc::Clamp,
                vec![Expr::Var(VarId::X), Expr::Number(0.0), Expr::Number(1.0)]
            )
        );
    }

    #[test]
    fn test_parse_if_expr() {
        let result = parse_expr("if x > 0 then STONE else AIR").unwrap();
        assert_eq!(
            result,
            Expr::if_then_else(
                Expr::binop(BinOpKind::Gt, Expr::Var(VarId::X), Expr::Number(0.0)),
                Expr::Number(1.0), // STONE
                Expr::Number(0.0)  // AIR
            )
        );
    }

    #[test]
    fn test_parse_let_expr() {
        let result = parse_expr("let a = x + 1; a * 2").unwrap();
        assert_eq!(
            result,
            Expr::let_in(
                "a",
                Expr::binop(BinOpKind::Add, Expr::Var(VarId::X), Expr::Number(1.0)),
                Expr::binop(BinOpKind::Mul, Expr::UserVar("a".into()), Expr::Number(2.0))
            )
        );
    }

    #[test]
    fn test_parse_match_expr() {
        let result = parse_expr("match floor(y) { 0 => BEDROCK, 1 => STONE, _ => GRASS }").unwrap();

        match result {
            Expr::Match { cases, default, .. } => {
                assert_eq!(cases.len(), 2);
                assert_eq!(cases[0].0, MatchPattern::Number(0.0));
                assert_eq!(cases[1].0, MatchPattern::Number(1.0));
                assert_eq!(*default, Expr::Number(2.0)); // GRASS
            }
            _ => panic!("Expected Match expression"),
        }
    }

    #[test]
    fn test_parse_complex_expression() {
        let result = parse_expr(
            "let height = noise(x * 0.1, z * 0.1, 0) * 10; if y < height then STONE else AIR",
        );
        assert!(result.is_ok(), "Failed to parse: {:?}", result);
    }

    #[test]
    fn test_parse_whitespace() {
        let result = parse_expr("  x + 1  ").unwrap();
        assert_eq!(
            result,
            Expr::binop(BinOpKind::Add, Expr::Var(VarId::X), Expr::Number(1.0))
        );
    }

    #[test]
    fn test_parse_nested_functions() {
        let result = parse_expr("sin(cos(x))").unwrap();
        assert_eq!(
            result,
            Expr::call(
                BuiltinFunc::Sin,
                vec![Expr::call(BuiltinFunc::Cos, vec![Expr::Var(VarId::X)])]
            )
        );
    }

    #[test]
    fn test_parse_parentheses() {
        let result = parse_expr("(x + 1) * 2").unwrap();
        assert_eq!(
            result,
            Expr::binop(
                BinOpKind::Mul,
                Expr::binop(BinOpKind::Add, Expr::Var(VarId::X), Expr::Number(1.0)),
                Expr::Number(2.0)
            )
        );
    }

    #[test]
    fn test_parse_error() {
        let result = parse_expr("x +");
        assert!(result.is_err());

        let result = parse_expr("unknown_func(x)");
        assert!(result.is_err());
    }
}
