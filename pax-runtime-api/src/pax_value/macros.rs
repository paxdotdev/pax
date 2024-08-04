// This macro requires that the $Type can be created by calling into on the $Variant contents
#[macro_export]
macro_rules! impl_default_coercion_rule {
    ($Type:ty, $Variant:path) => {
        impl CoercionRules for $Type {
            fn try_coerce(pax_value: PaxValue) -> Result<Self, String> {
                if let $Variant(val) = pax_value {
                    Ok(val.into())
                } else {
                    Err(format!(
                        "cound't coerce {:?} into {}",
                        pax_value,
                        std::any::type_name::<$Type>()
                    ))
                }
            }
        }
    };
}

// This macro implements from and to
#[macro_export]
macro_rules! impl_to_from_pax_value {
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
