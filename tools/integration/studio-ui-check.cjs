// TEST EVIDENCE ONLY: the real Wasm UI against the smoke gateway, broker, core, and fake router.
// No request is mocked or changed. Run by studio-service-smoke.sh after its API checks.
// Environment: PLAYWRIGHT_MODULE, CHROMIUM_PATH, STUDIO_URL, STUDIO_PASSWORD, STUDIO_OUT.
const {chromium}=require(process.env.PLAYWRIGHT_MODULE);const assert=require('node:assert/strict');
(async()=>{const browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH,headless:true,args:['--no-sandbox']});let p;
try{
 p=await browser.newPage({viewport:{width:1440,height:1000}});const errors=[],writes=[];
 p.on('pageerror',e=>errors.push(e.message));
 p.on('request',r=>{if(r.url().includes('/api/v1/')&&r.method()!=='GET'&&!r.url().endsWith('/session'))writes.push({url:new URL(r.url()).pathname,body:JSON.parse(r.postData()||'{}')});});
 await p.goto(process.env.STUDIO_URL);
 await p.getByLabel('Username',{exact:true}).fill('smoke-operator');await p.getByLabel('Password',{exact:true}).fill(process.env.STUDIO_PASSWORD);
 await p.getByRole('button',{name:'Sign in',exact:true}).click();
 await p.getByRole('navigation',{name:'Main navigation'}).getByRole('link',{name:'Topology',exact:true}).click();await p.locator('.canvas-node').first().waitFor();
 const modal=p.getByRole('dialog',{name:'Router source configuration',exact:true});const name=()=>modal.getByLabel('Policy name',{exact:true});
 await p.getByRole('button',{name:'Router source configuration',exact:true}).click();
 await modal.getByRole('button',{name:'Configure Smoke-01',exact:true}).click();await modal.getByRole('button',{name:'Source policies',exact:true}).click();
 assert.equal(await name().inputValue(),'PPPoE','the editor loads the policy that the service saved');
 assert.equal(writes.length,0,'opening the editor sends no mutation');
 await name().fill('PPPoE from UI');
 // Real collection and evaluation through the service.
 await modal.getByRole('button',{name:'Refresh discovery',exact:true}).click();await modal.getByText(/^snap-/).first().waitFor({timeout:30000});
 await modal.getByRole('button',{name:'Preview draft',exact:true}).click();await p.locator('.studio-preview-card').first().waitFor({timeout:30000});
 await p.getByRole('region',{name:'Read-only source preview',exact:true}).waitFor();
 assert.deepEqual(writes.map(w=>`${w.url}:${w.body.job?.kind}`),['/api/v1/topology/source-jobs:collect','/api/v1/topology/source-jobs:evaluate'],'preview uses only source jobs');
 await p.screenshot({path:`${process.env.STUDIO_OUT}/studio-real-preview.png`});
 // Save through the real writer path.
 await p.getByRole('button',{name:'Edit source draft',exact:true}).click();assert.equal(await name().inputValue(),'PPPoE from UI');
 await modal.getByRole('button',{name:'Save configuration',exact:true}).click();
 const status=modal.getByRole('region',{name:'Source save status',exact:true});await status.getByText(/Saved policy revision \d+/).waitFor({timeout:30000});
 const save=writes.find(w=>w.body.edit?.kind==='router.sources_set');assert.ok(save,'the UI sent router.sources_set');
 const op=await p.evaluate(async k=>(await fetch(`/api/v1/topology/operations/${k}`)).json(),save.body.key);
 assert.equal(op.state,'saved');assert.equal(Number(op.receipt.revision),save.body.expected_revision+1);
 const rid=save.body.edit.router_id;const cfg=await p.evaluate(async id=>(await fetch(`/api/v1/routers/${id}/source-config`)).json(),rid);
 assert.equal(cfg.policies[0].name,'PPPoE from UI','the service holds the saved name');assert.equal(String(cfg.policy_revision),String(op.receipt.revision));
 assert.match(await status.innerText(),/Saved policy revision/);await p.screenshot({path:`${process.env.STUDIO_OUT}/studio-real-save.png`});
 assert.deepEqual(errors,[]);
 console.log(`UI operation ${save.body.key} saved revision ${op.receipt.revision} (base ${save.body.expected_revision}); service policy "${cfg.policies[0].name}"`);
}catch(e){if(p)await p.screenshot({path:`${process.env.STUDIO_OUT}/studio-real-failure.png`}).catch(()=>{});throw e}finally{await browser.close()}
})().catch(e=>{console.error(e);process.exit(1)});
