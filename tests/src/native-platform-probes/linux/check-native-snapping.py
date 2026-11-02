#!/usr/bin/env python3
"""Real X11 wheel/input acceptance for the canonical native-scroll-snap fixture."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time
from native_process import NativeProcess, add_launch_arguments


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('app', type=Path)
    add_launch_arguments(parser)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    out = args.output.resolve(); out.mkdir(parents=True, exist_ok=False)
    report = out/'state.json'; capture = out/'capture.png'
    env = dict(os.environ, GSK_RENDERER='ngl', PAX_NATIVE_TEST_REPORT=str(report), PAX_NATIVE_TEST_SCREENSHOT=str(capture))
    log = (out/'app.log').open('w'); app = NativeProcess(args, env, log, out)
    checks = []; completed = False; failure = None

    def check(ok, label):
        checks.append({'check':label,'passed':bool(ok)})
        print('PASS' if ok else 'FAIL', label, flush=True)
        if not ok: raise RuntimeError(label)

    def state():
        if app.poll() is not None: raise RuntimeError(f'application exited {app.returncode}')
        try: return json.loads(report.read_text())
        except (FileNotFoundError,json.JSONDecodeError): return {}

    def wait(predicate, label, seconds=15):
        end=time.monotonic()+seconds
        while time.monotonic()<end:
            value=state()
            if predicate(value): return value
            time.sleep(.05)
        raise RuntimeError('Timeout: '+label)

    def xdo(*values):
        return subprocess.check_output(['xdotool',*map(str,values)],text=True,timeout=10).strip()

    def scrollers(value):
        return sorted([n for n in value.get('nodes',[]) if n['kind']=='Scroller'],key=lambda n:n['bounds'][0])

    def settle(label):
        previous=None; stable=0; end=time.monotonic()+12
        while time.monotonic()<end:
            value=state(); nodes=scrollers(value)
            offsets=tuple(round(n[k],2) for n in nodes for k in ['scrollX','scrollY'])
            if len(nodes)==2 and not any(n['snapPending'] or n['snapAnimating'] or n['snapInputHeld'] for n in nodes) and offsets==previous:
                stable+=1
                if stable>=6:
                    text=next(n['text'] for n in value['nodes'] if n.get('text','').startswith('x='))
                    match=re.match(r'x=([\d.-]+) y=([\d.-]+)',text)
                    if match and abs(float(match[1])-nodes[0]['scrollX'])<.15 and abs(float(match[2])-nodes[1]['scrollY'])<.15:
                        (out/(label+'.json')).write_text(json.dumps(value,indent=2)+'\n')
                        return value
            else: stable=0
            previous=offsets; time.sleep(.1)
        raise RuntimeError('Timeout waiting for native/shared stable agreement: '+label)

    def offsets(value):
        nodes=scrollers(value); return nodes[0]['scrollX'],nodes[1]['scrollY']

    def point(node, fx=.7, fy=.7):
        x,y,w,h=node['bounds']; return round(x+w*fx),round(y+h*fy)

    def click_label(label):
        node=next(n for n in state()['nodes'] if n.get('text')==label)
        xdo('mousemove','--window',window,*point(node,.5,.5)); xdo('click',1)
        time.sleep(.15)

    def wheel(axis,direction):
        node=scrollers(state())[axis]
        xdo('mousemove','--window',window,*point(node))
        xdo('click', (7 if direction>0 else 6) if axis==0 else (5 if direction>0 else 4))

    def snapshot(label):
        Path(str(capture)+'.request').touch()
        wait(lambda _:capture.exists() and not Path(str(capture)+'.request').exists(),'screenshot')
        capture.rename(out/(label+'.png'))

    try:
        wait(lambda s:len(scrollers(s))==2,'startup',90)
        window=app.x11_window('Pax native scroll snapping')
        xdo('windowsize','--sync',window,1024,680); xdo('windowfocus','--sync',window)
        time.sleep(1)
        maps=Path(f'/proc/{app.pid}/maps').read_text(); mounted=Path(f'/proc/{app.pid}/root')
        names=sorted({line.split()[-1] for line in maps.splitlines() if 'libpax_host.so' in line or 'libnative_scroll_snap.so' in line})
        (out/'identity.json').write_text(json.dumps({'pid':app.pid,'window':window,'executableSHA256':hashlib.sha256(Path(f'/proc/{app.pid}/exe').read_bytes()).hexdigest(),'libraries':{n:hashlib.sha256((mounted/n.lstrip('/')).read_bytes()).hexdigest() for n in names}},indent=2)+'\n')
        initial=settle('initial'); ids=[n['id'] for n in scrollers(initial)]
        check(offsets(initial)==(0,0),'initial native offsets and shared values agree')
        check([n['snapPointsX'] for n in scrollers(initial)]==[[0,450,900],[]],'horizontal percentage anchors use viewport pixels')
        check([n['snapPointsY'] for n in scrollers(initial)]==[[],[0,240,480]],'vertical percentage anchors use viewport pixels')
        for axis,anchor in [(0,450),(1,240)]:
            wheel(axis,1); value=settle(f'wheel-{axis}-forward')
            check(abs(offsets(value)[axis]-anchor)<.1,'small native wheel input advances one anchor')
            wheel(axis,1); value=settle(f'wheel-{axis}-second')
            check(abs(offsets(value)[axis]-anchor*2)<.1,'repeated native wheel input reaches next anchor')
            wheel(axis,-1); value=settle(f'wheel-{axis}-reverse')
            check(abs(offsets(value)[axis]-anchor)<.1,'native wheel input reverses to previous anchor')
        click_label('Reset'); settle('pointer-motion-reset'); wheel(1,1)
        node=scrollers(state())[1]; px,py=point(node)
        for i in range(12):
            xdo('mousemove','--window',window,px+i%2,py); time.sleep(.1)
        value=state()
        check(not scrollers(value)[1]['snapPending'] and abs(offsets(value)[1]-240)<.1,
              'unrelated pointer motion does not postpone settled scrolling')
        click_label('Pixel anchors'); value=settle('pixels')
        check(scrollers(value)[0]['snapPointsX']==[0,125,325,625] and scrollers(value)[1]['snapPointsY']==[0,125,325,480],'dynamic pixel lists clamp independently to actual extents')
        for i in range(2):
            click_label('Jump to 173'); value=settle(f'jump-{i}')
            check(all(abs(v-125)<.1 for v in offsets(value)),'repeated authored off-anchor request reaches nearest pixel anchor and agrees with engine')
        click_label('Clear anchors'); settle('clear')
        click_label('Jump to 173'); value=settle('free-jump')
        check(all(abs(v-173)<.1 for v in offsets(value)),'cleared lists preserve exact free authored offsets')
        wheel(1,1); value=settle('free-wheel')
        check(value and offsets(value)[1]>173 and not scrollers(value)[1]['snapPointsY'],'cleared native wheel scrolling remains continuous')
        click_label('Page anchors'); value=settle('pages-restored')
        check(all(any(abs(v-a)<.1 for a in n[key]) for v,n,key in zip(offsets(value),scrollers(value),['snapPointsX','snapPointsY'])),'restoring page anchors resolves current offsets')
        click_label('Resize viewports'); value=settle('resized')
        check(scrollers(value)[0]['snapPointsX']==[0,360,720] and scrollers(value)[1]['snapPointsY']==[0,180,360],'resizing recomputes percentage anchors without remounting')
        check([n['id'] for n in scrollers(value)]==ids,'dynamic policy and viewport patches preserve native identity')
        click_label('Reset'); settle('held-reset')
        node=scrollers(state())[1]; xdo('mousemove','--window',window,*point(node)); xdo('mousedown',1)
        try:
            xdo('click',5); time.sleep(.8); value=state(); position=offsets(value)[1]
            check(scrollers(value)[1]['snapInputHeld'] and position>0 and all(abs(position-a)>.1 for a in [0,240,480]),'held pointer prevents snapping during otherwise quiet native wheel movement')
        finally: xdo('mouseup',1)
        value=settle('held-released')
        check(abs(offsets(value)[1]-240)<.1,'pointer release permits snapping after quiet movement')
        # Confirm the actual button in the newly visible page receives native input.
        node=next(n for n in value['nodes'] if n.get('text')=='Vertical 1')
        xdo('mousemove','--window',window,*point(node,.5,.5)); xdo('click',1)
        wait(lambda s:any('clicks=1 ' in n.get('text','') for n in s['nodes']),'settled page button input')
        check(True,'native hit testing follows settled scroll offset')
        click_label('Reset'); settle('scrollbar-reset')
        vertical=scrollers(state())[1]
        bx,by,bw,bh=vertical['scrollbarBoundsY']
        thumb_x=round(bx+bw/2); thumb_y=round(by+bh/6)
        xdo('mousemove','--window',window,thumb_x,thumb_y); time.sleep(.5)
        xdo('mousedown',1)
        try:
            xdo('mousemove','--sync','--window',window,thumb_x,thumb_y+45)
            time.sleep(.8); value=state(); position=offsets(value)[1]
            check(scrollers(value)[1]['snapInputHeld'] and position>0 and
                  all(abs(position-a)>1 for a in [0,240,480]),
                  'dragged native scrollbar remains off-anchor while its button is held')
            (out/'scrollbar-held.json').write_text(json.dumps(value,indent=2)+'\n')
        finally: xdo('mouseup',1)
        value=settle('scrollbar-released')
        check(any(abs(offsets(value)[1]-a)<.1 for a in [0,240,480]),
              'native scrollbar release settles to an anchor and shared position agrees')
        snapshot('final'); app.terminate(); check(app.wait(timeout=30)==0,'fixture closes normally')
        log.flush(); text=(out/'app.log').read_text()
        check('PAX_LINUX_CLOSED' in text and 'gl_refs=0 gl_bytes=0' in text,'owned GL publications retired')
        completed=True
    except Exception as error:
        failure=str(error); raise
    finally:
        try:
            if app.poll() is None: app.terminate(); app.wait(timeout=30)
        finally:
            log.close(); (out/'checks.json').write_text(json.dumps({'completed':completed,'passed':completed and all(c['passed'] for c in checks),'error':failure,'pid':app.pid,'exitCode':app.poll(),'checks':checks},indent=2)+'\n')

if __name__=='__main__': main()
