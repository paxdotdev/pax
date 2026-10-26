// Same files over HTTP/2 for measurements and HTTP/1 for a convenient preview.
const fs=require('fs'),path=require('path'),zlib=require('zlib'),http=require('http'),http2=require('http2');
const root=path.resolve(process.argv[2]),cache=new Map();
const types={'.wasm':'application/wasm','.js':'text/javascript','.css':'text/css','.html':'text/html','.json':'application/json','.jpg':'image/jpeg','.png':'image/png','.ttf':'font/ttf','.svg':'image/svg+xml'};
function read(file){
 const stat=fs.statSync(file),key=file+':'+stat.mtimeMs;
 if(cache.has(key))return cache.get(key);
 const raw=fs.readFileSync(file),ext=path.extname(file),encoded=['.wasm','.js','.css','.html','.json'].includes(ext);
 const data={body:encoded?zlib.gzipSync(raw,{level:9}):raw,encoded,type:types[ext]||'application/octet-stream'};
 cache.set(key,data);return data;
}
for(const mode of ['baseline','split','pruned','mount-only'])if(fs.existsSync(path.join(root,mode)))for(const file of fs.readdirSync(path.join(root,mode)))if(fs.statSync(path.join(root,mode,file)).isFile())read(path.join(root,mode,file));
function handler(mode){return(req,res)=>{
 try{
  let name=decodeURIComponent(new URL(req.url,'https://localhost').pathname);
  if(name==='/favicon.ico'){res.writeHead(204);res.end();return}
  if(name==='/'||name==='/paxflix/'||name==='/calculator/')name='/index.html';
  let file=name.startsWith('/assets/')?path.join(root,name):path.join(root,mode,name);
  if(!file.startsWith(root+'/')){res.writeHead(403);res.end();return}
  if(!fs.existsSync(file)){res.writeHead(404);res.end();return}
  const data=read(file),headers={'Content-Type':data.type,'Content-Length':data.body.length,'Cache-Control':name.startsWith('/assets/')?'public, max-age=3600':'no-store','Timing-Allow-Origin':'*'};
  if(data.encoded)headers['Content-Encoding']='gzip';
  res.writeHead(200,headers);res.end(req.method==='HEAD'?undefined:data.body);
 }catch(e){res.writeHead(500);res.end(String(e))}
}}
for(const [mode,httpPort,httpsPort] of [['baseline',18754,18756],['split',18755,18757],['pruned',18758,18759],['mount-only',18760,18761]]){
 http.createServer(handler(mode)).listen(httpPort,'127.0.0.1');
 http2.createSecureServer({key:fs.readFileSync(path.join(root,'localhost-key.pem')),cert:fs.readFileSync(path.join(root,'localhost-cert.pem')),allowHTTP1:true},handler(mode)).listen(httpsPort,'127.0.0.1');
}
console.log('Website baseline :18754 / split :18755; HTTP/2 benchmark :18756 / :18757');
