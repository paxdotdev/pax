#[macro_export]
macro_rules! impl_to_from_pax_value {
    // For a single variant path
    ($Type:ty, $Variant:path) => {
        impl ToFromPaxValue for $Type {
            fn to_pax_value(self) -> PaxValue {
                $Variant(self)
            }

            fn from_pax_value(pax_value: PaxValue) -> Result<Self, String> {
                if let $Variant(val) = pax_value {
                    Ok(val)
                } else {
                    Err(format!(
                        "pax value cannot be coerced into {}",
                        std::any::type_name::<Self>(),
                    ))
                }
            }

            fn ref_from_pax_value(pax_value: &PaxValue) -> Result<&Self, String> {
                if let $Variant(ref val) = pax_value {
                    Ok(val)
                } else {
                    Err(format!(
                        "pax value cannot be coerced into {}",
                        std::any::type_name::<Self>(),
                    ))
                }
            }

            fn mut_from_pax_value(pax_value: &mut PaxValue) -> Result<&mut Self, String> {
                if let $Variant(ref mut val) = pax_value {
                    Ok(val)
                } else {
                    Err(format!(
                        "pax value cannot be coerced into {}",
                        std::any::type_name::<Self>(),
                    ))
                }
            }
        }
    };
    // For nested variant paths like Numeric::U8
    ($Type:ty, $OuterVariant:path, $InnerVariant:path) => {
        impl ToFromPaxValue for $Type {
            fn to_pax_value(self) -> PaxValue {
                $OuterVariant($InnerVariant(self))
            }

            fn from_pax_value(pax_value: PaxValue) -> Result<Self, String> {
                if let $OuterVariant($InnerVariant(val)) = pax_value {
                    Ok(val)
                } else {
                    Err(format!(
                        "pax value cannot be coerced into {}",
                        std::any::type_name::<Self>(),
                    ))
                }
            }

            fn ref_from_pax_value(pax_value: &PaxValue) -> Result<&Self, String> {
                if let $OuterVariant($InnerVariant(ref val)) = pax_value {
                    Ok(val)
                } else {
                    Err(format!(
                        "pax value cannot be coerced into {}",
                        std::any::type_name::<Self>(),
                    ))
                }
            }

            fn mut_from_pax_value(pax_value: &mut PaxValue) -> Result<&mut Self, String> {
                if let $OuterVariant($InnerVariant(ref mut val)) = pax_value {
                    Ok(val)
                } else {
                    Err(format!(
                        "pax value cannot be coerced into {}",
                        std::any::type_name::<Self>(),
                    ))
                }
            }
        }
    };
}
