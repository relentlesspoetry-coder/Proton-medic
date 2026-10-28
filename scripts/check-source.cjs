// Dependency-free structural and syntax checks, NOT a substitute for cargo test or tsc.
const fs=require('node:fs');
const path=require('node:path');
let ts;
try { ts=require('typescript'); } catch { const cp=require('node:child_process'); ts=require(require('node:path').join(cp.execFileSync('npm',['root','-g'],{encoding:'utf8'}).trim(),'typescript')); }
const root=path.resolve(__dirname,'..');
let pass=0;let failures=[];
function check(label, condition){ if(condition){console.log('PASS',label);pass++;}else {console.log('FAIL',label);failures.push(label);} }
function walk(root){return fs.readdirSync(root,{withFileTypes:true}).flatMap(e=>e.isDirectory()?walk(path.join(root,e.name)):[path.join(root,e.name)]);}
for(const file of walk(path.join(root,'src')).filter(f=>/\.tsx?$/.test(f))) {
 const s=fs.readFileSync(file,'utf8'); const ast=ts.createSourceFile(file,s,ts.ScriptTarget.Latest,true,file.endsWith('.tsx')?ts.ScriptKind.TSX:ts.ScriptKind.TS);
 check('TypeScript syntax '+path.relative(root,file),ast.parseDiagnostics.length===0);
}
for(const file of ['package.json','src-tauri/tauri.conf.json','src-tauri/capabilities/default.json','tsconfig.json']) {
 try {JSON.parse(fs.readFileSync(path.join(root,file),'utf8')); check('JSON config '+file,true);} catch {check('JSON config '+file,false);}
}
const rust=fs.readFileSync(path.join(root,'src-tauri/src/launch_test.rs'),'utf8');
const lib=fs.readFileSync(path.join(root,'src-tauri/src/lib.rs'),'utf8');
check('Tauri command registered',lib.includes('test_game_launch')&&lib.includes('mod launch_test;'));
check('Frontend command matches backend',fs.readFileSync(path.join(root,'src/LaunchTest.tsx'),'utf8').includes("invoke<LaunchResult>('test_game_launch'"));
check('Consent guard in native module',rust.includes('if !confirmed { return Err('));
check('Steam commands use no shell',!rust.includes('sh -c')&&!rust.includes('.kill(')&&rust.includes('font_runtime::child_command(binary).args(args)') && fs.readFileSync(path.join(root,'src-tauri/src/font_runtime.rs'),'utf8').includes('Command::new(binary)'));
check('Bounded observation',rust.includes('MIN_SECONDS: u64 = 5')&&rust.includes('MAX_SECONDS: u64 = 45'));
check('Disposable fake-Steam fixtures',rust.includes('fake-steam')&&rust.includes('fn fake_steam_launcher_is_executed_without_real_game_files'));
check('Original safe repair modules retained',['repairs.rs','auto_select.rs','advisor.rs','diagnostics.rs','scanner.rs'].every(f=>fs.existsSync(path.join(root,'src-tauri/src',f))));
console.log(`SOURCE CHECKS ${pass} passed, ${failures.length} failed`);
process.exitCode=failures.length?1:0;
