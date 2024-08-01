use super::ImplToFromPaxAny;
use super::Numeric;
use super::PaxAny;
use super::PaxValue;
use super::ToFromPaxAny;
use super::ToFromPaxValue;
use crate::impl_from_to_pax_any_for_from_to_pax_value;
use crate::impl_to_from_pax_value;
use crate::math::Space;
use crate::math::Transform2;
use crate::Color;
use crate::ColorChannel;
use crate::Fill;
use crate::Percent;
use crate::Rotation;
use crate::Size;
use crate::Stroke;
use crate::Transform2D;

// Primitive types
impl_to_from_pax_value!(bool, PaxValue::Bool);

impl_to_from_pax_value!(u8, PaxValue::Numeric, Numeric::U8);
impl_to_from_pax_value!(u16, PaxValue::Numeric, Numeric::U16);
impl_to_from_pax_value!(u32, PaxValue::Numeric, Numeric::U32);
impl_to_from_pax_value!(u64, PaxValue::Numeric, Numeric::U64);

impl_to_from_pax_value!(i8, PaxValue::Numeric, Numeric::I8);
impl_to_from_pax_value!(i16, PaxValue::Numeric, Numeric::I16);
impl_to_from_pax_value!(i32, PaxValue::Numeric, Numeric::I32);
impl_to_from_pax_value!(i64, PaxValue::Numeric, Numeric::I64);

impl_to_from_pax_value!(f32, PaxValue::Numeric, Numeric::F32);
impl_to_from_pax_value!(f64, PaxValue::Numeric, Numeric::F64);

impl_to_from_pax_value!(isize, PaxValue::Numeric, Numeric::ISize);
impl_to_from_pax_value!(usize, PaxValue::Numeric, Numeric::USize);

// don't allow to be serialized/deserialized sucessfully, just store it as a dyn Any
impl ImplToFromPaxAny for () {}

// for now. TBD how to handle this when we join Transform2D with Transform2 at some point
impl<F: Space, T: Space> ImplToFromPaxAny for Transform2<F, T> {}

impl_to_from_pax_value!(String, PaxValue::String);
impl_to_from_pax_value!(ColorChannel, PaxValue::ColorChannel);

// Pax internal types
impl_to_from_pax_value!(Numeric, PaxValue::Numeric);
impl_to_from_pax_value!(Size, PaxValue::Size);
impl_to_from_pax_value!(Color, PaxValue::Color);
impl_to_from_pax_value!(Transform2D, PaxValue::Transform2D);
impl_to_from_pax_value!(Rotation, PaxValue::Rotation);
impl_to_from_pax_value!(Percent, PaxValue::Percent);
impl_to_from_pax_value!(Fill, PaxValue::Fill);
impl_to_from_pax_value!(Stroke, PaxValue::Stroke);

impl From<PaxAny> for bool {
    fn from(pax_any: PaxAny) -> Self {
        match pax_any {
            PaxAny::Builtin(b) => bool::from_pax_value(b).unwrap(),
            PaxAny::Any(_) => panic!("can't convert Any to bool"),
        }
    }
}

// Pax Vec type
// impl<T: ToFromPaxAny + 'static> ToFromPaxValue for Vec<T> {
//     fn to_pax_value(self) -> PaxValue {
//         PaxValue::Vec(self.into_iter().map(|v| v.to_pax_any()).collect::<Vec<_>>())
//     }

//     fn from_pax_value(pax_value: PaxValue) -> Result<Self, String> {
//         match pax_value {
//             PaxValue::Vec(vec) => vec
//                 .into_iter()
//                 .map(|v| T::from_pax_any(v))
//                 .collect::<Result<Vec<_>, _>>(),
//             v => return Err(format!("can't coerce {:?} into Vec<PaxAny>", v)),
//         }
//     }

//     fn ref_from_pax_value(_pax_value: &PaxValue) -> Result<&Self, String> {
//         panic!("can't get a reference to a a container type");
//     }

//     fn mut_from_pax_value(_pax_value: &mut PaxValue) -> Result<&mut Self, String> {
//         panic!("can't get a reference to a a container type");
//     }
// }

// impl<T: ToFromPaxAny> ToFromPaxAny for Vec<T> {
//     fn to_pax_any(self) -> PaxAny {
//         PaxAny::Builtin(self.to_pax_value())
//     }

//     fn from_pax_any(pax_any: PaxAny) -> Result<Self, String> {
//         match pax_any {
//             PaxAny::Builtin(b) => <Vec<T>>::from_pax_value(b),
//             PaxAny::Any(_) => panic!("can't create a vec from an any type"),
//         }
//     }

//     fn ref_from_pax_any(_pax_any: &PaxAny) -> Result<&Self, String> {
//         panic!("can't get a reference to a a container type");
//     }

//     fn mut_from_pax_any(_pax_any: &mut PaxAny) -> Result<&mut Self, String> {
//         panic!("can't get a reference to a a container type");
//     }
// }
