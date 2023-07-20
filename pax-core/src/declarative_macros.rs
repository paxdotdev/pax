/// Used to manage boilerplate surrounding feature-gated access of TypesCoproduct variant structs.
/// Usage: let rect = safe_unwrap!(val, pax_stdCOCOprimitivesCOCORectangle);
#[macro_export]
macro_rules! safe_unwrap {
    ($value:expr, $discriminant:tt) => {
        {
            #[cfg(feature = "cartridge-inserted")]
            {
                match $value {
                    TypesCoproduct::$discriminant(val) => val,
                    _ => unreachable!(),
                }
            }
            #[cfg(not(feature = "cartridge-inserted"))]
            {
                unreachable!("Cannot run program without cartridge inserted.")
            }
        }
    };
}

/// Used to manage boilerplate around feature-gated property access for primitives.  This approach supersedes `unsafe_unwrap`, solving
/// the same problems unsafe_unwrap solved, without the (realized) hazards of memory leaks and crashes.
/// Usage: 1. assumes the presence of a property on self called `properties_raw`
///        2. alongside a struct definition for EllipseInstance, call `generate_property_access!(EllipseInstance, Ellipse);`
///        3. ensure, for any runtime build, that the feature `cartridge-inserted` is enabled.
#[macro_export]
macro_rules! generate_property_access {
    ($struct_name:ident, $property_type:ident) => {
        impl<R: 'static + RenderContext> $struct_name<R> {
            fn get_properties_mut(&mut self) -> &mut $property_type {
                #[cfg(feature = "cartridge-inserted")]
                {
                    match &mut self.properties_raw {
                        PropertiesCoproduct::$property_type(properties) => properties,
                        _ => unreachable!(),
                    }
                }
                #[cfg(not(feature = "cartridge-inserted"))]
                {
                    unreachable!("Cartridge not inserted.")
                }
            }

            fn get_properties(&self) -> &$property_type {
                #[cfg(feature = "cartridge-inserted")]
                {
                    match &self.properties_raw {
                        PropertiesCoproduct::$property_type(properties) => properties,
                        _ => unreachable!(),
                    }
                }
                #[cfg(not(feature = "cartridge-inserted"))]
                {
                    unreachable!("Cartridge not inserted.")
                }
            }
        }
    };
}


#[macro_export]
macro_rules! compute_properties_transform {
    () => {
        {
            let latest_transform = transform_coeffs;
            let is_new_transform = match &last_patch.transform {
                Some(cached_transform) => {
                    latest_transform.iter().enumerate().any(|(i,elem)|{
                        *elem != cached_transform[i]
                    })
                },
                None => {
                    true
                },
            };
            if is_new_transform {
                new_message.transform = Some(latest_transform.clone());
                last_patch.transform = Some(latest_transform.clone());
                has_any_updates = true;
            }
        }
    }
}

