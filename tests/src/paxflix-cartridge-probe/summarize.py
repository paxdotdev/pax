"""Summarize external benchmark evidence; no generated reports enter the repo."""
import gzip,hashlib,json,statistics,sys
from pathlib import Path
out=Path(sys.argv[1]);modes=['baseline','split','shell','decoder'];sizes={}
for mode in modes:
 files={}
 for p in sorted((out/mode).rglob('*')):
  if p.is_file() and 'assets' not in p.parts and p.suffix in ['.wasm','.js','.css','.html','.json'] and p.name!='package.json':
   b=p.read_bytes();files[str(p.relative_to(out/mode))]={'raw':len(b),'gzip':len(gzip.compress(b,compresslevel=9,mtime=0)),'sha256':hashlib.sha256(b).hexdigest()}
 sizes[mode]=files
sizes['assets']={str(p.relative_to(out/'snapshot/assets')):p.stat().st_size for p in sorted((out/'snapshot/assets').rglob('*')) if p.is_file()}
(out/'sizes.json').write_text(json.dumps(sizes,indent=2))
rows=json.loads((out/'bench.json').read_text());decode=json.loads((out/'decoder.json').read_text())
def distribution(values):
 v=sorted(values);return {'n':len(v),'median':statistics.median(v),'min':v[0],'max':v[-1]}
summary={'source':json.loads((out/'snapshot.json').read_text())['gitHead'],'browser':rows[0]['browser'],'bytes':{},'timings':{},'decoderMs':{'first':distribution([x['samples'][0]['ms'] for x in decode]),'warm':distribution([s['ms'] for x in decode for s in x['samples'][1:]])}}
for mode in modes:
 files=sizes[mode];summary['bytes'][mode]={'baseWasmGzip':files['pax-cartridge_bg.wasm']['gzip'],'programGzip':sum(f['gzip'] for f in files.values())}
summary['bytes']['childWasmGzip']=sizes['split']['paxflix.wasm']['gzip']
summary['bytes']['split']['initialProgramGzip']=summary['bytes']['split']['programGzip']-summary['bytes']['childWasmGzip']
images=[r for r in rows[0]['settled']['resources'] if r['name'].endswith(('.jpg','.png'))]
summary['bytes']['firstViewImages']={'count':len(images),'bytes':sum(r['encoded'] for r in images)}
summary['bytes']['allImages']={'count':sum(k.endswith(('.jpg','.png')) for k in sizes['assets']),'bytes':sum(v for k,v in sizes['assets'].items() if k.endswith(('.jpg','.png')))}
for mode in ['baseline','split']:
 for profile in ['local','network']:
  r=[x for x in rows if x['mode']==mode and x['profile']==profile]
  if not r:continue
  group={key:distribution([x[key] for x in r]) for key in ['mountToDomMs','mountToTwoFramesMs','requestToDomMs']}
  group.update({key:distribution([x['timings'][key] for x in r]) for key in r[0]['timings'] if key.endswith('Ms') and key!='activateCallMs'})
  for phase,name in [('firstTickMs','pax:tick'),('firstFrameMs','pax:renderLoopFrame')]:group[phase]=distribution([next(m['duration'] for m in x['measures'] if m['name']==name) for x in r])
  group['hostMemoryBytes']=distribution([x['hostMemory'] for x in r]);group['childMemoryBytes']=distribution([x['childMemory'] for x in r])
  summary['timings'][mode+'-'+profile]=group
(out/'summary.json').write_text(json.dumps(summary,indent=2));print(json.dumps(summary['bytes'],indent=2))
