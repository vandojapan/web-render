import {spawn} from 'node:child_process';
import {mkdir,writeFile} from 'node:fs/promises';
import assert from 'node:assert/strict';
const [exe,root,proof] = process.argv.slice(2);
assert(exe && root && proof);
const worker = spawn(exe,[root],{windowsHide:true,stdio:['pipe','pipe','pipe']});
let buffer=Buffer.alloc(0),wake;
let stderr='';
worker.stdout.on('data',data=>{buffer=Buffer.concat([buffer,data]);wake?.();wake=undefined;});
worker.stderr.on('data',data=>stderr+=data.toString());
let exitCode;
worker.on('exit',code=>{exitCode=code;wake?.();wake=undefined;});
const watchdog=setTimeout(()=>{worker.kill();},60000);
async function wait(){assert.equal(exitCode,undefined,`Worker exited early: ${stderr}`);await new Promise(resolve=>wake=resolve);}
async function line(){while(buffer.indexOf(10)<0)await wait();const n=buffer.indexOf(10),s=buffer.subarray(0,n).toString();buffer=buffer.subarray(n+1);return JSON.parse(s);}
async function bytes(n){while(buffer.length<n)await wait();const value=buffer.subarray(0,n);buffer=buffer.subarray(n);return value;}
const reports=[];
try {
  const ready=await line();assert.equal(ready.ready,1);
  let nonce=1;
  async function render(object,width=32,height=8,count=1){
    const request={version:1,nonce:nonce++,object,object_id:10,width,height,time:0,count};
    worker.stdin.write(JSON.stringify(request)+'\n');
    const response=await line();assert.equal(response.nonce,request.nonce);
    const pixels=await bytes(response.bytes);reports.push(response);return {response,pixels};
  }
  let result=await render('image-no-smooth');assert.equal(result.response.created_graphics,true);
  assert.deepEqual([...result.pixels.subarray(0,4)],[255,0,0,255]);
  assert.deepEqual([...result.pixels.subarray(31*4,32*4)],[0,0,255,255]);
  result=await render('image-no-smooth');assert.equal(result.response.created_graphics,false);
  result=await render('alpha-no-smooth');
  assert.deepEqual([...result.pixels.subarray(0,4)],[255,255,255,64]);
  assert.deepEqual([...result.pixels.subarray(31*4,32*4)],[255,255,255,192]);
  result=await render('image-no-smooth',64,16);assert.equal(result.response.created_graphics,true);
  result=await render('unknown');assert(result.response.error);assert.equal(result.pixels.length,0);
  result=await render('image-no-smooth',64,16);assert.equal(result.response.error,null);
  worker.stdin.end('shutdown\n');
  if(exitCode===undefined) await new Promise(resolve=>worker.once('exit',resolve));
  assert.equal(exitCode,0,stderr);
  await mkdir(proof,{recursive:true});
  await writeFile(`${proof}/worker-smoke.json`,JSON.stringify({status:'passed',ready,reports,exit_code:exitCode},null,2));
  await writeFile(`${proof}/worker-stderr.txt`,stderr);
  console.log('Native worker IPC, nearest, straight alpha, resize, error recovery and graceful exit passed');
} finally {clearTimeout(watchdog);if(exitCode===undefined)worker.kill();}
