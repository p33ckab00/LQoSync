// An old cached request must not end a newly authenticated session.
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');const assert=require('node:assert/strict');
(async()=>{
 const browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH,headless:true,args:['--no-sandbox']});let release;
 try{
  const p=await browser.newPage({viewport:{width:1440,height:1000}});const errors=[];p.on('pageerror',e=>errors.push(e.message));
  async function signin(){await p.getByLabel('Username',{exact:true}).fill('browser-test');await p.getByLabel('Password',{exact:true}).fill(process.env.BROWSER_TEST_PASSWORD);await p.getByRole('button',{name:'Sign in',exact:true}).click();await p.locator('.canvas-node').nth(3).waitFor();}
  async function editor(){await p.getByRole('button',{name:'Router source configuration',exact:true}).click();const modal=p.getByRole('dialog',{name:'Router source configuration',exact:true});await modal.getByRole('button',{name:'Configure Access-01',exact:true}).click();await modal.getByRole('button',{name:'Source policies',exact:true}).click();return modal;}
  await p.goto('http://127.0.0.1:19581/topology');await signin();const modal=await editor();await modal.getByLabel('Policy name',{exact:true}).fill('Retain after late reply');
  let first=true,seen,done;const held=new Promise(r=>release=r),started=new Promise(r=>seen=r),finished=new Promise(r=>done=r);
  await p.route('**/api/v1/routers/lq3-00000000000000a1/source-config',async route=>{if(!first){await route.fallback();return;}first=false;seen();await held;await route.fulfill({status:401,body:''});done();});
  let caps=0;await p.route('**/api/v1/topology/source-capabilities',async route=>{caps++;if(caps===2){await route.fulfill({status:401,body:''});}else{await route.fallback();}});
  await modal.getByRole('button',{name:'Reload router configuration',exact:true}).click();await started;
  await modal.getByRole('button',{name:'Reload router configuration',exact:true}).click();await p.getByRole('button',{name:'Sign in',exact:true}).waitFor();await signin();
  release();await finished;await p.waitForTimeout(150);
  assert.ok(await p.getByRole('navigation',{name:'Main navigation'}).isVisible(),'late 401 cannot clear the new session');
  await p.unroute('**/api/v1/routers/lq3-00000000000000a1/source-config');await p.unroute('**/api/v1/topology/source-capabilities');
  const recovered=await editor();assert.equal(await recovered.getByLabel('Policy name',{exact:true}).inputValue(),'Retain after late reply');assert.deepEqual(errors,[]);
  console.log('PASS: a late cached 401 cannot end the new session or discard its actor-bound draft.');
 }finally{if(release)release();await browser.close()}
})().catch(e=>{console.error(e);process.exit(1)});
