// Isolated synthetic fixture only; never read production account credentials.
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const assert=require('node:assert/strict');
const fs=require('node:fs');
(async()=>{
 const browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH,headless:true,args:['--no-sandbox']});
 try{
  const page=await browser.newPage({viewport:{width:1440,height:1100},colorScheme:'dark'});
  const errors=[];page.on('pageerror',e=>errors.push(e.message));
  await page.addInitScript(()=>{const Native=window.EventSource;window.testStreams=[];window.EventSource=class extends Native{constructor(...args){super(...args);window.testStreams.push(this);}};});
  await page.goto('http://127.0.0.1:19581');
  await page.getByLabel('Username',{exact:true}).fill('browser-test');
  await page.getByLabel('Password',{exact:true}).fill(process.env.BROWSER_TEST_PASSWORD);
  await page.getByRole('button',{name:'Sign in',exact:true}).click();
  const nic=page.locator('.dpdk-nic');await nic.getByText('2 / 2 UP',{exact:true}).waitFor();
  const summary=page.getByRole('region',{name:'Service summary',exact:true});
  assert.equal(await summary.locator('.dpdk-nic').count(),1,'NIC belongs with the service summary cards');
  assert.equal(await page.locator('.dpdk-panel .dpdk-nic').count(),0,'diagnostics must not duplicate the faceplate');
  assert.equal(await summary.getByRole('heading',{name:'LibreQoS',exact:true}).count(),1);
  const summaryBox=await summary.boundingBox();const chartBox=await page.locator('.overview-grid').boundingBox();
  assert.ok(summaryBox.y+summaryBox.height<=chartBox.y,'NIC summary must precede the traffic charts');
  const ports=nic.locator('.nic-physical-port');assert.equal(await ports.count(),2);
  assert.deepEqual(await ports.locator('figcaption strong').allTextContents(),['WAN','LAN']);
  assert.deepEqual(await ports.locator('svg').evaluateAll(ns=>ns.map(n=>n.dataset.connector)),['sfp','rj45']);
  assert.deepEqual(await ports.locator('.nic-pci').allTextContents(),['0000:01:00.0','0000:01:00.1']);
  assert.equal(await page.locator('#lqosync-nic-sfp').count(),1,'sprite IDs must be unique');
  assert.equal(await ports.first().locator('use').getAttribute('href'),'#lqosync-nic-sfp');
  assert.equal(await ports.nth(1).locator('use').getAttribute('href'),'#lqosync-nic-rj45');
  assert.ok((await ports.first().getAttribute('aria-label')).includes('confirmed'));
  assert.ok((await ports.nth(1).getAttribute('aria-label')).includes('inferred'));
  assert.equal(await ports.first().evaluate(n=>getComputedStyle(n).borderTopWidth),'0px');
  const boxes=await ports.evaluateAll(ns=>ns.map(n=>n.getBoundingClientRect().toJSON()));
  assert.ok(boxes[1].x-boxes[0].right<=10,'ports must be adjacent');
  for(const tile of await ports.all()){
   const art=await tile.locator('svg').boundingBox();const leds=await tile.locator('.nic-activity-leds').boundingBox();
   assert.ok(leds.y>=art.y+art.height && leds.y-art.y-art.height<=4,'LEDs must sit directly below artwork');
  }
  await page.evaluate(async()=>{window.nicView=await(await fetch('/api/v1/overview')).json();window.testStreams.forEach(s=>s.close());window.nicEpoch=window.nicView.dpdk.nic.updated;});
  await page.route('**/api/v1/overview',async route=>route.fulfill({status:200,contentType:'application/json',body:JSON.stringify(await page.evaluate(()=>window.nicView))}));
  // The observer records class changes even when a 180-ms pulse ends between test round trips.
  await ports.first().evaluate(n=>{window.nicFirst=n;window.nicPulses=[0,0];new MutationObserver(ms=>{for(const m of ms){const led=m.target;if(m.oldValue?.split(' ').includes('pulsing'))window.nicPulses[led.closest('.rx')?0:1]++;}}).observe(n,{subtree:true,attributes:true,attributeFilter:['class'],attributeOldValue:true});});
  const sample=async(rx,tx,extra={})=>page.evaluate(({rx,tx,extra})=>{
    const v=window.nicView;const d=v.dpdk;window.nicEpoch+=1;d.sampled_at=window.nicEpoch;d.nic.updated=window.nicEpoch;d.nic.stale=false;
    const p=d.ports[0];p.reset=false;p.state='UP';p.activity={sampled_at:window.nicEpoch,interval_ms:10000,rx_packets:rx,tx_packets:tx,rx_bytes:rx===null?null:rx*100,tx_bytes:tx===null?null:tx*100};Object.assign(p,extra);
    window.testStreams.at(-1).dispatchEvent(new MessageEvent('message',{data:JSON.stringify(v)}));
  },{rx,tx,extra});
  await sample(8,0);await page.waitForTimeout(240);
  assert.deepEqual(await page.evaluate(()=>window.nicPulses),[1,0]);
  assert.equal(await ports.first().evaluate(n=>n===window.nicFirst),true,'ordinary updates must retain port DOM');
  await page.evaluate(()=>window.testStreams.at(-1).dispatchEvent(new MessageEvent('message',{data:JSON.stringify(window.nicView)})));
  await page.waitForTimeout(220);assert.deepEqual(await page.evaluate(()=>window.nicPulses),[1,0]);
  await sample(0,9);await page.waitForTimeout(240);assert.deepEqual(await page.evaluate(()=>window.nicPulses),[1,1]);
  await sample(0,0);await page.waitForTimeout(220);assert.deepEqual(await page.evaluate(()=>window.nicPulses),[1,1]);
  await sample(8,8,{state:'DOWN'});await page.waitForTimeout(220);assert.equal(await ports.first().evaluate(n=>n.classList.contains('link-down')),true);assert.deepEqual(await page.evaluate(()=>window.nicPulses),[1,1]);
  await sample(8,8,{reset:true});await page.waitForTimeout(220);assert.deepEqual(await page.evaluate(()=>window.nicPulses),[1,1]);
  await page.evaluate(()=>{const v=window.nicView;v.dpdk.nic.stale=true;window.testStreams.at(-1).dispatchEvent(new MessageEvent('message',{data:JSON.stringify(v)}));});
  await nic.getByText('Link unknown · stale',{exact:true}).waitFor();
  assert.equal(await nic.locator('.nic-physical-port.link-unknown').count(),2);assert.equal(await nic.locator('.pulsing').count(),0);
  await sample(4,7);
  await page.emulateMedia({reducedMotion:'reduce'});
  assert.equal(await ports.first().locator('.rx .nic-activity-led').getAttribute('data-active'),'true');
  assert.equal(await ports.first().locator('.rx .nic-activity-led').evaluate(n=>getComputedStyle(n).animationName),'none');
  const out=process.env.NIC_EVIDENCE_DIR||'/tmp/lqosync-nic-evidence';fs.mkdirSync(out,{recursive:true});
  for(const theme of ['dark','light']){await page.getByLabel('Color theme').selectOption(theme);await summary.screenshot({path:`${out}/${theme}.png`});}
  await page.setViewportSize({width:320,height:1200});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await summary.screenshot({path:`${out}/mobile.png`});
  for(const width of [768,1024,1280,1600,1920]){
   await page.setViewportSize({width,height:1200});
   assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),`dashboard must fit ${width}px`);
   const boxes=await ports.evaluateAll(ns=>ns.map(n=>n.getBoundingClientRect().toJSON()));
   assert.equal(boxes[0].y,boxes[1].y,`ports must stay adjacent at ${width}px`);
   if(width===1920){await page.getByLabel('Color theme').selectOption('dark');await summary.screenshot({path:`${out}/wide.png`});}
   if(width>=1600){const cards=await summary.locator(':scope > article').evaluateAll(ns=>ns.map(n=>n.getBoundingClientRect().top));assert.ok(cards.every(y=>y===cards[0]),'all summary cards share one row on wide screens');}
  }
  // Unknown source metadata must keep a generic connector and an explicit unknown role.
  await page.evaluate(()=>{const v=window.nicView;v.dpdk.ports[0].physical={};window.testStreams.at(-1).dispatchEvent(new MessageEvent('message',{data:JSON.stringify(v)}));});
  await ports.first().getByText('Role unknown',{exact:true}).waitFor();assert.equal(await ports.first().locator('svg').getAttribute('data-connector'),'unknown');
  for(const connector of ['sfp','sfp+','sfp28','qsfp','qsfp+','qsfp28','rj45']){
   await page.evaluate(connector=>{const v=window.nicView;v.dpdk.ports[0].physical.connector=connector;window.testStreams.at(-1).dispatchEvent(new MessageEvent('message',{data:JSON.stringify(v)}));},connector);
   assert.equal(await ports.first().locator('svg').getAttribute('data-connector'),connector.startsWith('qsfp')?'qsfp':connector.startsWith('sfp')?'sfp':'rj45');
  }
  await page.evaluate(()=>{window.nicView.dpdk.ports[0].physical.connector='unknown';window.testStreams.at(-1).dispatchEvent(new MessageEvent('message',{data:JSON.stringify(window.nicView)}));});
  await ports.first().locator('.nic-contacts').waitFor({state:'attached'});
  await ports.first().focus();assert.equal(await ports.first().evaluate(n=>n===document.activeElement),true);
  await page.emulateMedia({forcedColors:'active'});assert.notEqual(await ports.first().locator('.nic-contacts').evaluate(n=>getComputedStyle(n).stroke),'none');
  assert.equal(await page.locator('#lqosync-nic-sfp .nic-kit-shape').first().evaluate(n=>getComputedStyle(n).stroke),await ports.first().locator('.nic-contacts').evaluate(n=>getComputedStyle(n).stroke));
  assert.deepEqual(errors,[]);
  console.log('PASS: adjacent physical ports, role/PCI/confidence, SVG families, LEDs below artwork, independent sample pulses, no replay/idle/down/reset/stale pulses, stable DOM, reduced motion, keyboard, Light/Dark, 320px, generic fallback, forced colors.');
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exit(1)});
