

pub fn compile_all_expressions<'a>(manifest: &'a mut PaxManifest) {

    let mut new_expression_specs : HashMap<usize, ExpressionSpec> = HashMap::new();
    let mut stack_offset = 0;
    let mut uid_gen = 0..;

    let mut component_id_map = HashMap::new();

    for cd in manifest.components.iter() {
        component_id_map.insert(&cd.source_id, &*cd);
    }

    let mut new_components = manifest.components.clone();
    new_components.iter_mut().for_each(|component_def : &mut ComponentDefinition|{

        let mut new_component_def = component_def.clone();
        let read_only_component_def = component_def.clone();


        if let Some(ref mut template) = new_component_def.template {
            template.iter_mut().for_each(|node_def| {
                let mut new_node_def = node_def.clone();
                let mut ctx = TemplateTraversalContext {
                    active_node_def: new_node_def,
                    scope_stack: vec![component_def.property_definitions.iter().map(|pd| {(pd.name.clone(), pd.clone())}).collect()],
                    uid_gen: 0..,
                    expression_specs: &mut new_expression_specs,
                    component_def: &read_only_component_def,
                    template_node_definitions: manifest.template_node_definitions.clone(),
                };

                ctx = recurse_template_and_compile_expressions(ctx);

                std::mem::swap(node_def, &mut ctx.active_node_def);
                std::mem::swap(&mut manifest.template_node_definitions, &mut ctx.template_node_definitions);
            });
        }

        std::mem::swap(component_def, &mut new_component_def);

    });
    manifest.components = new_components;
    manifest.expression_specs = Some(new_expression_specs);

    println!("{}", serde_json::to_string_pretty(&manifest).unwrap());
}