//! Abstract Syntax Tree for function expressions
//!
//! The AST is designed to be backend-agnostic, supporting both CPU (fasteval)
//! and GPU (WGSL) code generation from the same representation.

use std::fmt;

/// Variable identifiers available in expressions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VarId {
    /// Local x coordinate [-1, 1]
    X,
    /// Local y coordinate [-1, 1]
    Y,
    /// Local z coordinate [-1, 1]
    Z,
    /// World x coordinate
    WorldX,
    /// World y coordinate
    WorldY,
    /// World z coordinate
    WorldZ,
    /// Animation time in seconds
    Time,
    /// Current octree depth
    Depth,
    /// Random seed
    Seed,
}

impl VarId {
    /// Get the string name of the variable
    pub fn name(self) -> &'static str {
        match self {
            VarId::X => "x",
            VarId::Y => "y",
            VarId::Z => "z",
            VarId::WorldX => "wx",
            VarId::WorldY => "wy",
            VarId::WorldZ => "wz",
            VarId::Time => "time",
            VarId::Depth => "depth",
            VarId::Seed => "seed",
        }
    }

    /// Parse a variable name to VarId
    pub fn from_name(name: &str) -> Option<VarId> {
        match name {
            "x" => Some(VarId::X),
            "y" => Some(VarId::Y),
            "z" => Some(VarId::Z),
            "wx" => Some(VarId::WorldX),
            "wy" => Some(VarId::WorldY),
            "wz" => Some(VarId::WorldZ),
            "time" => Some(VarId::Time),
            "depth" => Some(VarId::Depth),
            "seed" => Some(VarId::Seed),
            _ => None,
        }
    }
}

impl fmt::Display for VarId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Binary operator kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOpKind {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,

    // Comparison
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,

    // Logical
    And,
    Or,
}

impl BinOpKind {
    /// Get the operator precedence (higher = binds tighter)
    pub fn precedence(self) -> u8 {
        match self {
            BinOpKind::Or => 1,
            BinOpKind::And => 2,
            BinOpKind::Lt | BinOpKind::Le | BinOpKind::Gt | BinOpKind::Ge => 3,
            BinOpKind::Eq | BinOpKind::Ne => 4,
            BinOpKind::Add | BinOpKind::Sub => 5,
            BinOpKind::Mul | BinOpKind::Div | BinOpKind::Mod => 6,
            BinOpKind::Pow => 7,
        }
    }

    /// Check if operator is right-associative
    pub fn is_right_assoc(self) -> bool {
        matches!(self, BinOpKind::Pow)
    }

    /// Get the symbol for the operator
    pub fn symbol(self) -> &'static str {
        match self {
            BinOpKind::Add => "+",
            BinOpKind::Sub => "-",
            BinOpKind::Mul => "*",
            BinOpKind::Div => "/",
            BinOpKind::Mod => "%",
            BinOpKind::Pow => "^",
            BinOpKind::Lt => "<",
            BinOpKind::Le => "<=",
            BinOpKind::Gt => ">",
            BinOpKind::Ge => ">=",
            BinOpKind::Eq => "==",
            BinOpKind::Ne => "!=",
            BinOpKind::And => "and",
            BinOpKind::Or => "or",
        }
    }
}

impl fmt::Display for BinOpKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.symbol())
    }
}

/// Unary operator kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOpKind {
    /// Numeric negation: -x
    Neg,
    /// Logical negation: not x
    Not,
}

impl UnaryOpKind {
    /// Get the symbol for the operator
    pub fn symbol(self) -> &'static str {
        match self {
            UnaryOpKind::Neg => "-",
            UnaryOpKind::Not => "not",
        }
    }
}

impl fmt::Display for UnaryOpKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.symbol())
    }
}

/// Built-in function identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinFunc {
    // Trigonometric
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Atan2,

    // Exponential/Power
    Sqrt,
    Pow,
    Exp,
    Ln,
    Log2,
    Log10,

    // Rounding
    Floor,
    Ceil,
    Round,
    Trunc,
    Fract,

    // Absolute/Sign
    Abs,
    Sign,

    // Min/Max/Clamp
    Min,
    Max,
    Clamp,

    // Interpolation
    Lerp,
    Smoothstep,
    Step,

    // Noise functions
    Noise,
    Fbm,
    Turbulence,
}

impl BuiltinFunc {
    /// Get the function name
    pub fn name(self) -> &'static str {
        match self {
            BuiltinFunc::Sin => "sin",
            BuiltinFunc::Cos => "cos",
            BuiltinFunc::Tan => "tan",
            BuiltinFunc::Asin => "asin",
            BuiltinFunc::Acos => "acos",
            BuiltinFunc::Atan => "atan",
            BuiltinFunc::Atan2 => "atan2",
            BuiltinFunc::Sqrt => "sqrt",
            BuiltinFunc::Pow => "pow",
            BuiltinFunc::Exp => "exp",
            BuiltinFunc::Ln => "ln",
            BuiltinFunc::Log2 => "log2",
            BuiltinFunc::Log10 => "log10",
            BuiltinFunc::Floor => "floor",
            BuiltinFunc::Ceil => "ceil",
            BuiltinFunc::Round => "round",
            BuiltinFunc::Trunc => "trunc",
            BuiltinFunc::Fract => "fract",
            BuiltinFunc::Abs => "abs",
            BuiltinFunc::Sign => "sign",
            BuiltinFunc::Min => "min",
            BuiltinFunc::Max => "max",
            BuiltinFunc::Clamp => "clamp",
            BuiltinFunc::Lerp => "lerp",
            BuiltinFunc::Smoothstep => "smoothstep",
            BuiltinFunc::Step => "step",
            BuiltinFunc::Noise => "noise",
            BuiltinFunc::Fbm => "fbm",
            BuiltinFunc::Turbulence => "turbulence",
        }
    }

    /// Parse a function name to BuiltinFunc
    pub fn from_name(name: &str) -> Option<BuiltinFunc> {
        match name {
            "sin" => Some(BuiltinFunc::Sin),
            "cos" => Some(BuiltinFunc::Cos),
            "tan" => Some(BuiltinFunc::Tan),
            "asin" => Some(BuiltinFunc::Asin),
            "acos" => Some(BuiltinFunc::Acos),
            "atan" => Some(BuiltinFunc::Atan),
            "atan2" => Some(BuiltinFunc::Atan2),
            "sqrt" => Some(BuiltinFunc::Sqrt),
            "pow" => Some(BuiltinFunc::Pow),
            "exp" => Some(BuiltinFunc::Exp),
            "ln" => Some(BuiltinFunc::Ln),
            "log2" => Some(BuiltinFunc::Log2),
            "log10" => Some(BuiltinFunc::Log10),
            "floor" => Some(BuiltinFunc::Floor),
            "ceil" => Some(BuiltinFunc::Ceil),
            "round" => Some(BuiltinFunc::Round),
            "trunc" => Some(BuiltinFunc::Trunc),
            "fract" => Some(BuiltinFunc::Fract),
            "abs" => Some(BuiltinFunc::Abs),
            "sign" => Some(BuiltinFunc::Sign),
            "min" => Some(BuiltinFunc::Min),
            "max" => Some(BuiltinFunc::Max),
            "clamp" => Some(BuiltinFunc::Clamp),
            "lerp" | "mix" => Some(BuiltinFunc::Lerp),
            "smoothstep" => Some(BuiltinFunc::Smoothstep),
            "step" => Some(BuiltinFunc::Step),
            "noise" => Some(BuiltinFunc::Noise),
            "fbm" => Some(BuiltinFunc::Fbm),
            "turbulence" => Some(BuiltinFunc::Turbulence),
            _ => None,
        }
    }

    /// Get the expected number of arguments
    pub fn arity(self) -> (usize, usize) {
        match self {
            // Single argument functions
            BuiltinFunc::Sin
            | BuiltinFunc::Cos
            | BuiltinFunc::Tan
            | BuiltinFunc::Asin
            | BuiltinFunc::Acos
            | BuiltinFunc::Atan
            | BuiltinFunc::Sqrt
            | BuiltinFunc::Exp
            | BuiltinFunc::Ln
            | BuiltinFunc::Log2
            | BuiltinFunc::Log10
            | BuiltinFunc::Floor
            | BuiltinFunc::Ceil
            | BuiltinFunc::Round
            | BuiltinFunc::Trunc
            | BuiltinFunc::Fract
            | BuiltinFunc::Abs
            | BuiltinFunc::Sign => (1, 1),

            // Two argument functions
            BuiltinFunc::Atan2 | BuiltinFunc::Pow | BuiltinFunc::Min | BuiltinFunc::Max => (2, 2),

            // Three argument functions
            BuiltinFunc::Clamp | BuiltinFunc::Lerp | BuiltinFunc::Smoothstep => (3, 3),

            // Step takes 2 args
            BuiltinFunc::Step => (2, 2),

            // Noise takes 3 coords
            BuiltinFunc::Noise => (3, 3),

            // FBM and turbulence take 3 coords + octaves
            BuiltinFunc::Fbm | BuiltinFunc::Turbulence => (4, 4),
        }
    }
}

impl fmt::Display for BuiltinFunc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Expression AST node
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Literal number
    Number(f64),

    /// Variable reference
    Var(VarId),

    /// User-defined variable reference (from let bindings)
    UserVar(String),

    /// Binary operation
    BinOp {
        op: BinOpKind,
        left: Box<Expr>,
        right: Box<Expr>,
    },

    /// Unary operation
    UnaryOp { op: UnaryOpKind, expr: Box<Expr> },

    /// Built-in function call
    Call { func: BuiltinFunc, args: Vec<Expr> },

    /// Conditional: if cond then a else b
    If {
        cond: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
    },

    /// Let binding: let name = value; body
    Let {
        name: String,
        value: Box<Expr>,
        body: Box<Expr>,
    },

    /// Match expression: match expr { cases... }
    Match {
        expr: Box<Expr>,
        cases: Vec<(MatchPattern, Expr)>,
        default: Box<Expr>,
    },
}

/// Pattern for match expressions
#[derive(Debug, Clone, PartialEq)]
pub enum MatchPattern {
    /// Match a specific number
    Number(f64),
    /// Match a range [low, high)
    Range { low: f64, high: f64 },
}

impl Expr {
    /// Create a number literal
    pub fn number(n: f64) -> Self {
        Expr::Number(n)
    }

    /// Create a variable reference
    pub fn var(id: VarId) -> Self {
        Expr::Var(id)
    }

    /// Create a user variable reference
    pub fn user_var(name: impl Into<String>) -> Self {
        Expr::UserVar(name.into())
    }

    /// Create a binary operation
    pub fn binop(op: BinOpKind, left: Expr, right: Expr) -> Self {
        Expr::BinOp {
            op,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// Create a unary operation
    pub fn unary(op: UnaryOpKind, expr: Expr) -> Self {
        Expr::UnaryOp {
            op,
            expr: Box::new(expr),
        }
    }

    /// Create a function call
    pub fn call(func: BuiltinFunc, args: Vec<Expr>) -> Self {
        Expr::Call { func, args }
    }

    /// Create a conditional
    pub fn if_then_else(cond: Expr, then_expr: Expr, else_expr: Expr) -> Self {
        Expr::If {
            cond: Box::new(cond),
            then_expr: Box::new(then_expr),
            else_expr: Box::new(else_expr),
        }
    }

    /// Create a let binding
    pub fn let_in(name: impl Into<String>, value: Expr, body: Expr) -> Self {
        Expr::Let {
            name: name.into(),
            value: Box::new(value),
            body: Box::new(body),
        }
    }

    /// Check if expression contains a specific variable
    pub fn contains_var(&self, var: VarId) -> bool {
        match self {
            Expr::Number(_) => false,
            Expr::Var(v) => *v == var,
            Expr::UserVar(_) => false,
            Expr::BinOp { left, right, .. } => left.contains_var(var) || right.contains_var(var),
            Expr::UnaryOp { expr, .. } => expr.contains_var(var),
            Expr::Call { args, .. } => args.iter().any(|a| a.contains_var(var)),
            Expr::If {
                cond,
                then_expr,
                else_expr,
            } => {
                cond.contains_var(var) || then_expr.contains_var(var) || else_expr.contains_var(var)
            }
            Expr::Let { value, body, .. } => value.contains_var(var) || body.contains_var(var),
            Expr::Match {
                expr,
                cases,
                default,
            } => {
                expr.contains_var(var)
                    || cases.iter().any(|(_, e)| e.contains_var(var))
                    || default.contains_var(var)
            }
        }
    }

    /// Check if expression uses time (requires re-evaluation each frame)
    pub fn uses_time(&self) -> bool {
        self.contains_var(VarId::Time)
    }

    /// Check if expression contains a specific function
    pub fn contains_func(&self, func: BuiltinFunc) -> bool {
        match self {
            Expr::Number(_) | Expr::Var(_) | Expr::UserVar(_) => false,
            Expr::BinOp { left, right, .. } => {
                left.contains_func(func) || right.contains_func(func)
            }
            Expr::UnaryOp { expr, .. } => expr.contains_func(func),
            Expr::Call { func: f, args } => {
                *f == func || args.iter().any(|a| a.contains_func(func))
            }
            Expr::If {
                cond,
                then_expr,
                else_expr,
            } => {
                cond.contains_func(func)
                    || then_expr.contains_func(func)
                    || else_expr.contains_func(func)
            }
            Expr::Let { value, body, .. } => value.contains_func(func) || body.contains_func(func),
            Expr::Match {
                expr,
                cases,
                default,
            } => {
                expr.contains_func(func)
                    || cases.iter().any(|(_, e)| e.contains_func(func))
                    || default.contains_func(func)
            }
        }
    }

    /// Check if expression uses any noise function
    pub fn uses_noise(&self) -> bool {
        self.contains_func(BuiltinFunc::Noise)
            || self.contains_func(BuiltinFunc::Fbm)
            || self.contains_func(BuiltinFunc::Turbulence)
    }

    /// Estimate computational complexity (rough heuristic for backend selection)
    pub fn estimate_complexity(&self) -> u32 {
        match self {
            Expr::Number(_) => 1,
            Expr::Var(_) | Expr::UserVar(_) => 1,
            Expr::BinOp { left, right, op } => {
                let base = match op {
                    BinOpKind::Add | BinOpKind::Sub => 1,
                    BinOpKind::Mul => 2,
                    BinOpKind::Div | BinOpKind::Mod => 3,
                    BinOpKind::Pow => 5,
                    _ => 1, // Comparison/logical
                };
                base + left.estimate_complexity() + right.estimate_complexity()
            }
            Expr::UnaryOp { expr, .. } => 1 + expr.estimate_complexity(),
            Expr::Call { func, args } => {
                let func_cost = match func {
                    BuiltinFunc::Noise => 20,
                    BuiltinFunc::Fbm => 50,
                    BuiltinFunc::Turbulence => 60,
                    BuiltinFunc::Sin | BuiltinFunc::Cos | BuiltinFunc::Tan => 5,
                    BuiltinFunc::Sqrt | BuiltinFunc::Exp | BuiltinFunc::Ln => 5,
                    _ => 2,
                };
                func_cost + args.iter().map(|a| a.estimate_complexity()).sum::<u32>()
            }
            Expr::If {
                cond,
                then_expr,
                else_expr,
            } => {
                1 + cond.estimate_complexity()
                    + then_expr.estimate_complexity()
                    + else_expr.estimate_complexity()
            }
            Expr::Let { value, body, .. } => {
                value.estimate_complexity() + body.estimate_complexity()
            }
            Expr::Match {
                expr,
                cases,
                default,
            } => {
                expr.estimate_complexity()
                    + cases
                        .iter()
                        .map(|(_, e)| e.estimate_complexity())
                        .sum::<u32>()
                    + default.estimate_complexity()
            }
        }
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Number(n) => write!(f, "{}", n),
            Expr::Var(v) => write!(f, "{}", v),
            Expr::UserVar(name) => write!(f, "{}", name),
            Expr::BinOp { op, left, right } => {
                write!(f, "({} {} {})", left, op, right)
            }
            Expr::UnaryOp { op, expr } => {
                write!(f, "({} {})", op, expr)
            }
            Expr::Call { func, args } => {
                write!(f, "{}(", func)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
            Expr::If {
                cond,
                then_expr,
                else_expr,
            } => {
                write!(f, "if {} then {} else {}", cond, then_expr, else_expr)
            }
            Expr::Let { name, value, body } => {
                write!(f, "let {} = {}; {}", name, value, body)
            }
            Expr::Match {
                expr,
                cases,
                default,
            } => {
                write!(f, "match {} {{ ", expr)?;
                for (pat, e) in cases {
                    write!(f, "{} => {}, ", pat, e)?;
                }
                write!(f, "_ => {} }}", default)
            }
        }
    }
}

impl fmt::Display for MatchPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MatchPattern::Number(n) => write!(f, "{}", n),
            MatchPattern::Range { low, high } => write!(f, "{}..{}", low, high),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_var_id_roundtrip() {
        for var in [
            VarId::X,
            VarId::Y,
            VarId::Z,
            VarId::WorldX,
            VarId::WorldY,
            VarId::WorldZ,
            VarId::Time,
            VarId::Depth,
            VarId::Seed,
        ] {
            assert_eq!(VarId::from_name(var.name()), Some(var));
        }
    }

    #[test]
    fn test_builtin_func_roundtrip() {
        for func in [
            BuiltinFunc::Sin,
            BuiltinFunc::Cos,
            BuiltinFunc::Noise,
            BuiltinFunc::Fbm,
            BuiltinFunc::Lerp,
            BuiltinFunc::Clamp,
        ] {
            assert_eq!(BuiltinFunc::from_name(func.name()), Some(func));
        }
    }

    #[test]
    fn test_expr_contains_var() {
        let expr = Expr::binop(BinOpKind::Add, Expr::var(VarId::X), Expr::number(1.0));
        assert!(expr.contains_var(VarId::X));
        assert!(!expr.contains_var(VarId::Y));
        assert!(!expr.contains_var(VarId::Time));
    }

    #[test]
    fn test_expr_uses_time() {
        let no_time = Expr::binop(BinOpKind::Add, Expr::var(VarId::X), Expr::var(VarId::Y));
        assert!(!no_time.uses_time());

        let with_time = Expr::binop(BinOpKind::Mul, Expr::var(VarId::X), Expr::var(VarId::Time));
        assert!(with_time.uses_time());
    }

    #[test]
    fn test_expr_uses_noise() {
        let no_noise = Expr::binop(BinOpKind::Add, Expr::var(VarId::X), Expr::number(1.0));
        assert!(!no_noise.uses_noise());

        let with_noise = Expr::call(
            BuiltinFunc::Noise,
            vec![
                Expr::var(VarId::X),
                Expr::var(VarId::Y),
                Expr::var(VarId::Z),
            ],
        );
        assert!(with_noise.uses_noise());
    }

    #[test]
    fn test_expr_complexity() {
        let simple = Expr::number(1.0);
        let complex = Expr::call(
            BuiltinFunc::Fbm,
            vec![
                Expr::var(VarId::X),
                Expr::var(VarId::Y),
                Expr::var(VarId::Z),
                Expr::number(4.0),
            ],
        );

        assert!(complex.estimate_complexity() > simple.estimate_complexity());
    }

    #[test]
    fn test_expr_display() {
        let expr = Expr::binop(BinOpKind::Add, Expr::var(VarId::X), Expr::number(1.0));
        assert_eq!(format!("{}", expr), "(x + 1)");

        let call = Expr::call(BuiltinFunc::Sin, vec![Expr::var(VarId::X)]);
        assert_eq!(format!("{}", call), "sin(x)");
    }

    #[test]
    fn test_binop_precedence() {
        assert!(BinOpKind::Mul.precedence() > BinOpKind::Add.precedence());
        assert!(BinOpKind::Pow.precedence() > BinOpKind::Mul.precedence());
        assert!(BinOpKind::And.precedence() > BinOpKind::Or.precedence());
    }
}
