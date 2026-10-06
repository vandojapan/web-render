const { test, before, after } = require('node:test');
const assert = require('node:assert/strict');
const http = require('node:http');
const fs = require('node:fs/promises');
const path = require('node:path');
const { chromium } = require(process.env.CODEX_PRIMARY_RUNTIME_NODE_MODULES
  ? path.join(process.env.CODEX_PRIMARY_RUNTIME_NODE_MODULES, 'playwright') : 'playwright');
let server, browser, base;
const html = `<!doctype html><html><head><style>
html,body {margin:0;background:transparent;} #box {width:40px;height:40px;background:#ff0066;
animation:slide 2s linear infinite;} @keyframes slide {from{transform:translateX(0)}to{transform:translateX(200px)}}
</style></head><body><div id="box"></div><p id="text"></p><script>
window.aviutl={render:async(frame,params)=>{ await Promise.resolve();
document.querySelector('#text').textContent=params.title;
document.body.dataset.frame=frame.currentFrame; }};
</script></body></html>`;
before(async () => {
  const runtime = await fs.readFile(path.join(__dirname, '../packages/vi5/src/client/htmlSession.mjs'));
  server = http.createServer((req,res) => {
    res.setHeader('Content-Type','text/html');
    if (req.url === '/runtime.mjs') {res.setHeader('Content-Type','text/javascript');res.end(runtime);}
    else if(req.url === '/scene.html') {res.setHeader('Content-Type','text/html');res.end(html);}
    else if(req.url === '/broken.html') {res.end('<img src="/missing.png">');}
    else if(req.url === '/media.html') {res.end('<video></video>');}
    else if(req.url === '/hanging.html') {res.end('<script>window.aviutl={render:()=>new Promise(()=>{})}</script>');}
    else if(req.url === '/late.html') {res.end('<p id="text"></p><script>window.aviutl={render:async()=>{await new Promise(r=>setTimeout(r,120));document.querySelector("#text").textContent="late"}}</script>');}
    else if(req.url === '/missing.png') {res.statusCode=404;res.end();}
    else {res.setHeader('Content-Type','text/html');res.end('<body style="margin:0;background:transparent"></body>');}
  });
  await new Promise(resolve => server.listen(0,'127.0.0.1',resolve));
  base = `http://127.0.0.1:${server.address().port}`;
  browser = await chromium.launch({headless:true,executablePath:process.env.CHROMIUM_EXECUTABLE_PATH,args:['--no-sandbox']});
});

test('timed-out or disposed HTML cannot update a replacement document', async()=>{
  const p=await page();
  const result=await p.evaluate(async()=>{
    const old=new api.HtmlSession('/late.html',320,180,3000);
    await old.ready;
    old.timeoutMs=40;
    let timedOut=false;
    try{await old.render({currentTime:0,currentFrame:0},{})}catch{timedOut=true}
    const removed=!old.iframe.isConnected;
    const fresh=new api.HtmlSession('/scene.html',320,180,3000);
    await fresh.render({currentTime:0.5,currentFrame:30},{title:'replacement'});
    await new Promise(r=>setTimeout(r,150));
    const text=fresh.iframe.contentDocument.querySelector('#text').textContent;
    fresh.dispose();
    let disposedRejected=false;
    try{await fresh.render({currentTime:0,currentFrame:0},{})}catch{disposedRejected=true}
    return {timedOut,removed,text,disposedRejected};
  });
  assert.deepEqual(result,{timedOut:true,removed:true,text:'replacement',disposedRejected:true});
  await p.close();
});
after(async()=>{await browser?.close(); if(server) await new Promise(resolve=>server.close(resolve));});
async function page() {
  const p=await browser.newPage({viewport:{width:480,height:360}});
  await p.goto(base);
  await p.evaluate(async()=>{window.api=await import('/runtime.mjs');});
  return p;
}
test('CSS frame time, reverse seek, async hook, Unicode params and native-paint transparency',async()=>{
  const p=await page();
  await p.evaluate(async()=>{
    window.session = new api.HtmlSession('/scene.html',320,200);
    await session.render({currentTime:0.5,currentFrame:30},{title:'琴葉姉妹\n"タイトル"'});
  });
  const state=await p.evaluate(()=>{
    const doc=session.iframe.contentDocument;
    return {transform:doc.querySelector('#box').getComputedStyle,
      css:session.iframe.contentWindow.getComputedStyle(doc.querySelector('#box')).transform,
      text:doc.querySelector('#text').textContent,frame:doc.body.dataset.frame,
      time:doc.getAnimations()[0].currentTime};
  });
  assert.equal(state.css,'matrix(1, 0, 0, 1, 50, 0)');
  assert.equal(state.text,'琴葉姉妹\n"タイトル"'); assert.equal(state.time,500);assert.equal(state.frame,'30');
  const first=await p.screenshot({omitBackground:true});
  await p.evaluate(async()=>{await session.render({currentTime:1.5,currentFrame:90},{title:'別'});
    await session.render({currentTime:0.5,currentFrame:30},{title:'琴葉姉妹\n"タイトル"'});});
  const repeated=await p.screenshot({omitBackground:true}); assert.deepEqual(repeated,first);
  await fs.writeFile(path.join(__dirname,'html-render-proof.png'),first);
  await p.close();
});
test('viewport bounds, same-origin restriction and recursion rejection',async()=>{
  const p=await page();
  const result=await p.evaluate(()=>{
    const errors=[];
    for(const fn of [()=>api.validateSize(4097,2160),()=>api.validateSize(1920,2241),
      ()=>api.validateSize(NaN,10),()=>api.resolvePageURL('https://example.com/'),
      ()=>api.resolvePageURL('/vi5'),()=>api.resolvePageURL('javascript:alert(1)')]) {
      try{fn();errors.push(false)}catch{errors.push(true)}
    }
    api.validateSize(3840,2160); return errors;
  }); assert.deepEqual(result,[true,true,true,true,true,true]);await p.close();
});
test('separate object documents, resize, cleanup, asset failures and media rejection',async()=>{
  const p=await page();
  const result=await p.evaluate(async()=>{
    const one=new api.HtmlSession('/scene.html',320,200);
    const two=new api.HtmlSession('/scene.html',640,300);
    await one.render({currentTime:0,currentFrame:0},{title:'one'});one.hide();
    await two.render({currentTime:1,currentFrame:60},{title:'two'});
    const separate=one.iframe.contentDocument.querySelector('#text').textContent==='one' &&
      two.iframe.contentDocument.querySelector('#text').textContent==='two';
    const size=two.iframe.contentWindow.innerWidth;
    one.dispose();two.dispose();
    const broken=new api.HtmlSession('/broken.html',100,100,500);
    let brokenRejected=false;try{await broken.ready}catch{brokenRejected=true}
    const media=new api.HtmlSession('/media.html',100,100);
    let mediaRejected=false;try{await media.render({currentTime:0},{})}catch{mediaRejected=true}finally{media.dispose()}
    const hanging=new api.HtmlSession('/hanging.html',100,100,100);
    let timeout=false;try{await hanging.render({currentTime:0},{})}catch(error){timeout=String(error).includes('timeout')}finally{hanging.dispose()}
    return {separate,size,brokenRejected,mediaRejected,timeout,left:document.querySelectorAll('iframe').length};
  });assert.deepEqual(result,{separate:true,size:640,brokenRejected:true,mediaRejected:true,timeout:true,left:0});await p.close();
});
