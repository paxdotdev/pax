#[macro_export]
macro_rules! impl_from_to_pax_any_for_from_to_pax_value {
    ($Type:ty) => {
        impl ToFromPaxAny for $Type {
            fn to_pax_any(self) -> PaxAny {
                self.to_pax_value().to_pax_any()
            }

            fn from_pax_any(pax_any: PaxAny) -> Result<Self, String> {
                PaxValue::from_pax_any(pax_any)
                    .and_then(|pax_value| Self::from_pax_value(pax_value))
            }

            fn ref_from_pax_any(pax_any: &PaxAny) -> Result<&Self, String> {
                PaxValue::ref_from_pax_any(pax_any)
                    .and_then(|pax_value| Self::ref_from_pax_value(pax_value))
            }

            fn mut_from_pax_any(pax_any: &mut PaxAny) -> Result<&mut Self, String> {
                PaxValue::mut_from_pax_any(pax_any)
                    .and_then(|pax_value| Self::mut_from_pax_value(pax_value))
            }
        }
    };
}

// This macro implements from and to
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
                        "pax value {:?} cannot be coerced into {}",
                        pax_value,
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

        impl_from_to_pax_any_for_from_to_pax_value!($Type);
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
        impl_from_to_pax_any_for_from_to_pax_value!($Type);
    };
}
