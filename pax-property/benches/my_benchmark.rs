use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pax_property::{print_graph, Property, PropertyId};

pub fn property_set_benchmark(c: &mut Criterion) {
    let properties = generate_dependency_graph(10, 10);
    print_graph();
    
    c.bench_function("set property", |b| b.iter(|| {
        for prop in &properties {
            prop.set(black_box(1));
        }
    }));
}

fn generate_dependency_graph(depth: usize, width:usize) -> Vec<Property<i32>> {
    // dxw grid of properties where each property is the sum of the previous layer
    let mut properties: Vec<Property<i32>>  = Vec::with_capacity(depth * width);
    let mut dependents: Vec<Property<i32>> = Vec::with_capacity(width);
    for i in 0..depth {
        let layer_deps = dependents.clone();
        dependents.clear();
        for j in 0..width {
            let prop = if i == 0 {
                Property::new(0)
            } else {
                let deps_clone = layer_deps.clone();
                let evaluator = move || {
                    deps_clone.iter().map(|p| p.get()).sum()
                };
                Property::expression(evaluator, &get_untyped_dependencies(layer_deps.clone()))
            };
            if j % 2 == 0 {
                prop.subscribe(|| {
                    // calculate 10th fibonacci number
                    let mut a = 0;
                    let mut b = 1;
                    for _ in 0..10 {
                        let c = a + b;
                        a = b;
                        b = c;
                    }
                });
            }
            dependents.push(prop.clone());
            properties.push(prop);
        }
    }
    properties
}

fn get_untyped_dependencies(properties: Vec<Property<i32>>) -> Vec<PropertyId> {
    let v: Vec<PropertyId> = properties.iter().map(|p| p.get_id()).collect();
    v
}


criterion_group!(benches, property_set_benchmark);

criterion_main!(benches);




// #[derive(Clone)]
// pub struct Property<T> {
//     untyped_property: UntypedProperty,
//     _cached_data : Rc<RefCell<CachedData<T>>>,
// }

// impl <T:PropertyValue> Property<T> {
//     pub fn new(val: T) -> Self {
//         Self {
//             untyped_property: UntypedProperty::new(val.clone(), Vec::with_capacity(0), PropertyType::Literal),
//             _cached_data: Rc::new(RefCell::new(CachedData {
//                 _cached_version: 0,
//                 _cached_value: val,
//             })),
//         }
//     }

//     pub fn expression(evaluator: impl Fn() -> T + 'static, dependents: &[UntypedProperty]) -> Self {
//         let inbound: Vec<_> = dependents.iter().map(|v| v.get_id()).collect();
//         let start_val = evaluator();
//         let evaluator = Rc::new(generate_untyped_closure(evaluator));
//         Self {
//             untyped_property: UntypedProperty::new(
//                 start_val.clone(),
//                 inbound,
//                 PropertyType::Expression { evaluator },
//             ),
//             _cached_data: Rc::new(RefCell::new(CachedData {
//                 _cached_version: 0,
//                 _cached_value: start_val,
//             })),
//         }
//     }

//     /// Gets the currently stored value. Only clones from table if the version
//     /// has changed since last retrieved
//     #[inline(never)]
//     pub fn get(&self) -> Ref<T> {
//         let current_version = PROPERTY_TABLE.with(|t| t.get_version(self.untyped_property.id));
//         {
//             let cached_version = {
//                 let cached_data = self._cached_data.borrow();
//                 cached_data._cached_version
//             };
//             if cached_version != current_version {
//                 let mut cached_data = self._cached_data.borrow_mut();
//                 let new_val = PROPERTY_TABLE.with(|t| t.get(self.untyped_property.id));
//                 cached_data._cached_value = new_val;
//                 cached_data._cached_version = current_version;
//             }
//         }
//         Ref::map(self._cached_data.borrow(), |cached_data| &cached_data._cached_value)
//     }

//     #[inline(never)]
//     pub fn set(&self, val: T) {
//         PROPERTY_TABLE.with(|t| t.set(self.untyped_property.id, val));
//     }

// }