import {readFile,writeFile,readdir} from 'node:fs/promises';
import {resolve,join} from 'node:path';
import {createHash} from 'node:crypto';
import {deflateSync} from 'node:zlib';
import assert from 'node:assert/strict';
const proof=resolve(process.argv[2]??'docs/proofs/libprocessing');
const isolated=join(proof,'isolated');
const json=async p=>JSON.parse((await readFile(p,'utf8')).replace(/^\uFEFF/,''));
const csv=async p=>{const lines=(await readFile(p,'utf8')).trim().split(/\r?\n/);const keys=lines.shift().split(',');return lines.map(line=>Object.fromEntries(line.split(',').map((v,i)=>[keys[i],Number.isFinite(Number(v))?Number(v):v])));};
const hash=b=>createHash('sha256').update(b).digest('hex');
const median=values=>values.toSorted((a,b)=>a-b)[Math.floor(values.length/2)];
const crcTable=Array.from({length:256},(_,i)=>{for(let b=0;b<8;b++)i=i&1?0xedb88320^(i>>>1):i>>>1;return i>>>0;});
const crc=bytes=>{let c=0xffffffff;for(const b of bytes)c=crcTable[(c^b)&255]^(c>>>8);return(c^0xffffffff)>>>0;};
const chunk=(tag,data)=>{const head=Buffer.alloc(4);head.writeUInt32BE(data.length);const body=Buffer.concat([Buffer.from(tag),data]);const tail=Buffer.alloc(4);tail.writeUInt32BE(crc(body));return Buffer.concat([head,body,tail]);};
async function png(path,rgba,w,h){
  assert.equal(rgba.length,w*h*4);
  const header=Buffer.alloc(13);header.writeUInt32BE(w,0);header.writeUInt32BE(h,4);header[8]=8;header[9]=6;
  const rows=Buffer.alloc(h*(w*4+1));for(let y=0;y<h;y++)rgba.copy(rows,y*(w*4+1)+1,y*w*4,(y+1)*w*4);
  await writeFile(path,Buffer.concat([Buffer.from([137,80,78,71,13,10,26,10]),chunk('IHDR',header),chunk('sRGB',Buffer.from([0])),chunk('IDAT',deflateSync(rows)),chunk('IEND',Buffer.alloc(0))]));
}
assert.match(await readFile(join(proof,'images/passed.txt'),'utf8'),/passed/);
const cef=await json(join(proof,'cef/cef-result.json'));assert.equal(cef.status,'passed');
// Correct the initial probe's metadata label: frame caching lives in aux2,
// and direct CEF IPC duplicates are redraws, never host-cache measurements.
for(const row of cef.results){if('cache_hit_samples' in row){row.same_frame_samples=row.cache_hit_samples;row.same_frame_redraw_median_ms=row.cache_hit_median_ms;delete row.cache_hit_samples;delete row.cache_hit_median_ms;}row.host_frame_cache_present=false;}
await writeFile(join(proof,'cef/cef-result.json'),JSON.stringify(cef,null,2));
const metrics=await json(join(isolated,'process-metrics.json'));
const gpu=(await readFile(join(isolated,'gpu.csv'),'utf8')).trim().split(/\r?\n/).map(line=>{const [timestamp,index,util,memory,total]=line.split(',').map(v=>v.trim());return{time:Date.parse(timestamp),index:Number(index),util:Number(util),memory_mib:Number(memory),total_mib:Number(total)};});
const results=[];const golden=[];const raster=[];
for(const [w,h] of [[1920,1080],[3840,2160]])for(const scene of ['shapes','text','image']){
  const rows={};const pixels={};
  for(const mode of ['default','no-smooth']){
    const label=`${mode}-${w}-${scene}`;const row=(await csv(join(isolated,`${label}.csv`)))[0];
    const stages=await csv(join(isolated,`${label}-stages.csv`));
    const process=metrics.find(m=>m.label===label);assert.equal(process.exit_code,0);
    const start=Date.parse(process.steady_start_observed),end=Date.parse(process.steady_end_observed??process.finished);
    const observed=gpu.filter(g=>g.index===0&&g.time>=start&&g.time<=end);
    rows[mode]={...row,stages_median_ms:Object.fromEntries(['command_generation_ms','cpu_tessellation_and_submit_ms','readback_including_gpu_wait_ms'].map(k=>[k,median(stages.map(s=>s[k]))])),
      cpu_percent_of_16_logical_processors:100*process.steady_cpu_ms_observed/(process.steady_wall_ms_observed*16),peak_working_set_mib:process.peak_working_set_bytes/1048576,peak_private_mib:process.peak_private_bytes/1048576,
      gpu_utilization_global_mean_percent:observed.length?observed.reduce((s,g)=>s+g.util,0)/observed.length:null,gpu_global_memory_peak_mib:observed.length?Math.max(...observed.map(g=>g.memory_mib)):null,gpu_monitor_samples:observed.length};
    pixels[mode]=await readFile(join(isolated,`${label}.rgba`));
    await png(join(isolated,`${label}.png`),pixels[mode],w,h);
  }
  const baseline=await readFile(join(proof,`baseline-${w}-${scene}.rgba`));
  assert.equal(hash(baseline),hash(pixels.default),`unspecified output changed: ${w} ${scene}`);
  golden.push({width:w,height:h,scene,baseline_sha256:hash(baseline),unchanged:true});
  const browser=await readFile(join(proof,`cef/cef-${w}-${scene}.rgba`));assert.equal(browser.length,w*h*4);
  await png(join(proof,`cef/cef-${w}-${scene}.png`),browser,w,h);
  for(const mode of ['default','no-smooth']){
    let changed=0,sum=0,max=0;const native=pixels[mode];
    for(let i=0;i<native.length;i+=4){let pixelChanged=false;for(let c=0;c<4;c++){const d=Math.abs(native[i+c]-browser[i+c]);sum+=d;max=Math.max(max,d);pixelChanged ||= d!==0;}if(pixelChanged)changed++;}
    raster.push({width:w,height:h,scene,native_mode:mode,differing_pixels:changed,mean_absolute_rgba_error:sum/native.length,max_channel_error:max});
  }
  results.push({width:w,height:h,scene,default:rows.default,no_smooth:rows['no-smooth'],median_reduction_percent:100*(1-rows['no-smooth'].median_total_readback_ms/rows.default.median_total_readback_ms),cef:cef.results.find(r=>r.width===w&&r.scene===scene)});
}
for(const file of (await readdir(join(proof,'images'))).filter(f=>f.endsWith('.rgba'))){
  let bytes=await readFile(join(proof,'images',file));
  // PNG uses straight alpha. GPU RGB is sRGB-encoded linear-premultiplied color.
  if(file.includes('alpha')){bytes=Buffer.from(bytes);for(let i=0;i<bytes.length;i+=4){const a=bytes[i+3]/255;for(let c=0;c<3;c++){const s=bytes[i+c]/255;const lin=s<=0.04045?s/12.92:((s+0.055)/1.055)**2.4;const straight=a?Math.min(1,lin/a):0;bytes[i+c]=Math.round(255*(straight<=0.0031308?12.92*straight:1.055*straight**(1/2.4)-0.055));}}}
  await png(join(proof,'images',file.replace(/\.rgba$/,'.png')),bytes,256,256);
}
const repeat_results=[];
for(const [w,h] of [[1920,1080],[3840,2160]])for(const scene of ['shapes','text','image']){
  const a=(await csv(join(proof,`repeat/default-${w}-${scene}.csv`)))[0],b=(await csv(join(proof,`repeat/no-smooth-${w}-${scene}.csv`)))[0];
  for(const mode of ['default','no-smooth'])assert.equal(hash(await readFile(join(proof,`repeat/${mode}-${w}-${scene}.rgba`))),hash(await readFile(join(isolated,`${mode}-${w}-${scene}.rgba`))));
  repeat_results.push({width:w,height:h,scene,default:a,no_smooth:b,median_reduction_percent:100*(1-b.median_total_readback_ms/a.median_total_readback_ms),all_pixels_identical_to_first_run:true});
}
const report={date:'2026-10-07',status:'passed',libprocessing_commit:'0b1a55dd2010a7f170b010ac5fceb48aed7f8d9e',environment:{os:'Windows 11 Pro 26200',cpu:'AMD Ryzen 7 7700',logical_processors:16,ram_gib:31.2,gpu:'NVIDIA GeForce RTX 4070 Ti SUPER',driver:'591.86',native_backend:'Vulkan',native_build:'release',wgpu:'29.0.4'},results,repeat_results,golden,raster_differences:raster,
  measurement_limits:['Native totals include CPU command generation, tessellation, submission and tightly packed GPU readback. They exclude IPC and AviUtl2 handoff.','Readback stage includes remaining GPU draw/transfer wait; no GPU timestamp-query measurement.','CEF totals include JavaScript, paint/readback, compression, IPC and decode; they are not directly equal scope to native totals.','No native frame cache is implemented. Direct CEF IPC bypasses the aux2 frame cache; same-frame calls are deterministic redraw probes, not cache hits. Native/host cache-hit timing remains unmeasured.','CPU telemetry is a coarse 50ms observation of the marked steady interval; GPU telemetry is global NVIDIA utilization/memory at 100ms and includes other desktop processes.','Native unspecified golden images came from the unchanged library; their original timing runs overlapped a build, so only the isolated default/noSmooth measurements are used for conclusions.','Native AviUtl2 preview/seek/export and JavaScript/MIDI adapters are not connected in this native-first implementation.']};
await writeFile(join(proof,'comparison.json'),JSON.stringify(report,null,2));
console.table(results.map(r=>({size:`${r.width}x${r.height}`,scene:r.scene,default_ms:r.default.median_total_readback_ms.toFixed(3),no_smooth_ms:r.no_smooth.median_total_readback_ms.toFixed(3),reduction_percent:r.median_reduction_percent.toFixed(1),cef_ipc_ms:r.cef.median_ms.toFixed(3)})));
console.log(`PASS GPU cases and ${golden.length} unchanged default golden images`);
