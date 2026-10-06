import {execFileSync} from 'node:child_process';
import {writeFile,readFile} from 'node:fs/promises';
import {resolve,join} from 'node:path';
import assert from 'node:assert/strict';
const root=resolve('native/libprocessing');
const git=args=>execFileSync('git',args,{cwd:root,encoding:'utf8',stdio:['ignore','pipe','pipe']});
assert.equal(git(['rev-parse','HEAD']).trim(),'0b1a55dd2010a7f170b010ac5fceb48aed7f8d9e');
let patch=git(['diff','--no-ext-diff','--binary']);
for(const file of git(['ls-files','--others','--exclude-standard']).trim().split(/\r?\n/).filter(Boolean)){
  try {patch+=git(['diff','--no-index','--','/dev/null',file]);}
  catch(error){if(error.status!==1)throw error;patch+=error.stdout;}
}
await writeFile(resolve('native/libprocessing-no-smooth.patch'),patch,'utf8');
await writeFile(resolve('native/processing-no-smooth.h'),await readFile(join(root,'crates/processing_ffi/include/processing.h')));
git(['apply','--reverse','--check','../libprocessing-no-smooth.patch']);
console.log(`Independent patch: ${patch.split('\n').length} lines; reverse applicability checked; generated C header copied.`);
