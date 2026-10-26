"""Generate bindings from the pinned manifest; application handlers remain unchanged."""
import json,re,shutil,sys
from pathlib import Path
root=Path(__file__).resolve().parent;out=Path(sys.argv[1]);guest=out/'guest-src'
m=json.loads((out/'component.json').read_text());types={json.loads(k)['_type_id']:v for k,v in m['type_table'].items()}
components={json.loads(k)['_type_id']:v for k,v in m['components'].items()}
def default(t):
 if t=='bool':return {'Bool':False}
 if t in ['usize','u64','f64']:return {'Numeric':{dict(usize='USize',u64='U64',f64='F64')[t]:0}}
 if t=='std::string::String':return {'String':''}
 if t.startswith('std::vec::Vec<'):return {'Vec':[]}
 return {'Object':[[f['name'],default(f['type_id']['_type_id'])] for f in types[t]['property_definitions']]}
schema={'abi':3,'components':[]};construct=[];handlers=[];hi=0
for i,(tid,c) in enumerate(sorted(components.items())):
 if c['is_struct_only_component']:continue
 # Contiguous indices are shared by the descriptor and dispatcher, not Rust layouts.
 i=len(construct);name=tid.split('::')[-1];module=tid.split('::')[1:-1];file=out/'snapshot/src'/('/'.join(module)+'.rs' if module else 'lib.rs');source=file.read_text()
 fields=[{'name':f['name'],'kind':{'bool':1,'usize':2,'u64':3,'f64':4,'std::string::String':5}.get(f['type_id']['_type_id'],0),'default':default(f['type_id']['_type_id'])} for f in types[tid]['property_definitions']]
 found=re.search(r'impl '+name+r'\s*\{(.*?)(?:\n\}\n|\n\}$)',source,re.S)
 hs=[]
 for h,args in re.findall(r'pub fn (\w+)\(([^)]*)\)',found.group(1) if found else ''):
  if '&NodeContext' not in args:continue
  event=re.search(r'Event<(\w+)>',args);event=event.group(1) if event else ''
  hs.append({'name':h,'event':event,'index':hi})
  call=f'let value=value.downcast_mut::<{tid}>().unwrap();'
  if event:call+=f'let event=Event::new(<{event} as pax_kit::events::WireEvent>::decode(request.event));value.{h}(&ctx,event.clone());event.cancelled() as u32'
  else:call+=f'value.{h}(&ctx);0'
  handlers.append(f'{hi}=>{{{call}}}');hi+=1
 schema['components'].append({'id':tid,'fields':fields,'handlers':hs})
 construct.append(f'{i}=>Box::new({tid}{{'+','.join(f'{f["name"]}:Property::field(b,{j})' for j,f in enumerate(fields))+'})')
(out/'schema.json').write_text(json.dumps(schema,separators=(',',':')))
shutil.copytree(out/'snapshot',guest,dirs_exist_ok=True)
(guest/'Cargo.toml').write_text(f'''[workspace]\n[package]\nname="paxflix-guest"\nversion="0.0.0"\nedition="2021"\n[lib]\ncrate-type=["cdylib"]\n[dependencies]\npax-kit={{package="pax-module-guest-api",path="{root/'guest-api'}"}}\nserde_json="1"\n[profile.release]\nopt-level="z"\nlto=true\ncodegen-units=1\npanic="abort"\nstrip="symbols"\n''')
s=(guest/'src/lib.rs').read_text().replace('pax_kit::runnable! { root: crate::Paxflix }','');s+='\nmod dispatch;\n';(guest/'src/lib.rs').write_text(s)
shutil.copy2(root/'arena.rs',guest/'src/arena.rs')
(guest/'src/dispatch.rs').write_text('''use crate::*;
use std::{any::Any,cell::RefCell,rc::Rc};
#[path="arena.rs"] mod arena;
thread_local!{static INSTANCES:RefCell<arena::Arena<Rc<RefCell<Box<dyn Any>>>>>=RefCell::new(Default::default());}
#[no_mangle] pub extern "C" fn module_dispatch(op:u32,a:u32,b:u32)->u32{match op{
1=>{let value:Box<dyn Any>=match a{'''+','.join(construct)+''',_=>panic!("component ID")};INSTANCES.with(|x|x.borrow_mut().insert(Rc::new(RefCell::new(value))))},
2=>{let value=INSTANCES.with(|x|x.borrow().try_get(a).cloned());let Some(value)=value else{return u32::MAX};let request=pax_kit::request();let ctx=NodeContext::from_request(&request);let mut value=value.borrow_mut();match b{'''+','.join(handlers)+''',_=>panic!("handler ID")}},
3=>pax_kit::evaluate(a),4=>{pax_kit::drop_callback(a);0},
5=>{let value=INSTANCES.with(|x|x.borrow_mut().remove(a));drop(value);0},
6=>pax_kit::output_word(a),7=>INSTANCES.with(|x|x.borrow().len() as u32),8=>pax_kit::callbacks(),_=>panic!("operation")}}
static DEFINITION:&[u8]=include_bytes!("../../component.paxm");
static SCHEMA:&[u8]=include_bytes!("../../schema.json");
#[no_mangle] pub extern "C" fn module_data(which:u32)->u32{match which{0=>DEFINITION.as_ptr() as u32,1=>DEFINITION.len() as u32,2=>SCHEMA.as_ptr() as u32,3=>SCHEMA.len() as u32,_=>3}}
''')
print('Generated',len(construct),'components and',hi,'handlers')
