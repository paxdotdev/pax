
pub enum PaxExpression {
    Primary(PaxPrimary),
    Prefix(PaxPrefix),
    Infix(PaxInfix),
    Postfix(PaxPostfix),
}

pub enum PaxPrimary {
    Literal(PaxValue),
    Idenfifier(String),
    Expression(PaxExpression),
}

pub struct PaxPrefix {
    operator: PaxOperator,
    rhs: PaxExpression,
}

pub struct PaxInfix {
    operator: PaxOperator,
    lhs: PaxExpression,
    rhs: PaxExpression,
}

pub struct PaxPostfix {
    operator: PaxOperator,
    lhs: PaxExpression,
}


