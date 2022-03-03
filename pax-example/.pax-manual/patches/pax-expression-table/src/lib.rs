use pax_core;
use pax_core::{Evaluator, InjectionContext};
use pax_properties_coproduct::{PropertiesCoproduct, PatchCoproduct};

pub struct MyManualMacroExpression {
    pub transpiled_evaluator: fn(properties: &PropertiesCoproduct) -> PatchCoproduct,
}

//Need to look up functions from RIL
//  - can direct-reference by symbol, via RIL codegen
//Need to look up functions dynamically from designtime
//  - can code-gen a match statement or hashtable
//    still have the problem of polymorphic return types
//Need to pack functions into codegenned expression table
//  - can generate ID, store in manifest, and codegen as a LUT key
//    here in expressiontable

//Need to know the return type (in fact, no!  by the following approach,
//we just need to know the field name in the consumer.  The consumer can unpack
// using my_patch.field_name.unwrap()

//One possibility — transact in Patches, which can be
// codegenned to unwrap/wrap values.  Sort of the same
// trick as PropertiesCoproduct, except for return values,
// and would require another codegenned coproduct like PropertiesPatchCoproduct
// evaluator: fn(PropertiesCoproduct) -> PropertiesPatchCoproduct
// the above would allow all fn's to be stored/indexed in the same DS

//A challenge:

impl Evaluator<PatchCoproduct> for MyManualMacroExpression {
    fn inject_and_evaluate(&self, ic: &InjectionContext) -> T {
        //TODO:CODEGEN
        //       pull necessary data from `ic`,
        //       map into the variadic args of self.variadic_evaluator()
        //       Perhaps this is a LUT of `String => (Fn(InjectionContext) -> V)` for any variadic type (injection stream) V
        let engine = ic.engine;
        let borrowed = ic.stack_frame.peek().borrow();
        let properties_borrowed = borrowed.properties.borrow();
        (self.transpiled_evaluator)(&properties_borrowed)
    }
}


fn test() {
    let m = MyManualMacroExpression {
        transpiled_evaluator: |x: isize|{5},
    };
}

// pub struct InjectionContext<'a> {
//     //TODO: add scope tree, etc.
//     pub engine: &'a PaxEngine,
//     pub stack_frame: Rc<RefCell<StackFrame>>,
// }