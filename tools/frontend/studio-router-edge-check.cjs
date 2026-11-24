// F1 review regressions. Synthetic loopback fixture only.
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const assert=require('node:assert/strict');const fs=require('node:fs');
(async()=>{
 const browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH,headless:true,args:['--no-sandbox']});
 const context=await browser.newContext();const failures=[];
 const out=process.env.ROUTERS_EVIDENCE_DIR||'/tmp/lqosync-studio-router-edge';fs.mkdirSync(out,{recursive:true});
 try{
  const login=await context.newPage();await login.goto('http://127.0.0.1:19581');await login.getByLabel('Username',{exact:true}).fill('browser-test');await login.getByLabel('Password',{exact:true}).fill(process.env.BROWSER_TEST_PASSWORD);await login.getByRole('button',{name:'Sign in',exact:true}).click();await login.getByRole('navigation',{name:'Main navigation'}).waitFor();await login.close();
  async function check(name,size,run){const p=await context.newPage();await p.setViewportSize(size);
   try{
    await p.goto('http://127.0.0.1:19581/topology');await p.locator('.canvas-node').nth(3).waitFor();await p.getByRole('button',{name:'Router source configuration',exact:true}).click();
    const modal=p.getByRole('dialog',{name:'Router source configuration',exact:true});await modal.getByRole('button',{name:'Configure Access-01',exact:true}).click();await modal.getByRole('button',{name:'Access profiles',exact:true}).click();
    await run(p,modal);await p.screenshot({path:`${out}/studio-router-edge-${name}.png`});console.log(`PASS: ${name}`);
   }catch(e){failures.push(`${name}: ${e.message}`);console.error(`FAIL: ${name}: ${e.message}`);await p.screenshot({path:`${out}/studio-router-edge-${name}-failure.png`}).catch(()=>{});}finally{await p.close()}
  }
  await check('phone-selector',{width:390,height:844},async(p,modal)=>{
   assert.ok(await modal.locator('.studio-router-list').evaluate(e=>e.clientHeight)>=60,'router strip has enough height for its choices');
   const choice=modal.getByRole('button',{name:'Configure Access-01',exact:true});await choice.scrollIntoViewIfNeeded();
   assert.ok(await choice.evaluate(e=>{const r=e.getBoundingClientRect();return document.elementFromPoint(r.x+r.width/2,r.y+r.height/2)?.closest('button')===e}),'registered router center is reachable');
   const add=modal.getByRole('button',{name:'Add router',exact:true});await add.scrollIntoViewIfNeeded();await add.click();
   await p.getByRole('dialog',{name:'Add router',exact:true}).waitFor();await p.keyboard.press('Escape');
   await modal.getByRole('button',{name:'Source policies',exact:true}).click();await modal.getByLabel('Policy name',{exact:true}).fill('Phone PPP');
   const content=modal.locator('.studio-router-content');await content.evaluate(e=>e.scrollTop=e.scrollHeight);
   await modal.getByLabel('Minimum upload (bps)',{exact:true}).fill('12345');await content.evaluate(e=>e.scrollTop=0);
   assert.equal(await modal.getByLabel('Policy name',{exact:true}).inputValue(),'Phone PPP','independent content scrolling keeps the draft');
  });
  await check('secret-panel-close',{width:1440,height:1000},async(p,modal)=>{
   const writes=[];p.on('request',r=>{if(r.url().includes('/api/v1/')&&r.method()!=='GET')writes.push(r.url())});
   await modal.getByRole('button',{name:'Add access profile',exact:true}).click();await modal.getByLabel('RouterOS user',{exact:true}).fill('profile-reader');await modal.getByLabel('Credential',{exact:true}).fill('F1-panel-secret');
   await modal.getByRole('button',{name:'Add router',exact:true}).click();await p.getByRole('dialog',{name:'Add router',exact:true}).waitFor();await p.keyboard.press('Escape');
   await modal.getByLabel('Profile label',{exact:true}).waitFor();
   assert.equal(await modal.getByLabel('Credential',{exact:true}).inputValue(),'','leaving the secret panel clears credential input');
   assert.equal(await modal.getByLabel('RouterOS user',{exact:true}).inputValue(),'profile-reader','non-secret profile input survives');assert.deepEqual(writes,[],'opening and cancelling registration sends no writes');
  });
  assert.deepEqual(failures,[]);
 }finally{await browser.close()}
})().catch(e=>{console.error(e);process.exit(1)});
