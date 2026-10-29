//! Run with `cargo run -p pax-manifest --features compiler --release --example bench_text_bodies`.
//! Measures lowering, source serialization, and rich/baked encoding separately.
#[cfg(feature = "compiler")]
fn main() {
    use pax_language::{parse_pax_str, Rule};
    use pax_manifest::{parsing::*, *};
    use std::{
        collections::{BTreeMap, HashMap},
        time::Instant,
    };
    fn measure(name: &str, size: usize, mut run: impl FnMut()) {
        let mut times = Vec::new();
        for sample in 0..6 {
            let start = Instant::now();
            run();
            if sample != 0 {
                times.push(start.elapsed().as_micros());
            }
        }
        times.sort();
        println!("{name},{size},{}", times[2]);
    }
    let max: usize = std::env::var("PAX_TEXT_BENCH_MAX")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4 * 1024 * 1024);
    println!("operation,source_bytes,median_us");
    for power in 10..=22 {
        let size = 1 << power;
        if size > max {
            break;
        }
        let source = format!(
            "<Text>{}</Text>",
            "  Unicode 🚀 &amp; references.\n".repeat(size / 32 + 1)
        );
        let pair = parse_pax_str(Rule::pax_component_definition, &source).unwrap();
        let text = TypeId::build_singleton("test::Text", Some("Text"));
        let root = TypeId::build_singleton("test::Root", Some("Root"));
        let lower = || {
            let mut context = TemplateNodeParseContext {
                template: ComponentTemplate::new(root.clone(), None),
                pascal_identifier_to_type_id_map: HashMap::from([("Text".into(), text.clone())]),
                route_branch_descriptors: HashMap::new(),
                content_properties: HashMap::from([(text.clone(), "text".into())]),
            };
            parse_template_from_component_definition_string(&mut context, &source, pair.clone());
            context.template
        };
        measure("lower", source.len(), || {
            std::hint::black_box(lower());
        });
        let component = ComponentDefinition {
            type_id: root.clone(),
            is_main_component: true,
            is_primitive: false,
            is_struct_only_component: false,
            module_path: "test".into(),
            primitive_instance_import_path: None,
            template: Some(lower()),
            settings: None,
            timelines: vec![],
            route_branch: None,
        };
        measure("source", source.len(), || {
            std::hint::black_box(
                code_serialization::press_code_serialization_template(component.clone()).unwrap(),
            );
        });
        let manifest = PaxManifest {
            components: BTreeMap::from([(root.clone(), component)]),
            main_component_type_id: root,
            type_table: HashMap::new(),
            assets_dirs: vec![],
            engine_import_path: "pax_engine".into(),
        };
        measure("rich", source.len(), || {
            std::hint::black_box(binary::to_vec(&manifest).unwrap());
        });
        measure("baked", source.len(), || {
            std::hint::black_box(
                program_ir::binary::to_vec(&program_ir::ProgramIR::from_manifest(&manifest))
                    .unwrap(),
            );
        });
    }
}

#[cfg(not(feature = "compiler"))]
fn main() {
    eprintln!("Enable --features compiler for this benchmark");
}
