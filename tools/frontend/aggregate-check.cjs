// Synthetic loopback fixture only. No production account or telemetry access.
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const assert=require('node:assert/strict');
const fs=require('node:fs');
(async()=>{
  const browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH,headless:true,args:['--no-sandbox']});
  const page=await browser.newPage({viewport:{width:1440,height:1100},colorScheme:'dark'});
  const errors=[];page.on('pageerror',e=>errors.push(e.message));
  await page.addInitScript(()=>{
    const Native=window.EventSource;window.testStreams=[];
    window.EventSource=class extends Native{constructor(...args){super(...args);window.testStreams.push(this);}};
  });
  await page.goto('http://127.0.0.1:19581');
  await page.getByLabel('Username',{exact:true}).fill('browser-test');
  await page.getByLabel('Password',{exact:true}).fill(process.env.BROWSER_TEST_PASSWORD);
  await page.getByRole('button',{name:'Sign in',exact:true}).click();
  const card=page.getByRole('region',{name:'Traffic (Aggregate)',exact:true});
  await card.locator('.traffic-line.unmapped').first().waitFor();
  assert.equal(await card.locator('svg').count(),1);
  assert.equal(await card.locator('.traffic-line').count(),6);
  const geometry=await card.locator('linearGradient').evaluateAll(nodes=>nodes.map(n=>({y:Number(n.getAttribute('y1')),units:n.getAttribute('gradientUnits'),stops:n.querySelectorAll(':scope > stop').length})));
  assert.deepEqual(geometry.map(g=>g.y),[0,240]);
  assert.ok(geometry.every(g=>g.units==='userSpaceOnUse'&&g.stops===7));
  for(const direction of ['download','upload']){
    const path=await card.locator(`.traffic-line.actual.${direction}-line`).getAttribute('d');
    const ys=[...path.matchAll(/[ML][\d.]+,([\d.]+)/g)].map(m=>Number(m[1]));
    assert.ok(ys.length>2);
    assert.ok(ys.every(y=>direction==='download'?y<=120:y>=120));
  }
  assert.ok((await card.locator('.unmapped.download-line').getAttribute('d')).split('M').length>=3,'unmapped gap stays visible');
  let path=await card.locator('.actual.download-line').getAttribute('d');
  for(let i=0;i<3;i++){
    await page.waitForFunction(old=>document.querySelector('.actual.download-line')?.getAttribute('d')!==old,path,{timeout:3500});
    path=await card.locator('.actual.download-line').getAttribute('d');
  }
  const mapped=card.getByRole('button',{name:/^Mapped/});
  await mapped.click();
  await page.waitForTimeout(1200);
  assert.equal(await mapped.getAttribute('aria-pressed'),'false');
  assert.equal(await card.locator('.mapped.download-line').getAttribute('d'),'');
  await mapped.click();
  const chart=card.locator('svg');
  const bounds=await chart.boundingBox();
  await page.mouse.move(bounds.x+bounds.width/2,bounds.y+bounds.height/4);
  await page.waitForFunction(()=>document.querySelector('.traffic-inspector>p')?.textContent.startsWith('Inspect'));
  await page.mouse.click(bounds.x+bounds.width/2,bounds.y+bounds.height/4);
  const pinned=await card.locator('.traffic-inspector>p').innerText();
  await page.waitForTimeout(1200);
  assert.equal(await card.locator('.traffic-inspector>p').innerText(),pinned,'live refresh must preserve selected timestamp');
  await chart.focus();
  await page.keyboard.press('ArrowLeft');
  assert.notEqual(await card.locator('.traffic-inspector>p').innerText(),pinned);
  await page.keyboard.press('Escape');
  assert.match(await card.locator('.traffic-inspector>p').innerText(),/^Latest/);

  const out=process.env.AGGREGATE_EVIDENCE_DIR||'/tmp/lqosync-aggregate-evidence';fs.mkdirSync(out,{recursive:true});
  for(const theme of ['dark','light']){
    await page.getByLabel('Color theme').selectOption(theme);
    await page.mouse.move(0,0);
    await page.evaluate(()=>document.activeElement.blur());
    await card.screenshot({path:`${out}/${theme}.png`});
  }
  for(const width of [768,320]){
    await page.setViewportSize({width,height:1400});
    assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),`overflow at ${width}`);
    assert.ok(await card.evaluate(n=>n.scrollWidth<=n.clientWidth));
    if(width===320)await card.screenshot({path:`${out}/mobile.png`});
  }
  await page.setViewportSize({width:1440,height:1100});
  const touchBounds=await chart.boundingBox();
  await chart.dispatchEvent('pointerdown',{pointerType:'touch',clientX:touchBounds.x+touchBounds.width/2,clientY:touchBounds.y+touchBounds.height/2});
  assert.match(await card.locator('.traffic-inspector>p').innerText(),/^Pinned/);
  await card.getByRole('button',{name:'Live',exact:true}).click();
  await page.evaluate(async()=>{
    window.aggregateView=await(await fetch('/api/v1/overview')).json();
    window.testStreams.forEach(s=>s.close());
    const view=window.aggregateView;const at=view.traffic.at(-1).at;
    const p=(at,d,u,gap=false)=>({at,down_mbps:d,up_mbps:u,gap_before:gap});
    view.traffic=[p(at-2,90,30),p(at-1,100,40),p(at,110,50)];
    view.classification={mapped:{available:true,stale:false,points:[p(at-2,80,20),p(at,95,40)]},unmapped:{available:true,stale:false,points:[p(at-2,0,0),p(at,3,.2,true)]},history_capped:false};
    window.testStreams.at(-1).dispatchEvent(new MessageEvent('message',{data:JSON.stringify(view)}));
  });
  await page.route('**/api/v1/overview',async route=>route.fulfill({status:200,contentType:'application/json',body:JSON.stringify(await page.evaluate(()=>window.aggregateView))}));
  await card.getByRole('button',{name:'Live',exact:true}).click();
  await chart.focus();
  await page.keyboard.press('ArrowLeft');
  const unmapped=card.locator('.traffic-inspector tbody tr.unmapped');
  assert.deepEqual(await unmapped.locator('td').allTextContents(),['—','—'],'missing timestamp must not borrow a neighbor or become zero');
  await page.keyboard.press('ArrowLeft');
  assert.deepEqual(await unmapped.locator('td').allTextContents(),['0 bps','0 bps']);
  await page.keyboard.press('Escape');
  await page.evaluate(()=>{
    const view=window.aggregateView;view.classification.unmapped.stale=true;
    view.traffic.at(-1).down_mbps=220;
    window.testStreams.at(-1).dispatchEvent(new MessageEvent('message',{data:JSON.stringify(view)}));
  });
  await page.waitForFunction(()=>Number(document.querySelector('.chart-frame linearGradient')?.getAttribute('y1'))>0);
  assert.match(await card.locator('.capacity-note').innerText(),/110.0%/);
  await unmapped.getByText('Stale / gap',{exact:true}).waitFor();
  await page.evaluate(()=>{
    const view=window.aggregateView;view.wan_capacity=null;view.classification=null;
    window.testStreams.at(-1).dispatchEvent(new MessageEvent('message',{data:JSON.stringify(view)}));
  });
  await card.getByText('WAN capacity unavailable.',{exact:false}).waitFor();
  assert.equal(await card.locator('.chart-frame.rainbow').count(),0);
  await unmapped.getByText('Unavailable',{exact:true}).waitFor();
  await page.emulateMedia({forcedColors:'active'});
  assert.equal(await card.locator('.traffic-area').first().evaluate(n=>getComputedStyle(n).display),'none');
  await page.emulateMedia({forcedColors:'none',reducedMotion:'reduce'});
  assert.deepEqual(errors,[]);
  await browser.close();
  console.log('PASS: three mirrored sources, shared domain, capacity gradients, gaps, one-second updates, persistent toggles/pin, keyboard/touch, exact inspection, zero/unknown/stale, overflow, themes, 768/320px, forced colors.');
})().catch(e=>{console.error(e);process.exit(1);});
