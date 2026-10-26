"""Local gzip transport with explicit content lengths for resource timing."""
import argparse,gzip,http.server,mimetypes
from pathlib import Path
from functools import lru_cache
@lru_cache(maxsize=128)
def compressed(path,mtime):return gzip.compress(Path(path).read_bytes(),compresslevel=9,mtime=0)
p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('--port',type=int,default=18752);a=p.parse_args()
class Handler(http.server.SimpleHTTPRequestHandler):
 def __init__(self,*args,**kwargs):super().__init__(*args,directory=str(a.root),**kwargs)
 def send_head(self):
  path=Path(self.translate_path(self.path))
  if path.is_dir():path=path/'index.html'
  if not path.is_file():return super().send_head()
  body=path.read_bytes();encoded=path.suffix in ['.wasm','.js','.css','.html','.json'] and 'gzip' in self.headers.get('Accept-Encoding','')
  if encoded:body=compressed(str(path),path.stat().st_mtime_ns)
  self.send_response(200);self.send_header('Content-Type',mimetypes.guess_type(str(path))[0] or 'application/octet-stream');self.send_header('Content-Length',str(len(body)));self.send_header('Cache-Control','no-store');self.send_header('Timing-Allow-Origin','*')
  if encoded:self.send_header('Content-Encoding','gzip')
  self.end_headers();import io;return io.BytesIO(body)
 def log_message(self,*args):pass
for mode in ['baseline','split','shell','decoder']:
 for path in (a.root/mode).glob('*'):
  if path.suffix in ['.wasm','.js','.css','.html','.json']:compressed(str(path),path.stat().st_mtime_ns)
http.server.ThreadingHTTPServer(('127.0.0.1',a.port),Handler).serve_forever()
