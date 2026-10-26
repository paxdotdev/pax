"""Keep transfer totals and timing milestones separate; retain every sample."""
import argparse, hashlib, json, statistics, subprocess
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('build',type=Path);p.add_argument('run',type=Path);a=p.parse_args()
rows=json.loads((a.run/'bench.json').read_text())
sizes={}
for mode in ['baseline','pruned','mount-only','split']:
    if not (a.build/mode).exists(): continue
    files=[f for f in sorted((a.build/mode).glob('*')) if f.is_file()]
    compressed=json.loads(subprocess.check_output(['node','-e','const fs=require("fs"),z=require("zlib");console.log(JSON.stringify(process.argv.slice(1).map(p=>z.gzipSync(fs.readFileSync(p),{level:9}).length)))',*[str(f) for f in files]]))
    sizes[mode]={f.name:dict(raw=f.stat().st_size,gzip=n,sha256=hashlib.sha256(f.read_bytes()).hexdigest()) for f,n in zip(files,compressed)}
groups={}
for row in rows:
    r=row['resources'];t=row['timings']
    t['lastRequestedResource']=max(x['end'] for x in r)
    t['initialViewLoadComplete']=max(t['lastRequestedResource'],t['startupSettled'],t.get('packageReady',0),t.get('firstCanvasSubmission',0),t.get('activatedSceneFrame',0))
    if 'childFetchStart' in t:
        t['childFetchMs']=t['childFetchEnd']-t['childFetchStart']
        t['wasmFetchOverlapMs']=max(0,min(t['baseFetchEnd'],t['childFetchEnd'])-max(t['baseFetchStart'],t['childFetchStart']))
    t['baseFetchMs']=t['baseFetchEnd']-t['baseFetchStart']
    key=row['profile']+'/'+row['variant'];groups.setdefault(key,[]).append(row)
summary={}
for key,group in groups.items():
    metrics={}
    for metric in sorted(set().union(*(x['timings'].keys() for x in group))):
        vals=[x['timings'][metric] for x in group if metric in x['timings']]
        metrics[metric]={'median':statistics.median(vals),'min':min(vals),'max':max(vals),'n':len(vals)}
    first=group[0]
    summary[key]={'timings':metrics,'requestedEncodedBytes':[sum(r.get('wireBody',r['encoded']) for r in x['resources'])+x.get('navigation',{}).get('encodedBodySize',0) for x in group],
                  'resources':[sorted((r['url'].split('/',3)[-1],r['encoded']) for r in x['resources']) for x in group],
                  'baseWasmFetchCounts':[sum('pax-cartridge_bg.wasm' in r['url'] for r in x['resources']) for x in group],
                  'protocols':sorted(set(r['protocol'] for x in group for r in x['resources'])),
                  'hostMemory':[x['hostMemory'] for x in group],'childMemory':[x['childMemory'] for x in group]}
(a.run/'summary.json').write_text(json.dumps({'sizes':sizes,'groups':summary},indent=2))
for key,g in summary.items():print(key,{m:round(g['timings'][m]['median'],1) for m in ['firstViewportPresentation','firstCanvasSubmission','startupSettled','initialViewLoadComplete'] if m in g['timings']},g['requestedEncodedBytes'])
