#!/usr/bin/env python3
"""Actual Router/Link updates and bundled default fonts in one native process."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time
from native_process import NativeProcess, add_launch_arguments


def main():
    parser=argparse.ArgumentParser();parser.add_argument('app',type=Path);add_launch_arguments(parser)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--cases',type=Path,default=Path(__file__).parents[1]/'navigation-cases.json')
    args=parser.parse_args();out=args.output.resolve();out.mkdir(parents=True,exist_ok=False)
    cases=json.loads(args.cases.read_text());report=out/'state.json';capture=out/'capture.png';route_log=out/'routes.jsonl'
    env=dict(os.environ,GSK_RENDERER='ngl',PAX_NATIVE_TEST_REPORT=str(report),PAX_NATIVE_TEST_SCREENSHOT=str(capture),PAX_NAVIGATION_LOG=str(route_log))
    log=(out/'app.log').open('w');app=NativeProcess(args,env,log,out)
    checks=[];completed=False;failure=None
    def check(ok,label):
        checks.append({'check':label,'passed':bool(ok)});print('PASS' if ok else 'FAIL',label,flush=True)
        if not ok:raise RuntimeError(label)
    def state():
        if app.poll() is not None:raise RuntimeError(f'application exited {app.returncode}')
        try:return json.loads(report.read_text())
        except (FileNotFoundError,json.JSONDecodeError):return {}
    def routes():
        try:return [json.loads(line) for line in route_log.read_text().splitlines() if line]
        except (FileNotFoundError,json.JSONDecodeError):return []
    def wait(predicate,label,seconds=20):
        end=time.monotonic()+seconds
        while time.monotonic()<end:
            value=state()
            if predicate(value):return value
            time.sleep(.1)
        raise RuntimeError('Timeout: '+label)
    def xdo(*values):return subprocess.check_output(['xdotool',*map(str,values)],text=True,timeout=10).strip()
    def snapshot(label):
        value=state();(out/(label+'.json')).write_text(json.dumps(value,indent=2)+'\n')
        Path(str(capture)+'.request').touch()
        wait(lambda _:capture.exists() and not Path(str(capture)+'.request').exists(),'screenshot')
        capture.rename(out/(label+'.png'))
        return value
    try:
        wait(lambda s:any(n.get('text')=='Absolute' for n in s.get('nodes',[])) and routes(),'startup',90)
        window=app.x11_window('Pax native navigation');xdo('windowsize','--sync',window,1024,720);xdo('windowfocus','--sync',window);time.sleep(1)
        mapped=Path(f'/proc/{app.pid}/maps').read_text();mounted=Path(f'/proc/{app.pid}/root')
        names=sorted({line.split()[-1] for line in mapped.splitlines() if 'libpax_host.so' in line or 'libnative_navigation.so' in line})
        (out/'identity.json').write_text(json.dumps({'pid':app.pid,'window':window,'executableSHA256':hashlib.sha256(Path(f'/proc/{app.pid}/exe').read_bytes()).hexdigest(),'libraries':{n:hashlib.sha256((mounted/n.lstrip('/')).read_bytes()).hexdigest() for n in names}},indent=2)+'\n')
        initial=snapshot('initial');check(routes()[-1]==cases['initial'],'initial native virtual route is empty')
        samples=[n for n in initial['nodes'] if n.get('text','').startswith('Default ')]
        check(len(samples)==3,'three default-font samples are rendered')
        for node in samples:
            check(node['resolvedFont']=='Roboto','actual Pango glyph font resolves to bundled Roboto')
            check(node['textLayout']['unknownGlyphs']==0 and not node['textLayout']['ellipsized'],'default glyphs are present and not ellipsized')
        bold=next(n for n in samples if 'bold weight' in n['text']);italic=next(n for n in samples if 'italic face' in n['text'])
        check(bold['resolvedFontWeight']==700,'actual resolved default font uses bold weight700')
        check(italic['resolvedFontSlant']==2,'actual resolved default font uses italic face')
        for node in samples:
            expected='Roboto-Italic.ttf' if node is italic else 'Roboto.ttf'
            check(Path(node['resolvedFontFile']).name==expected,'actual glyph file matches packaged '+expected)
            actual=mounted/node['resolvedFontFile'].lstrip('/')
            packaged=next((args.app.parent/'assets').rglob(expected))
            check(hashlib.sha256(actual.read_bytes()).hexdigest()==hashlib.sha256(packaged.read_bytes()).hexdigest(),'loaded font file matches packaged face bytes')
        plain=next(n for n in samples if 'Default Roboto:' in n['text'])
        x,y,w,h=plain['bounds'];route_count=len(routes());plain_before=state()
        xdo('mousemove','--window',window,round(x+8),round(y+h/2));xdo('mousedown',1)
        for dx in range(20,180,20):
            xdo('mousemove','--window',window,round(x+dx),round(y+h/2));time.sleep(.03)
        xdo('mouseup',1)
        wait(lambda s:s['scenePointerReleases']>plain_before['scenePointerReleases'] and any(n['id']==plain['id'] and n.get('selectionEnd',0)>n.get('selectionStart',0) for n in s['nodes']),'native text selection and release')
        check(len(routes())==route_count,'native Text selection remains intact without navigation side effects')
        snapshot('native-selection')
        prime=next(n for n in state()['nodes'] if n.get('text')=='Absolute')
        prime_before=state()
        px,py,pw,ph=prime['bounds'];xdo('mousemove','--window',window,round(px+pw/2),round(py+ph/2));xdo('click',1)
        wait(lambda s:s['scenePointerReleases']>prime_before['scenePointerReleases'] and routes()[-1]==cases['fixture_steps'][0]['expected'],'prime non-home route')
        link=next(n for n in state()['nodes'] if n.get('text')=='Home link')
        x,y,w,h=link['bounds'];before_count=len(routes());drag_before=state()
        xdo('mousemove','--window',window,round(x+8),round(y+h/2));xdo('mousedown',1)
        for dx in range(20,110,15):
            xdo('mousemove','--window',window,round(x+dx),round(y+h/2));time.sleep(.03)
        xdo('mouseup',1)
        wait(lambda s:s['scenePointerReleases']>drag_before['scenePointerReleases'],'Link drag release observed')
        check(len(routes())==before_count,'dragging selection across Link text does not activate navigation')
        check(any(n['id']==link['id'] and n.get('selectionEnd',0)>n.get('selectionStart',0) for n in state()['nodes']),
              'Link text retains genuine native selection after drag')
        before=state();before_count=len(routes())
        screen_width,screen_height=map(int,xdo('getdisplaygeometry').split())
        xdo('mousemove',screen_width-2,screen_height-2);xdo('mousedown',1)
        xdo('mousemove','--window',window,round(x+w/2),round(y+h/2));xdo('mouseup',1)
        wait(lambda s:s['scenePointerReleases']>before['scenePointerReleases'],'unmatched release observed')
        check(state()['scenePointerPresses']==before['scenePointerPresses'],'outside press does not enter the scene')
        check(state()['scenePointerReleases']>before['scenePointerReleases'],'unmatched release actually reaches the scene observer')
        check(len(routes())==before_count,'release without a scene press does not activate Link')
        (out/'unmatched-release.json').write_text(json.dumps({'before':before,'after':state()},indent=2)+'\n')
        reset_before=state()
        xdo('mousemove','--window',window,round(x+w/2),round(y+h/2));xdo('click',1)
        wait(lambda s:s['scenePointerReleases']>reset_before['scenePointerReleases'] and routes()[-1]==cases['initial'],'ordinary Link click resets primed route')
        check(True,'ordinary Link click still activates after cancelled and unmatched pointer sequences')
        for index,step in enumerate(cases['fixture_steps']):
            value=state();node=next(n for n in value['nodes'] if n.get('text')==step['label'])
            before_count=len(routes());before_presses=value['scenePointerPresses']
            x,y,w,h=node['bounds'];xdo('mousemove','--window',window,round(x+w/2),round(y+h/2));xdo('click',1)
            wait(lambda s:s['scenePointerReleases']>value['scenePointerReleases'] and routes() and routes()[-1]==step['expected'],'route '+step['label'])
            check(routes()[-1]==step['expected'],step['label']+' updates actual Router location')
            (out/f'pointer-{index}.json').write_text(json.dumps({'before':value,'after':state()},indent=2)+'\n')
            check(len(routes())==before_count+1,step['label']+' produces exactly one route update')
            check(state()['scenePointerPresses']==before_presses+1,step['label']+' pointer press is observed exactly once')
            check(app.poll() is None and app.pid==json.loads((out/'identity.json').read_text())['pid'],'navigation preserves same native application process')
            (out/f'route-{index}.json').write_text(json.dumps(state(),indent=2)+'\n')
        snapshot('final');app.terminate();check(app.wait(timeout=30)==0,'fixture closes normally')
        log.flush();text=(out/'app.log').read_text();check('PAX_LINUX_CLOSED' in text and 'gl_refs=0 gl_bytes=0' in text,'owned GL publications retired')
        completed=True
    except Exception as error:failure=str(error);raise
    finally:
        try:
            subprocess.run(['xdotool','mouseup','1'],check=False,timeout=10)
            if app.poll() is None:app.terminate();app.wait(timeout=30)
        finally:
            log.close();(out/'checks.json').write_text(json.dumps({'completed':completed,'passed':completed and all(c['passed'] for c in checks),'error':failure,'pid':app.pid,'exitCode':app.poll(),'checks':checks},indent=2)+'\n')

if __name__=='__main__':main()
