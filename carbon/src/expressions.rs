

pub struct Variable {
    pub name: String,
    pub value: PolymorphicValue,
}

pub union PolymorphicValue {
    pub float: f64,
    pub int: i64,
    pub boolean: bool,
    //TODO:  support String
}