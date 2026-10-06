import fs from 'node:fs/promises';
import crypto from 'node:crypto';
const proof='docs/proofs/libprocessing-aviutl';
const read=async path=>JSON.parse((await fs.readFile(`${proof}/${path}`,'utf8')).replace(/^\uFEFF/,''));
const preview=await read('verified/preview-result.json');
const reference=await read('verified/pixel-reference.json');
const oldAlpha=(await read('replay/preview-result.json')).cases.find(c=>c.object==='alpha-no-smooth');
const newAlpha=preview.cases.find(c=>c.object==='alpha-no-smooth');
const p5=await read('p5-regression/baseline-comparison.json');
const himawari=await read('himawari-regression/result.json');
const smoke=await read('release-smoke/display.json');
const artifacts={};
for(const path of ['target/release/web_render_aux2.dll','native/libprocessing/target/release/web-render-processing-server.exe',
  '.aviutl2-cli/development/data/Plugin/web-render/web-render.aux2','.aviutl2-cli/development/data/Plugin/web-render/web-render-processing-server.exe',
  'examples/processing/web-render.processing.json','Cargo.lock','native/web-render-processing-server/Cargo.lock']){
  artifacts[path]=crypto.createHash('sha256').update(await fs.readFile(path)).digest('hex');
}
const result={
  status:'preview_verified_export_unverified',backend:'libprocessing',display:'DISPLAY2',dimensions:[1920,1080],
  measurement:'debug plugin + release native worker; SDK scene capture including IPC and host composition; 30 steady frames per case',
  preview:{status:'passed',...preview,pixel_reference:reference},
  alpha_conversion:{before_host_median_ms:oldAlpha.median_host_ms,after_host_median_ms:newAlpha.median_host_ms,
    improvement_percent:(1-newAlpha.median_host_ms/oldAlpha.median_host_ms)*100},
  standard_png_export:{status:'unverified',sdk:'CommonEditSection::output_file reports not supported',
    gui:'Save dialog opened on DISPLAY2; foreground/hit-test remained a different window; no completed output',
    last_attempt:await read('export-input/result.json')},
  cef_regression:{himawari:himawari.status,p5_all_nine_pngs_match_previous_capture:p5.every(c=>c.byte_equal),
    p5_known_repeated_frame_differences:[85,60]},
  unit_tests:{passed:18,ignored_child_fixture:1},release_smoke:smoke,artifacts,
};
if(reference.status!=='passed'||himawari.status!=='passed'||!p5.every(c=>c.byte_equal)||smoke.exit_code!==0)throw Error('Verification failed');
if(artifacts['target/release/web_render_aux2.dll']!==artifacts['.aviutl2-cli/development/data/Plugin/web-render/web-render.aux2'])throw Error('Deployment mismatch');
if(artifacts['native/libprocessing/target/release/web-render-processing-server.exe']!==artifacts['.aviutl2-cli/development/data/Plugin/web-render/web-render-processing-server.exe'])throw Error('Worker deployment mismatch');
await fs.writeFile(`${proof}/summary.json`,JSON.stringify(result,null,2));
console.log(JSON.stringify({status:result.status,alpha_improvement_percent:result.alpha_conversion.improvement_percent,
  native_cases:preview.cases.length,release_smoke:smoke.exit_code,p5_baseline_equal:true},null,2));
