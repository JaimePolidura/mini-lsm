pub enum Expression {
    None,

    Binary(Box<Expression>, Box<Expression>, BinaryOperator),
    Unary(UnaryOperator, Box<Expression>),
    String(String),
    Identifier(String),
    NumberF64(f64),
    NumberI64(f64),
}

pub enum UnaryOperator {
    Plus,
    Minus,
}

pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    And,
    Or,
    NotEqual,
    Equal,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
}