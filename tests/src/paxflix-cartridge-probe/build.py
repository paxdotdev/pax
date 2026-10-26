"""Pinned Paxflix packaging experiment; generated crates and outputs stay external."""
import argparse,hashlib,json,os,re,shutil,subprocess,time
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--output',type=Path,required=True);p.add_argument('--phase',choices=['baseline','package','split','shell','decoder','guest','all'],default='all');p.add_argument('--interface',type=Path,required=True);a=p.parse_args()
root=Path(__file__).resolve().parent;repo=root.parents[2];out=a.output.resolve();out.mkdir(parents=True,exist_ok=True)
env=dict(os.environ,PAX_BUILD_TARGET='web',PAX_BUILD_DESIGNTIME='0',CARGO_BUILD_JOBS='2',CARGO_INCREMENTAL='0')
env.setdefault('CARGO_TARGET_DIR',str(out/'target'))
def run(cmd,log,extra=None):
 start=time.monotonic()
 with (out/log).open('w') as f:subprocess.run(cmd,env=dict(env,**(extra or {})),stdout=f,stderr=subprocess.STDOUT,check=True)
 print(log,round(time.monotonic()-start,2),flush=True)
def web_build(crate,name,extra=None,features=None):
 command=['wasm-pack','build',str(crate),'--target','web','--out-name','pax-cartridge','--out-dir',str(out/name),'--release','--no-opt']
 if features:command+=['--features',features]
 run(command,name+'-build.log',extra)
 glue=out/name/'pax-cartridge.js';s=glue.read_text();entries=re.findall(r'export function (__pax_init_web_[a-f0-9]+)\(',s);assert len(entries)==1,entries
 glue.write_text(s+'\nexport { '+entries[0]+' as init_web };\n')
 for f in ['pax-interface-web.js','pax-interface-web.css']:shutil.copy2(a.interface/f,out/name/f)
 for f in (root/'web').iterdir():shutil.copy2(f,out/name/f.name)
 (out/name/'mode.json').write_text(json.dumps({'mode':name}))
 if not (out/name/'assets').exists():(out/name/'assets').symlink_to(out/'snapshot/assets',target_is_directory=True)
def snapshot():
 src=repo/'examples/paxflix';snap=out/'snapshot'
 if not snap.exists():
  snap.mkdir();shutil.copytree(src/'src',snap/'src');shutil.copytree(src/'assets',snap/'assets');shutil.copy2(src/'catalog.json',snap/'catalog.json')
  hashes={str(f.relative_to(src)):hashlib.sha256(f.read_bytes()).hexdigest() for f in sorted(src.rglob('*')) if f.is_file() and '.pax' not in f.parts and 'target' not in f.parts}
  (out/'snapshot.json').write_text(json.dumps({'gitHead':subprocess.check_output(['git','rev-parse','HEAD'],cwd=repo,text=True).strip(),'files':hashes},indent=2))
 return snap
if a.phase in ['baseline','all']:
 snap=snapshot();baseline=out/'baseline-src';shutil.copytree(snap,baseline,dirs_exist_ok=True)
 (baseline/'Cargo.toml').write_text(f'''[workspace]\n[package]\nname="paxflix-baseline-probe"\nversion="0.0.0"\nedition="2021"\npublish=false\n[lib]\ncrate-type=["cdylib","rlib"]\n[dependencies]\npax-kit={{path="{repo/'pax-kit'}"}}\nserde_json="1"\nwasm-bindgen="0.2.129"\n[profile.release]\nopt-level="z"\nlto=true\ncodegen-units=1\npanic="abort"\nstrip="symbols"\n''')
 source=(baseline/'src/lib.rs').read_text();source=source.replace('pax_kit::runnable! { root: crate::Paxflix }','')
 source+='\n'+(root/'shell.rs').read_text();(baseline/'src/lib.rs').write_text(source)
 (baseline/'src/shell.pax').write_text((root/'shell.pax').read_text().replace('DeferredPaxflix','Paxflix'))
 run(['cargo','metadata','--manifest-path',str(baseline/'Cargo.toml'),'--format-version','1','--no-deps'],'baseline-metadata.log')
 web_build(baseline,'baseline',{'PAX_MANIFEST_OUTPUT':str(out/'baseline-manifest.json')})
if a.phase in ['package','all']:
 run(['cargo','run','--manifest-path',str(root/'Cargo.toml'),'-p','paxflix-module-packer','--',str(out/'baseline-manifest.json'),str(out/'component.paxm'),str(out/'component.json')],'packer.log')
 subprocess.run(['python3',str(root/'generate.py'),str(out)],check=True)
if a.phase in ['guest','all']:
 run(['cargo','build','--lib','--manifest-path',str(out/'guest-src/Cargo.toml'),'--target','wasm32-unknown-unknown','--release'],'guest-build.log')
 run([os.environ.get('PAX_PROBE_BINDGEN',str(Path.home()/'Library/Caches/.wasm-pack/wasm-bindgen-cargo-install-0.2.129/wasm-bindgen')),str(Path(env['CARGO_TARGET_DIR'])/'wasm32-unknown-unknown/release/paxflix_guest.wasm'),'--target','web','--out-dir',str(out/'guest-bindgen'),'--out-name','paxflix'],'guest-bindgen.log')
 shutil.copy2(out/'guest-bindgen/paxflix_bg.wasm',out/'paxflix.wasm')
if a.phase in ['split','shell','decoder','all']:
 for mode in (['split','shell','decoder'] if a.phase=='all' else [a.phase]):
  crate=root/'host'
  if mode in ['shell','decoder']:
   crate=out/(mode+'-src');shutil.copytree(root/'host',crate,dirs_exist_ok=True)
   (crate/'Cargo.toml').write_text('[workspace]\n'+(root/'host/Cargo.toml').read_text().replace('../../../../',str(repo)+'/')+'\n[profile.release]\nopt-level="z"\nlto=true\ncodegen-units=1\npanic="abort"\nstrip="symbols"\n')
   source='use pax_kit::*;\nmod bundle;\nuse bundle::DeferredPaxflix;\n'+(root/'shell.rs').read_text();(crate/'src/lib.rs').write_text(source)
   shutil.copy2(root/'shell-bundle.rs',crate/'src/bundle.rs')
   if mode=='decoder':
    with (crate/'src/lib.rs').open('a') as f:f.write('\n#[wasm_bindgen::prelude::wasm_bindgen] pub fn probe_decode(bytes:&[u8])->u32{pax_engine::pax_manifest::binary::from_slice(bytes).unwrap().components.len() as u32}\n')
  web_build(crate,mode)
  if mode=='split':shutil.copy2(out/'paxflix.wasm',out/mode/'paxflix.wasm')
