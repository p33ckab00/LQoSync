// Synthetic fixture: small screens, initial reads, and incomplete WAN drafts.
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const assert=require('node:assert/strict');
const fs=require('node:fs');
(async()=>{
 const browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH,headless:true,args:['--no-sandbox']});
 const context=await browser.newContext();const failures=[];
 const out=process.env.ROUTERS_EVIDENCE_DIR||'/tmp/lqosync-studio-shell';fs.mkdirSync(out,{recursive:true});
 try{
  const login=await context.newPage();await login.goto('http://127.0.0.1:19581');
  await login.getByLabel('Username',{exact:true}).fill('browser-test');
  await login.getByLabel('Password',{exact:true}).fill(process.env.BROWSER_TEST_PASSWORD);
  await login.getByRole('button',{name:'Sign in',exact:true}).click();
  await login.getByRole('navigation',{name:'Main navigation'}).waitFor();await login.close();
  async function check(name,size,run,cold=false){
   const page=await context.newPage();await page.setViewportSize(size);
   try{
    if(!cold){await page.goto('http://127.0.0.1:19581/topology');await page.locator('.canvas-node').nth(3).waitFor();}
    await run(page);await page.screenshot({path:`${out}/studio-edge-${size.width}-${name.split(' ')[0]}.png`});console.log(`PASS: ${name}`);
   }catch(e){failures.push(`${name}: ${e.message}`);console.error(`FAIL: ${name}: ${e.message}`)}
   finally{await page.close()}
  }
  await check('landscape world and last tool',{width:667,height:375},async p=>{
   const viewport=p.locator('.canvas-viewport');assert.ok(await viewport.evaluate(e=>e.clientHeight)>=80,'world has at least 80 px height');
   const north=viewport.getByRole('button',{name:/^WAN North,/});await north.scrollIntoViewIfNeeded();await north.click();
   const last=p.getByRole('button',{name:'Card settings',exact:true});await last.scrollIntoViewIfNeeded();
   const frame=await p.locator('.canvas-frame').boundingBox(),box=await last.boundingBox();
   assert.ok(box.y>=frame.y&&box.y+box.height<=frame.y+frame.height,'last tool stays in the canvas');
   await p.screenshot({path:`${out}/studio-landscape-canvas.png`});
   await last.click();await p.getByRole('dialog',{name:'WAN North',exact:true}).waitFor();
  });
  await check('phone reload hit target',{width:375,height:667},async p=>{
   assert.ok(await p.getByRole('button',{name:'Reload view',exact:true}).evaluate(e=>{const r=e.getBoundingClientRect();return document.elementFromPoint(r.x+r.width/2,r.y+r.height/2)?.closest('button')===e}),'Reload center reaches Reload');
  });
  await check('cold loading and failed first read',{width:1440,height:1000},async p=>{
   let release,seen;const held=new Promise(r=>release=r),started=new Promise(r=>seen=r);
   await p.route('**/api/v1/topology',async route=>{seen();await held;await route.fulfill({status:503,body:''}).catch(()=>{})});
   try{
    await p.goto('http://127.0.0.1:19581/topology');await started;
    const pending=p.getByRole('status').filter({hasText:'Loading the topology view…'});await pending.waitFor();
    const box=await pending.boundingBox(),head=await p.locator('.topology-head').boundingBox();
    assert.ok(box.y>=head.y+head.height,'loading stays below navigation');
    assert.equal(await p.locator('.canvas-evidence:visible').count(),0,'no empty evidence');
   }finally{release()}
   await p.getByRole('status').filter({hasText:'The topology view is not available yet.'}).waitFor();
   assert.equal(await p.getByText('Loading the topology view…',{exact:true}).count(),0,'failed first read ends loading');
  },true);
  await check('WAN draft survives nested information',{width:1440,height:1000},async p=>{
   const writes=[];p.on('request',r=>{if(r.method()!=='GET'&&r.url().includes('/api/v1/'))writes.push(r.url())});
   await p.getByRole('button',{name:'Router inventory',exact:true}).click();
   const inventory=p.getByRole('dialog',{name:'Router inventory',exact:true});
   await inventory.getByRole('button',{name:'Add pppoe-out3 as a WAN',exact:true}).click();
   const form=inventory.locator('.canvas-wan-form');await form.getByLabel('Name',{exact:true}).fill('Unfinished uplink');
   await form.getByLabel('Download Mbps',{exact:true}).fill('321');
   const info=inventory.getByRole('button',{name:'Configuration information',exact:true});await info.click({timeout:5000});
   await p.getByRole('dialog',{name:'Configuration workflow',exact:true}).waitFor();await p.keyboard.press('Escape');
   assert.equal(await info.evaluate(e=>e===document.activeElement),true,'help returns focus to the editor');
   assert.equal(await form.getByLabel('Name',{exact:true}).inputValue(),'Unfinished uplink');
   assert.equal(await form.getByLabel('Download Mbps',{exact:true}).inputValue(),'321');
   assert.deepEqual(writes,[],'incomplete draft and help send no writes');
  });
  await check('standalone appearance icons',{width:375,height:667},async p=>{
   assert.equal(await p.locator('.main-top').getByLabel('Color theme',{exact:true}).count(),0,'Builder has no external theme selector');
   const dock=p.locator('.canvas-appearance');await dock.getByRole('button',{name:'Canvas appearance',exact:true}).click();
   await dock.getByRole('button',{name:'Dark theme',exact:true}).click();
   assert.equal(await p.locator('html').getAttribute('data-theme'),'dark');
  });
  assert.deepEqual(failures,[]);
 }finally{await browser.close()}
})().catch(e=>{console.error(e);process.exit(1)});
