// Synthetic loopback fixture only. No production accounts or telemetry.
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const assert=require('node:assert/strict');const fs=require('node:fs');
(async()=>{
 const browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH,headless:true,args:['--no-sandbox']});
 try{
  const page=await browser.newPage({viewport:{width:1440,height:1100},colorScheme:'dark',hasTouch:true});
  const errors=[];page.on('pageerror',e=>errors.push(e.message));
  await page.addInitScript(()=>{const Native=EventSource;window.testStreams=[];window.EventSource=class extends Native{constructor(...a){super(...a);window.testStreams.push(this);}};});
  await page.goto('http://127.0.0.1:19581');await page.getByLabel('Username',{exact:true}).fill('browser-test');await page.getByLabel('Password',{exact:true}).fill(process.env.BROWSER_TEST_PASSWORD);await page.getByRole('button',{name:'Sign in',exact:true}).click();
  const tile=page.locator('.nic-physical-port').first();await tile.waitFor();
  await tile.hover();const hud=page.getByRole('region',{name:'Port 0 NIC diagnostics',exact:true});await hud.waitFor();
  await hud.getByText('100.00 kB',{exact:true}).waitFor();assert.match(await hud.innerText(),/800.0 kbps/);
  await hud.hover();await page.waitForTimeout(350);assert.equal(await hud.isVisible(),true,'pointer can enter the panel');
  await page.keyboard.press('Escape');await hud.waitFor({state:'detached'});await page.mouse.move(5,5);
  await tile.focus();await page.keyboard.press('Enter');await hud.waitFor();
  const first=await hud.locator('.nic-hud-stamp').innerText();await page.waitForFunction(first=>document.querySelector('.nic-hud-stamp').textContent!==first,first,{timeout:5000});
  await hud.getByText('NIC counter details',{exact:true}).click();await page.waitForTimeout(1200);assert.equal(await hud.locator('details').getAttribute('open'),'');
  await hud.getByText('NIC counter details',{exact:true}).click();
  await page.evaluate(async()=>{window.nicView=await(await fetch('/api/v1/overview')).json();window.testStreams.forEach(s=>s.close());});
  await page.route('**/api/v1/overview',async r=>r.fulfill({status:200,contentType:'application/json',body:JSON.stringify(await page.evaluate(()=>window.nicView))}));
  const send=async()=>page.evaluate(()=>window.testStreams.at(-1).dispatchEvent(new MessageEvent('message',{data:JSON.stringify(window.nicView)})));
  for(let i=0;i<3;i++){await page.evaluate(()=>{const d=window.nicView.dpdk;d.sampled_at++;d.nic.updated=d.sampled_at;d.ports[0].activity.sampled_at=d.sampled_at;});await send();await page.waitForTimeout(50);}
  assert.notEqual(await hud.locator('.nic-hud-rx path').getAttribute('d'),'');
  await page.keyboard.press('Escape');await page.setViewportSize({width:1440,height:1600});await tile.focus();await page.keyboard.press('Enter');await hud.waitFor();
  const out=process.env.NIC_HUD_EVIDENCE_DIR||'/tmp/lqosync-nic-hud-evidence';fs.mkdirSync(out,{recursive:true});
  for(const theme of ['dark','light']){await page.getByLabel('Color theme').selectOption(theme);await page.locator('.nic-hud-position').screenshot({path:`${out}/${theme}.png`});}
  await page.evaluate(()=>{const p=window.nicView.dpdk.ports[0];p.reset=true;p.activity.sampled_at++;});await send();await hud.getByText('Rate unavailable',{exact:true}).first().waitFor();assert.match(await hud.locator('.nic-hud-stamp').innerText(),/Counter reset/);
  await page.evaluate(()=>{const d=window.nicView.dpdk;d.nic.stale=true;d.ports[0].reset=false;});await send();await hud.getByText('Stale / unknown',{exact:true}).waitFor();assert.match(await hud.locator('.nic-hud-stamp').innerText(),/Last known/);
  await page.evaluate(()=>{const d=window.nicView.dpdk;d.nic.stale=false;d.ports[0].details=d.ports[0].details.filter(r=>!r.label.startsWith('NIC rx_'));});await send();await hud.getByText('Composition unavailable for these driver counters.',{exact:true}).waitFor();
  await hud.getByRole('button',{name:'Close NIC diagnostics',exact:true}).click();await hud.waitFor({state:'detached'});
  await page.setViewportSize({width:320,height:1200});await tile.scrollIntoViewIfNeeded();await tile.tap();await hud.waitFor();
  const box=await page.locator('.nic-hud-position').boundingBox();assert.ok(box.x>=0 && box.x+box.width<=320);assert.ok(box.y>=0 && box.y+box.height<=1200);assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
  await page.locator('.nic-hud-position').screenshot({path:`${out}/mobile.png`});
  await page.emulateMedia({forcedColors:'active',reducedMotion:'reduce'});assert.notEqual(await hud.locator('svg path').first().evaluate(n=>getComputedStyle(n).stroke),'none');
  assert.deepEqual(errors,[]);console.log('PASS: NIC hover/keyboard/tap HUD, one-second source updates, measured rates, bounded trends, persistent raw counters, reset/stale/unknown handling, theme screenshots, 320px bounds, forced colors.');
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exit(1)});
