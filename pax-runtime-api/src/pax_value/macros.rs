/// Implements coercion by converting the selected `PaxValue` variant's contents.
/// The optional third argument opts into `CoercionRules::is_identity_roundtrip`;
/// omit it unless the type satisfies that method's exact, side-effect-free contract.
#[macro_export]
macro_rules! impl_default_coercion_rule {
    ($Type:ty, $Variant:path) => {
        $crate::impl_default_coercion_rule!($Type, $Variant, false);
    };
    ($Type:ty, $Variant:path, $identity_roundtrip:expr) => {
        impl CoercionRules for $Type {
            fn is_identity_roundtrip() -> bool {
                $identity_roundtrip
            }
            fn try_coerce(pax_value: PaxValue) -> Result<Self, String> {
                if let $Variant(val) = pax_value {
                    Ok(val.into())
                } else {
                    Err(format!(
                        "couldn't coerce {:?} into {}",
                        pax_value,
                        std::any::type_name::<$Type>()
                    ))
                }
            }
        }
    };
}

#[macro_export]
macro_rules! impl_to_pax_value {
    // For a single variant path
    ($Type:ty, $Variant:path) => {
        impl ToPaxValue for $Type {
            fn to_pax_value(self) -> PaxValue {
                $Variant(self)
            }
        }
    };
    // For nested variant paths like Numeric::U8
    // looks almost exactly the same as above, just with nested variant
    ($Type:ty, $OuterVariant:path, $InnerVariant:path) => {
        impl ToPaxValue for $Type {
            fn to_pax_value(self) -> PaxValue {
                $OuterVariant($InnerVariant(self))
            }
        }
    };
}
