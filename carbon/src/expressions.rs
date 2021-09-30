use std::collections::HashMap;

pub struct Variable {
    pub name: String,
    pub value: PolymorphicValue,
}

pub union PolymorphicValue {
    pub float: f64,
    pub int: i64,
    pub boolean: bool,
    //TODO:  support String
    //  ^ perhaps `ManuallyDrop<String>`, a la
    //  `ManuallyDrop::new(String::from("literal"))`
    //  see https://doc.rust-lang.org/reference/items/unions.html
    //  NOTE: it appears rustc ^1.50 is needed for this
}

pub enum PolymorphicType {
    Float,
    Integer,
    Boolean
}

pub trait Property<T> {
    //either unwrap T
    //or provide a fn -> T
    fn eval_in_place(&mut self) -> &T;
    fn read(&self) -> &T;
}

pub struct PropertyLiteral<T> {
    pub value: T,
}

impl<T> Property<T> for PropertyLiteral<T> {
    fn eval_in_place(&mut self) -> &T {
        &self.value
    }
    fn read(&self) -> &T {
        &self.value
    }
}

pub struct PropertyExpression<T> {
    pub expression: Expression<T>
}

impl<T> Property<T> for PropertyExpression<T> {
    fn eval_in_place(&mut self) -> &T {
        self.expression.eval_in_place();
        &self.expression.last_value
    }
    fn read(&self) -> &T {
        &self.expression.last_value
    }
}

pub struct Expression<T> {
    pub evaluator: fn(HashMap<String, PolymorphicValue>) -> T,
    pub dependencies : HashMap<String, PolymorphicType>,
    pub last_value: T,
}

impl<T> Expression<T> {
    pub fn eval_in_place(&mut self) -> &T {
        //first: derive values
        //  - iterate through dependencies
        //  - parse dep string into a value; cast as PolymorphicType
        //  - future: track use of dependency in dep graph
        //then: call the evaluator, passing the derived values


        for (key, value) in self.dependencies.iter() {

            //TODO:  we will need a reference to Engine here
            //       -- should we make it 'static or should
            //          we pass a reference?  (or other?)

            //  this value needs to be evaluated from a combination of:
            //  - engine, for globals like current frame count
            //  - local component, for locals like vars and descendents
            //  - ... who should own this?
            //  - (should we accept a ref to both here?)
            // println!("{} / {}", key, value);
            match value {
                PolymorphicType::Float => {

                }
                PolymorphicType::Integer => {

                }
                PolymorphicType::Boolean => {

                }
            }
            // map.remove(key);
        }

        let dep_values : HashMap<String, PolymorphicValue> = HashMap::new();

        self.last_value = (self.evaluator)(dep_values);
        &self.last_value
    }
}