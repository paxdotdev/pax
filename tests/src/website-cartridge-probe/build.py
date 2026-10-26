"""Freeze the active website and compare one whole-component split on one engine."""
import argparse, hashlib, json, os, re, shutil, subprocess, time
from pathlib import Path

p = argparse.ArgumentParser()
p.add_argument('--output', type=Path, required=True)
p.add_argument('--website', type=Path, required=True)
p.add_argument('--paxflix-output', type=Path, required=True)
p.add_argument('--interface', type=Path, required=True)
p.add_argument('--phase', choices=['prepare', 'baseline', 'pruned', 'mount-only', 'split', 'web', 'all'], default='all')
a = p.parse_args()
fixture = Path(__file__).resolve().parent
repo = fixture.parents[2]
previous = fixture.parent / 'paxflix-cartridge-probe'
out = a.output.resolve()
out.mkdir(parents=True, exist_ok=True)
env = dict(os.environ, PAX_BUILD_TARGET='web', PAX_BUILD_DESIGNTIME='0',
           CARGO_BUILD_JOBS='2', CARGO_INCREMENTAL='0', CARGO_NET_OFFLINE='true')
env.setdefault('CARGO_TARGET_DIR', str(a.paxflix_output / 'target'))

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def prepare():
    snap = out / 'snapshot'
    if snap.exists():
        return
    snap.mkdir()
    website = a.website.resolve()
    for name in ['src', 'assets']:
        shutil.copytree(website / name, snap / name)
    shutil.copy2(website / 'Cargo.toml', snap / 'Cargo.toml')
    shutil.copytree(snap/'src', out/'original-website/src')
    shutil.copy2(snap/'Cargo.toml',out/'original-website/Cargo.toml')
    record = dict(websiteHead=subprocess.check_output(['git', '-C', str(website), 'rev-parse', 'HEAD'], text=True).strip(),
                  engineHead=subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
                  websiteFiles={str(f.relative_to(snap)): digest(f) for f in snap.rglob('*') if f.is_file()},
                  guestHash=digest(a.paxflix_output / 'paxflix.wasm'))
    # Source drawers retain the exact same text in both variants, including
    # their external example snippets, independently of template substitution.
    for source in (snap / 'src').rglob('*.rs'):
        original = website / source.relative_to(snap)
        def include(match):
            target = (original.parent / match[1]).resolve()
            stored = out / 'included-source' / digest(target)
            stored.parent.mkdir(exist_ok=True)
            shutil.copy2(target, stored)
            return f'include_str!("{stored}")'
        source.write_text(re.sub(r'include_str!\("([^"]+)"\)', include, source.read_text()))
    (out / 'snapshot.json').write_text(json.dumps(record, indent=2))
    (out / 'website-working.diff').write_bytes(subprocess.check_output(['git', '-C', str(website), 'diff', '--', 'pax-website']))
    assets = out / 'assets'
    for path in [repo/'pax-std/assets', repo/'examples/calculator/assets', a.paxflix_output/'snapshot/assets', snap/'assets']:
        if path.exists():
            shutil.copytree(path, assets, dirs_exist_ok=True)

def build(mode):
    crate = out / (mode + '-src')
    shutil.copytree(out / 'snapshot/src', crate / 'src', dirs_exist_ok=True)
    deps = f'''pax-kit={{path="{repo/'pax-kit'}", features=["shaders"]}}
calculator={{path="{repo/'examples/calculator'}",default-features=false}}
wasm-bindgen="0.2.129"
'''
    if mode in ['baseline', 'pruned', 'mount-only']:
        dependency = repo/'examples/paxflix'
        if mode != 'baseline':
            dependency = out/'paxflix-no-entry'
            shutil.copytree(a.paxflix_output/'snapshot/src',dependency/'src',dirs_exist_ok=True)
            shutil.copy2(a.paxflix_output/'snapshot/catalog.json',dependency/'catalog.json')
            cargo = (repo/'examples/paxflix/Cargo.toml').read_text().replace('../../pax-kit',str(repo/'pax-kit'))
            (dependency/'Cargo.toml').write_text('[workspace]\n'+cargo)
            lib = dependency/'src/lib.rs'
            lib.write_text(lib.read_text().replace('pax_kit::runnable! { root: crate::Paxflix }',''))
        deps += f'paxflix={{path="{dependency}",default-features=false}}\n'
    else:
        deps += f'''pax-runtime-api={{path="{repo/'pax-runtime-api'}"}}
kurbo="0.11.3"
serde={{version="1",features=["derive"]}}
serde_json="1"
'''
        for name in ['bridge.rs', 'fields.rs', 'trampolines.rs', 'bundle.rs']:
            shutil.copy2(previous/'host/src'/name, crate/'src'/name)
        for name in ['arena.rs', 'events.rs', 'wire.rs']:
            shutil.copy2(previous/name, crate/'src'/name)
        shutil.copy2(previous/'host/build.rs', crate/'build.rs')
        bundle = crate/'src/bundle.rs'
        text = bundle.read_text().replace('crate::Shell', 'crate::Example')
        text = text.replace('self.descriptors.get(id).copied()', 'self.descriptors.get(id).copied().or_else(|| BASE.with(|b| b.borrow().get(id).copied()))')
        text = text.replace('pub embedded: Property<bool>,', 'pub embedded: Property<bool>,\n    pub disable_keyboard: Property<bool>,')
        begin = text.index('        let embedded = node')
        end = text.index('        args.children =', begin)
        text = text[:begin] + '''        let imports = ["embedded", "disable_keyboard"].map(|name| {
            (name, node.properties_scope.borrow().get(name).unwrap().get_untyped_property().clone())
        });
        args.prototypical_properties = rt::PropertiesInit::Factory(Box::new(move |_, expanded| {
            if expanded.is_some() { return None; }
            let value = create(index);
            let props = properties(&value);
            let b = bundle();
            for (name, property) in &imports {
                let field = b.schema.components[index].fields.iter().position(|f| f.name == *name).unwrap();
                *bridge::get(props.fields[field]).borrow_mut() = fields::Field::Bool(LocalProperty::new_from_untyped(property.clone()));
            }
            Some(Rc::new(RefCell::new(value)))
        }));
''' + text[end:]
        bundle.write_text(text)
        for name in ['paxflix_showcase.rs', 'route_pages.rs']:
            path = crate/'src'/name
            path.write_text(path.read_text().replace('use paxflix::Paxflix;', 'use crate::module_gate::WebsitePaxflix;'))
        for name in ['paxflix_showcase.pax', 'paxflix_page.pax']:
            path = crate/'src'/name
            path.write_text(path.read_text().replace('<Paxflix ', '<WebsitePaxflix '))
        lib = crate/'src/lib.rs'
        lib.write_text(lib.read_text() + '''
use std::cell::{Cell, RefCell};
mod arena; mod bridge; mod bundle; mod events; mod fields; mod wire; mod module_gate;
impl Example { pub fn probe_mount(&mut self, ctx: &NodeContext) { bundle::capture(ctx); } }
''')
        with (crate/'src/lib.pax').open('a') as f:
            f.write('\n@settings { @mount: probe_mount }\nif false { <Group/><Frame/><Text/><Image/><Rectangle/><Ellipse/><Scroller/><Stacker/><ImportSettings/><EventBlocker/><DynamicIslandSpacer/> }\n')
        for name in ['module_gate.rs', 'module_gate.pax']:
            shutil.copy2(fixture/name, crate/'src'/name)
    if mode == 'mount-only':
        for name in ['paxflix_showcase.rs', 'route_pages.rs']:
            path = crate/'src'/name
            path.write_text(path.read_text().replace('use paxflix::Paxflix;', 'use crate::module_gate::WebsitePaxflix;'))
        for name in ['paxflix_showcase.pax', 'paxflix_page.pax']:
            path = crate/'src'/name
            path.write_text(path.read_text().replace('<Paxflix ', '<WebsitePaxflix '))
        with (crate/'src/lib.rs').open('a') as f: f.write('\nmod module_gate;\n')
        (crate/'src/module_gate.rs').write_text((fixture/'module_gate.rs').read_text().replace('use crate::bundle::DeferredPaxflix;', 'use paxflix::Paxflix;'))
        (crate/'src/module_gate.pax').write_text((fixture/'module_gate.pax').read_text().replace('DeferredPaxflix','Paxflix'))
    (crate/'Cargo.toml').write_text(f'''[workspace]
[package]
name="website-{mode}-probe"
version="0.0.0"
edition="2021"
[lib]
crate-type=["cdylib","rlib"]
[dependencies]
{deps}
[profile.release]
opt-level="z"
lto=true
codegen-units=1
panic="abort"
strip="symbols"
''')
    start = time.monotonic()
    with (out/(mode+'-build.log')).open('w') as log:
        subprocess.run(['wasm-pack','build',str(crate),'--target','web','--out-name','pax-cartridge','--out-dir',str(out/mode),'--release','--no-opt'],
                       env=dict(env, PAX_MANIFEST_OUTPUT=str(out/(mode+'-manifest.json'))), stdout=log,stderr=subprocess.STDOUT,check=True)
    print(mode, round(time.monotonic()-start, 2), flush=True)
    finish(mode)

def finish(mode):
    crate = out / (mode + '-src')
    glue = out/mode/'pax-cartridge.js'
    source = glue.read_text()
    metadata = json.loads(subprocess.check_output(['cargo','metadata','--manifest-path',str(crate/'Cargo.toml'),'--no-deps','--format-version','1'],env=env))
    package, = metadata['packages']
    library, = [t for t in package['targets'] if 'cdylib' in t['kind']]
    identity = json.dumps([package['id'],library['name']],separators=(',',':')).encode()
    entry = '__pax_init_web_' + hashlib.sha256(identity).hexdigest()
    assert entry in source, entry
    if 'export async function init_web' not in source:
        glue.write_text(source + f'\nexport async function init_web(options) {{ await window.probeBeforeMount?.(wasm); return {entry}(options); }}\n')
    for name in ['pax-interface-web.js','pax-interface-web.css','pax-fonts.json','pax-web-config.json','route-metadata.json']:
        if (a.interface/name).exists(): shutil.copy2(a.interface/name, out/mode/name)
    for f in (fixture/'web').iterdir(): shutil.copy2(f, out/mode/f.name)
    (out/mode/'mode.json').write_text(json.dumps({'mode': 'baseline' if mode == 'pruned' else mode}))
    if mode == 'split': shutil.copy2(a.paxflix_output/'paxflix.wasm',out/mode/'paxflix.wasm')

prepare()
if a.phase == 'web':
    for mode in ['baseline','pruned','mount-only','split']:
        if (out/mode/'pax-cartridge.js').exists(): finish(mode)
else:
    for mode in ['baseline','split'] if a.phase == 'all' else [a.phase]:
        if mode != 'prepare': build(mode)
