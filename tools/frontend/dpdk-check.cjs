// Isolated browser fixture only. Never use production account credentials.
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const assert=require('node:assert/strict');
const fs=require('node:fs');
(async()=>{
  const browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH,headless:true,args:['--no-sandbox']});
  const page=await browser.newPage({viewport:{width:1440,height:1100},colorScheme:'dark'});
  const errors=[];page.on('pageerror',e=>errors.push(e.message));
  await page.addInitScript(()=>{
    window.testRealNow=Date.now;
    Date.now=()=>window.testRealNow()+3600000; // One-hour client clock skew.
    const Native=window.EventSource;window.testStreams=[];
    window.EventSource=class extends Native{constructor(...args){super(...args);window.testStreams.push(this);}};
  });
  await page.goto('http://127.0.0.1:19581');
  await page.getByLabel('Username',{exact:true}).fill('browser-test');
  await page.getByLabel('Password',{exact:true}).fill(process.env.BROWSER_TEST_PASSWORD);
  await page.getByRole('button',{name:'Sign in',exact:true}).click();
  const nic=page.locator('.nic-summary');
  await nic.getByText('0000:01:00.0',{exact:true}).first().waitFor();
  // One DPDK Health card: worker ring, health band, worker telemetry, and the physical NIC ports.
  const ring=page.locator('.worker-ring-card');
  assert.equal(await ring.locator('.nic-summary').count(),1,'the NIC ports live in the DPDK Health card');
  assert.equal(await page.getByRole('region',{name:'DPDK stats & health',exact:true}).count(),0,'the old DPDK card is gone');
  assert.equal(await page.locator('details.diagnostics').count(),0,'the static diagnostics are gone');
  assert.match(await ring.locator('.worker-chip-busy').textContent(),/75.0%/);
  assert.match(await ring.locator('.panel-note').innerText(),/mean busy 50.0%/);
  assert.equal(await ring.locator('.worker-sector').count(),9);
  assert.equal(await ring.locator('.phase-tick').count(),9);
  assert.equal(await ring.locator('h2').innerText(),'DPDK Health');
  assert.equal(await ring.locator('.health-band').count(),9);
  assert.match(await ring.locator('.health-line').innerText(),/^Bridge ok · /);
  // Keyboard focus: the bounding-box centre of an arc is not on its stroke, so a pointer hover would miss it.
  await ring.locator('#health-band-4').focus();
  assert.match(await ring.locator('.worker-focus').innerText(),/^Buffer pool · Free /);
  await page.keyboard.press('Escape');await ring.locator('#health-band-4').blur();
  const out=process.env.DPDK_EVIDENCE_DIR||'/tmp/lqosync-dpdk-evidence';fs.mkdirSync(out,{recursive:true});
  const picture=async path=>{
    await page.evaluate(()=>{document.activeElement.blur();window.scrollTo(0,0);});
    await ring.screenshot({path});
  };
  for(const theme of ['dark','light']){
    await page.getByLabel('Color theme').selectOption(theme);
    await page.evaluate(()=>document.activeElement.blur());
    await picture(`${out}/${theme}.png`);
  }
  for(const width of [768,320]){
    await page.setViewportSize({width,height:1000});
    assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
    assert.ok(await ring.evaluate(n=>n.scrollWidth<=n.clientWidth),'card must fit narrow screens');
    if(width===320)await picture(`${out}/mobile.png`);
  }
  await page.setViewportSize({width:1440,height:1100});
  await page.emulateMedia({reducedMotion:'reduce',forcedColors:'active'});
  assert.equal(await ring.locator('.phase-value').first().evaluate(n=>getComputedStyle(n).animationName),'none');
  assert.equal(await ring.locator('.health-value').first().evaluate(n=>getComputedStyle(n).animationName),'none');
  await page.emulateMedia({reducedMotion:'no-preference',forcedColors:'none'});
  // Stop the fixture stream. Keep fallback reads consistent with each synthetic update.
  await page.evaluate(async()=>{
    window.testStreams.forEach(s=>s.close());
    window.dpdkView=await(await fetch('/api/v1/overview')).json();
  });
  await page.route('**/api/v1/overview',async route=>{
    const view=await page.evaluate(()=>window.dpdkView);
    await route.fulfill({status:200,contentType:'application/json',body:JSON.stringify(view)});
  });
  const pulses=await ring.locator('.phase-pulse').evaluateAll(ns=>ns.map(n=>n.getAnimations().length));
  await page.evaluate(()=>{
    const view=window.dpdkView;const at=Math.floor(window.testRealNow()/1000);
    view.dpdk.sampled_at=at;
    view.dpdk.analytics.updated=at;
    view.dpdk.mean_busy_ppm=900000;
    view.dpdk.workers[0].busy_ppm=950000;
    for(const w of view.dpdk.workers)for(const p of w.phases)p.cycles+=1000;
    window.testStreams.at(-1).dispatchEvent(new MessageEvent('message',{data:JSON.stringify(view)}));
  });
  await page.waitForFunction(()=>document.querySelector('.worker-ring-card .panel-note').textContent.includes('mean busy 90.0%'));
  assert.match(await ring.locator('.worker-chip-busy').textContent(),/95.0%/);
  // New cycles recreate the phase ticks, so the pulse runs again.
  const fresh=await ring.locator('.phase-pulse').evaluateAll(ns=>ns.filter(n=>n.getAnimations().some(a=>a.playState==='running')).length);
  assert.ok(fresh>0,`a new sample must pulse the phase ring (before ${pulses})`);
  await page.evaluate(()=>{
    const view=window.dpdkView;const at=Math.floor(window.testRealNow()/1000);
    view.dpdk.sampled_at=at;
    view.dpdk.analytics.updated=at-20;view.dpdk.analytics.stale=false;
    view.dpdk.health.updated=at;view.dpdk.health.state='degraded';view.dpdk.console.status='degraded';
    view.dpdk.nic.updated=at;view.dpdk.ports[0].state='down';
    view.dpdk.ports[0].name='long_port_name_'.repeat(8);
    view.dpdk.ports[0].reset=true;
    window.testStreams.at(-1).dispatchEvent(new MessageEvent('message',{data:JSON.stringify(view)}));
  });
  await ring.locator('.health-line').getByText(/^Bridge degraded · /).waitFor();
  assert.equal(await ring.locator('#health-band-0').getAttribute('aria-label'),'Bridge status: degraded');
  await nic.locator('.nic-physical-port.link-down').first().waitFor();
  assert.equal(await ring.locator('.worker-ring-head .chip').innerText(),'Stale · last known');
  assert.match(await nic.textContent(),/Counter reset/);
  assert.ok(await ring.evaluate(n=>n.scrollWidth<=n.clientWidth),'long port names must wrap');
  await page.evaluate(()=>{
    const view=window.dpdkView;const at=Math.floor(window.testRealNow()/1000);
    view.dpdk.sampled_at=at;
    view.dpdk.health.updated=at-30;view.dpdk.nic.updated=at-30;
    window.testStreams.at(-1).dispatchEvent(new MessageEvent('message',{data:JSON.stringify(view)}));
  });
  await ring.locator('.health-line').getByText(/^Unknown health · last known degraded/).waitFor();
  assert.ok((await ring.locator('#health-band-0').getAttribute('class')).includes('stale'));
  await nic.getByText('Link unknown · stale',{exact:true}).waitFor();
  await page.evaluate(()=>{
    const view=window.dpdkView;const at=Math.floor(window.testRealNow()/1000);
    view.dpdk.sampled_at=at;
    for(const key of ['analytics','health','nic']){view.dpdk[key].updated=at;view.dpdk[key].stale=false;}
    view.dpdk.health.state='healthy';view.dpdk.console.status='healthy';
    window.testStreams.at(-1).dispatchEvent(new MessageEvent('message',{data:JSON.stringify(view)}));
  });
  await ring.locator('.health-line').getByText(/^Bridge healthy · /).waitFor();
  assert.equal(await ring.locator('.worker-ring-head .chip').innerText(),'Live');
  // Without the stream, fallback reads repeat one snapshot: the bridge source ages out to unknown.
  await page.evaluate(()=>window.testStreams.forEach(s=>s.dispatchEvent(new Event('error'))));
  await ring.locator('.health-line').getByText(/^Unknown health/).waitFor();
  await page.evaluate(()=>{
    window.dpdkView.dpdk=null;
    window.testStreams.at(-1).dispatchEvent(new MessageEvent('message',{data:JSON.stringify(window.dpdkView)}));
  });
  await ring.getByText('Worker telemetry unavailable',{exact:true}).waitFor();
  assert.equal(await ring.locator('.worker-chip-busy').textContent(),'—');
  assert.equal(await ring.locator('.health-line').innerText(),'Bridge health unavailable');
  assert.deepEqual(errors,[]);
  await browser.close();
  console.log('PASS: one DPDK Health card with one-hour client clock skew: worker ring and mean, phase and health pulses, health band focus, NIC ports in the card, independent stale/degraded/link states, reset label, missing data, Light/Dark, reduced motion, forced colors, and 768/320px layouts.');
})().catch(e=>{console.error(e);process.exit(1);});
