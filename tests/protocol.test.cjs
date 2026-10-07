const path=require('node:path');
const root=path.resolve(__dirname,'..');
const {chromium}=require(process.env.CODEX_PRIMARY_RUNTIME_NODE_MODULES ? process.env.CODEX_PRIMARY_RUNTIME_NODE_MODULES+'/playwright' : 'playwright');
const pb=require(path.join(root,'packages/vi5/node_modules/@bufbuild/protobuf'));
require('node:test').test('Vite runtime emits the matching frame metadata and page image',async()=>{
 const net=require('node:net');
 const listener=net.createServer();
 await new Promise((resolve,reject)=>{listener.once('error',reject);listener.listen(0,'127.0.0.1',resolve)});
 const port=listener.address().port;
 await new Promise(resolve=>listener.close(resolve));
 const base=`http://localhost:${port}`;
 const child=require('node:child_process').spawn(process.execPath,['../../packages/vi5/dist/cli.mjs','start','--port',String(port)],{cwd:path.join(root,'examples/html'),stdio:'pipe',windowsHide:true});
 let b,output='',spawnError;
 child.stdout.on('data',chunk=>{output+=chunk});child.stderr.on('data',chunk=>{output+=chunk});
 child.on('error',error=>{spawnError=error});
 try {
 const deadline=Date.now()+20000;
 for(;;){
  if(spawnError)throw spawnError;
  if(child.exitCode!==null)throw new Error(`Vite exited (${child.exitCode}): ${output}`);
  try{if((await fetch(`${base}/vi5`,{signal:AbortSignal.timeout(1000)})).ok)break}catch{}
  if(Date.now()>deadline)throw new Error(`Vite readiness timed out: ${output}`);
  await new Promise(r=>setTimeout(r,100));
 }
 const schema=await import(require('node:url').pathToFileURL(path.join(root,'packages/vi5/src/gen/common_pb.ts')).href);
 b=await chromium.launch({executablePath:process.env.CHROMIUM_EXECUTABLE_PATH,headless:true,args:['--no-sandbox']});
 const p=await b.newPage({viewport:{width:4096,height:2304}});const errors=[];p.on('pageerror',e=>errors.push(String(e)));
 await p.goto(`${base}/vi5`);await p.waitForFunction(()=>window.__vi5__?.objects.has('html-title'));
 // Keep initialization metadata until the native reader acknowledges it, even
 // if logs and the initial object list are ready before the first OSR paint.
 await p.waitForFunction(()=>window.__vi5__.ctx.getImageData(0,0,1,1).data[0]===255);
 const initialization=await p.evaluate(async()=>{
  const nonce=()=>{const d=window.__vi5__.ctx.getImageData(0,0,3,1).data;return (d[4]|d[5]<<8|d[6]<<16|d[8]<<24)>>>0};
  const settle=()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(()=>requestAnimationFrame(r))));
  window.__vi5__.pushLog('info','queued-before-initialization-ack');
  await settle();const before=nonce();
  window.__vi5__.acknowledge(999);await settle();const wrong=nonce();
  window.__vi5__.acknowledge(0);await settle();return {before,wrong,after:nonce()};
 });
 require('node:assert/strict').deepEqual(initialization,{before:0,wrong:0,after:1});
 const data=pb.toBinary(schema.BatchRenderRequestSchema,pb.create(schema.BatchRenderRequestSchema,{renderRequests:[{
  renderNonce:17,object:'html-title',objectId:1n,isOffline:true,
  frameInfo:{screenWidth:1920,screenHeight:1080,currentFrame:90,currentTime:1.5,framerate:60,totalFrames:600,totalTime:10},
  parameters:[{key:'title',value:{case:'textValue',value:'HTML全体をAviUtl2へ'}},{key:'accent',value:{case:'colorValue',value:{r:255,g:70,b:130,a:255}}}],
 }]}));
 await p.evaluate(async encoded=>{await window.__vi5__.render(12345,encoded)},Buffer.from(data).toString('base64'));
 const result=await p.evaluate(()=>{const d=window.__vi5__.ctx.getImageData(0,0,6,1).data;return {header:Array.from(d),iframes:document.querySelectorAll('iframe').length,text:document.querySelector('iframe')?.contentDocument.querySelector('#title')?.textContent}});
 console.log(JSON.stringify({errors,result}));await p.screenshot({path:path.join(__dirname,'protocol-proof.png'),omitBackground:true});
 require('node:assert/strict').deepEqual(errors,[]);require('node:assert/strict').equal(result.text,'HTML全体をAviUtl2へ');require('node:assert/strict').deepEqual([result.header[4],result.header[5],result.header[6],result.header[8]],[57,48,0,0]);
 // Notifications must wait for the real native-copy ACK and survive a wrong ACK.
 const ackResult=await p.evaluate(async()=>{
  const nonce=()=>{const d=window.__vi5__.ctx.getImageData(0,0,3,1).data;return (d[4]|d[5]<<8|d[6]<<16|d[8]<<24)>>>0};
  const settle=()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(()=>requestAnimationFrame(r))));
  window.__vi5__.pushLog('info','queued-during-native-capture');
  await settle();const before=nonce();
  window.__vi5__.acknowledge(999);await settle();const wrong=nonce();
  window.__vi5__.acknowledge(12345);await settle();const after=nonce();
  const d=window.__vi5__.ctx.getImageData(0,0,4096,64).data;
  const length=(d[9]|d[10]<<8|d[12]<<16|d[13]<<24)>>>0;
  const message=Array.from({length},(_,i)=>{const j=11+i;return d[4*Math.floor(j/3)+j%3]});
  return {before,wrong,after,message};
 });
 const assert=require('node:assert/strict');assert.equal(ackResult.before,12345);assert.equal(ackResult.wrong,12345);assert.equal(ackResult.after,1);
 const notifications=await require(path.join(root,'packages/vi5/node_modules/jiti')).createJiti(__filename).import(path.join(root,'packages/vi5/src/gen/server-js_pb.ts'));
 assert.ok(pb.fromBinary(notifications.NotificationsSchema,new Uint8Array(ackResult.message)).entries.some(e=>e.entry.case==='log'&&e.entry.value.message==='queued-during-native-capture'));
 // Exercise the p5 path after HTML. The native reader reserves 64 rows for
 // metadata, so even a small canvas must start at y >= 64 and retain its pixels.
 await p.evaluate(()=>window.__vi5__.register({
  id:'p5-protocol-proof',label:'p5 protocol proof',parameters:{},
  async setup(ctx){window.__p5SetupCount=(window.__p5SetupCount??0)+1;await new Promise(resolve=>setTimeout(resolve,150));return ctx.createCanvas(48,24)},
  draw(ctx,sketch){sketch.clear();sketch.noStroke();sketch.fill(123,45,210);sketch.rect(0,0,48,24)},
 }));
 const p5Data=pb.toBinary(schema.BatchRenderRequestSchema,pb.create(schema.BatchRenderRequestSchema,{renderRequests:[{
  renderNonce:18,object:'p5-protocol-proof',objectId:2n,isOffline:false,
  frameInfo:{screenWidth:1920,screenHeight:1080,currentFrame:90,currentTime:1.5,framerate:60,totalFrames:600,totalTime:10},
 }]}));
 // The first preview waits for delayed setup and paints the requested frame.
 await p.evaluate(async encoded=>{await window.__vi5__.render(23456,encoded)},Buffer.from(p5Data).toString('base64'));
 const p5Paint=await p.evaluate(()=>{
  const d=window.__vi5__.ctx.getImageData(0,0,4096,64).data;
  const length=(d[9]|d[10]<<8|d[12]<<16|d[13]<<24)>>>0;
  return {message:Array.from({length},(_,i)=>{const j=11+i;return d[4*Math.floor(j/3)+j%3]}),
   pixel:Array.from(window.__vi5__.ctx.getImageData(10,70,1,1).data)};
 });
 const p5Response=pb.fromBinary(notifications.RootRenderResponseSchema,new Uint8Array(p5Paint.message));
 assert.equal(p5Response.response.case,'success');
 assert.equal(p5Response.response.value.renderResponses[0].nonce,18);
 assert.deepEqual({...p5Response.response.value.renderResponses[0].response.value},
  {$typeName:'serverjs.RendereredObjectInfo',x:0,y:64,width:48,height:24});
 assert.deepEqual(p5Paint.pixel,[123,45,210,255]);
 assert.equal(await p.evaluate(()=>window.__p5SetupCount),1);
 await p.evaluate(()=>window.__vi5__.acknowledge(23456));
 // Failed async p5 setup must produce a correlated error, not leave rendering
 // waiting on an initialization promise that never settles.
 await p.evaluate(()=>window.__vi5__.register({
  id:'p5-setup-failure',label:'p5 setup failure',parameters:{},
  async setup(){throw new Error('expected async p5 setup failure')},draw(){},
 }));
 const failedP5Data=pb.toBinary(schema.BatchRenderRequestSchema,pb.create(schema.BatchRenderRequestSchema,{renderRequests:[{
  renderNonce:19,object:'p5-setup-failure',objectId:3n,isOffline:false,
  frameInfo:{screenWidth:1920,screenHeight:1080,currentFrame:0,currentTime:0,framerate:60,totalFrames:600,totalTime:10},
 }]}));
 await p.evaluate(async encoded=>{
  await Promise.race([window.__vi5__.render(34567,encoded),new Promise((_,reject)=>setTimeout(()=>reject(new Error('p5 setup did not reject')),3000))]);
 },Buffer.from(failedP5Data).toString('base64'));
 const failedMessage=await p.evaluate(()=>{
  const d=window.__vi5__.ctx.getImageData(0,0,4096,64).data;
  const length=(d[9]|d[10]<<8|d[12]<<16|d[13]<<24)>>>0;
  return Array.from({length},(_,i)=>{const j=11+i;return d[4*Math.floor(j/3)+j%3]});
 });
 const failedResponse=pb.fromBinary(notifications.RootRenderResponseSchema,new Uint8Array(failedMessage));
 assert.equal(failedResponse.response.value.renderResponses[0].nonce,19);
 assert.match(failedResponse.response.value.renderResponses[0].response.value,/expected async p5 setup failure/);
 await p.evaluate(()=>window.__vi5__.acknowledge(34567));
 } finally {
  try{await b?.close()}finally{
   if(child.exitCode===null&&!spawnError){const exited=require('node:events').once(child,'exit');child.kill();await exited;}
  }
 }
});
