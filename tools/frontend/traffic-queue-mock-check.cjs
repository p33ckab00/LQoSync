// Synthetic, local design reference only. No production service or account access.
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
const {pathToFileURL}=require('node:url');
const root=path.resolve(__dirname,'../..');
const prototype=path.join(root,'docs/frontend/prototypes/traffic-queue-dynamics-v5.html');
const evidence=path.join(root,'docs/frontend/evidence/traffic-queue-v5');
const images=path.join(root,'docs/frontend/images');
(async()=>{
  fs.mkdirSync(evidence,{recursive:true});
  const browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH,headless:true,args:['--no-sandbox']});
  try {
    const page=await browser.newPage({viewport:{width:1440,height:1100},colorScheme:'dark'});
    const errors=[],requests=[];
    page.on('pageerror',e=>errors.push(e.message));
    page.on('request',r=>{if(/^https?:/.test(r.url()))requests.push(r.url());});
    await page.goto(pathToFileURL(prototype).href);
    await page.locator('[data-series=down]').waitFor();
    const fill=()=>page.locator('#chart > rect').getAttribute('fill');
    await page.getByLabel('Color theme').selectOption('system');
    assert.equal(await fill(),'#17191b');
    await page.emulateMedia({colorScheme:'light'});
    await page.waitForFunction(()=>document.querySelector('#chart > rect').getAttribute('fill')==='#fff');
    await page.getByLabel('Color theme').selectOption('dark');
    await page.emulateMedia({colorScheme:'light'});
    assert.equal(await fill(),'#17191b','explicit setting overrides the OS');
    await page.reload();
    assert.equal(await page.getByLabel('Color theme').inputValue(),'dark');
    await page.locator('[data-series=down]').waitFor();
    await page.getByRole('button',{name:'Unmapped',exact:true}).click();
    assert.equal(await page.locator('[data-series=unDown]').count(),0);
    assert.match(await page.locator('#readings').textContent(),/Unmapped/,'hidden paths keep inspector data');
    await page.getByRole('button',{name:'Unmapped',exact:true}).click();
    assert.equal(await page.getByLabel('Scope',{exact:true}).count(),0);
    assert.match(await page.locator('#chart-title').textContent(),/DPDK traffic/);
    assert.equal(await page.locator('[data-series=access]').count(),0);
    assert.doesNotMatch(await page.locator('main').textContent(),/subscriber|DEMO-042|Access \/ Internet RTT/i);
    const chart=page.locator('#chart');
    await chart.focus();await page.keyboard.press('Home');
    assert.equal(await page.locator('#inspection-heading').textContent(),'Pinned sample');
    const pinned=await page.locator('#stamp').textContent();
    await page.getByRole('button',{name:'Play demo',exact:true}).click();
    const old=await page.locator('[data-series=down]').getAttribute('d');
    await page.waitForFunction(d=>document.querySelector('[data-series=down]').getAttribute('d')!==d,old);
    assert.equal(await page.locator('#stamp').textContent(),pinned,'timestamp stays pinned after a sample append');
    assert.match(await page.locator('#quality').textContent(),/Outside retained window/);
    await page.getByRole('button',{name:'Pause demo',exact:true}).click();
    await chart.focus();await page.keyboard.press('End');await page.keyboard.press('ArrowLeft');
    assert.notEqual(await page.locator('#stamp').textContent(),pinned);
    await page.keyboard.press('Escape');
    assert.equal(await page.locator('#inspection-heading').textContent(),'Latest sample');
    const rect=await chart.boundingBox();
    await page.mouse.click(rect.x+rect.width*.5,rect.y+200);
    const tapStamp=await page.locator('#stamp').textContent();
    await page.mouse.move(rect.x+rect.width*.8,rect.y+400);
    assert.equal(await page.locator('#stamp').textContent(),tapStamp,'click pins all lanes');
    await page.getByRole('button',{name:'Latest sample',exact:true}).click();
    await page.getByLabel('Source state').selectOption('gap');
    const gaps=await page.locator('[data-series=down]').getAttribute('d');
    assert.equal((gaps.match(/M/g)||[]).length,2,'missing samples split the path');
    await page.getByLabel('Source state').selectOption('reset');
    assert.match(await page.locator('#quality').textContent(),/Reset/);
    assert.match(await page.locator('#readings').textContent(),/Unavailable \/ Unavailable/);
    await page.getByLabel('Source state').selectOption('capacity');
    assert.match(await chart.textContent(),/Capacity unavailable/);
    assert.equal(await chart.locator('path[fill="url(#down-fill)"]').count(),0,'unknown capacity has no capacity color fill');
    await page.getByLabel('Source state').selectOption('unavailable');
    assert.equal(await page.locator('[data-series=down]').getAttribute('d'),'');
    await page.getByLabel('Source state').selectOption('stale');
    assert.match(await page.locator('#quality').textContent(),/Stale/);
    await page.getByLabel('Source state').selectOption('observed');
    await page.getByText('Sample data table',{exact:true}).click();
    await page.locator('#samples tr').first().waitFor();
    assert.equal(await page.locator('#samples tr').count(),61);
    assert.equal(await page.locator('#columns').getByText('Access ms',{exact:true}).count(),0);
    await page.getByText('Sample data table',{exact:true}).click();
    await page.emulateMedia({reducedMotion:'reduce'});
    for(const width of [1440,768,390,320]){
      await page.setViewportSize({width,height:1100});
      await page.waitForFunction(()=>Math.abs(document.querySelector('#chart').viewBox.baseVal.width-document.querySelector('#chart').getBoundingClientRect().width)<2);
      assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),`page overflow at ${width}`);
      const clipped=await chart.evaluate(svg=>[...svg.querySelectorAll('text')].filter(t=>{const b=t.getBBox();return b.x<0||b.x+b.width>svg.viewBox.baseVal.width+1;}).map(t=>t.textContent));
      assert.deepEqual(clipped,[],`clipped SVG labels at ${width}`);
    }
    // Return the deterministic sample set before exporting the coding references.
    await page.reload();await page.locator('[data-series=down]').waitFor();
    const exports=[
      ['traffic-queue-dynamics-v5','dark',1440],
      ['traffic-queue-dynamics-v5-light','light',1440],
      ['traffic-queue-dynamics-v5-mobile','dark',390],
    ];
    for(const [name,theme,width] of exports){
      await page.setViewportSize({width,height:1100});
      await page.getByLabel('Color theme').selectOption(theme);
      await page.waitForFunction(()=>Math.abs(document.querySelector('#chart').viewBox.baseVal.width-document.querySelector('#chart').getBoundingClientRect().width)<2);
      const lowContrast=await page.evaluate(()=>{
        const c=getComputedStyle(document.documentElement);
        const luminance=hex=>{hex=hex.slice(1);if(hex.length===3)hex=[...hex].map(x=>x+x).join('');return [0,2,4].map(i=>parseInt(hex.slice(i,i+2),16)/255).map(v=>v<=.04045?v/12.92:((v+.055)/1.055)**2.4).reduce((s,v,i)=>s+v*[.2126,.7152,.0722][i],0);};
        const bg=luminance(c.getPropertyValue('--surface').trim());
        return ['text','muted','accent','good','warn','violet','bad'].filter(k=>{const fg=luminance(c.getPropertyValue('--'+k).trim());return (Math.max(bg,fg)+.05)/(Math.min(bg,fg)+.05)<4.5;});
      });
      assert.deepEqual(lowContrast,[],`${theme} text and metric colors meet 4.5:1 against the panel`);
      const svg=await chart.evaluate(el=>{const clone=el.cloneNode(true);clone.removeAttribute('tabindex');clone.setAttribute('role','img');clone.setAttribute('width',clone.viewBox.baseVal.width);clone.setAttribute('height','690');return new XMLSerializer().serializeToString(clone);});
      fs.writeFileSync(path.join(images,name+'.svg'),svg+'\n');
      await page.screenshot({path:path.join(evidence,name+'.png'),fullPage:true});
    }
    const touch=await browser.newPage({viewport:{width:390,height:1100},hasTouch:true,isMobile:true});
    touch.on('pageerror',e=>errors.push(e.message));
    await touch.goto(pathToFileURL(prototype).href);
    const touchChart=touch.locator('#chart');await touchChart.waitFor();
    await touchChart.tap({position:{x:180,y:210}});
    assert.equal(await touch.locator('#inspection-heading').textContent(),'Pinned sample');
    await touch.getByRole('button',{name:'Latest sample',exact:true}).tap();
    assert.equal(await touch.locator('#inspection-heading').textContent(),'Latest sample');
    await touch.close();
    assert.deepEqual(errors,[]);assert.deepEqual(requests,[],'the prototype must not contact a live service');
    console.log('PASS: themes, DPDK-only scope, paths, pinned timestamps, streaming, keyboard, touch, gaps, reset, unavailable/stale sources, capacity, table, contrast, 4 widths; 3 SVG/PNG exports; no page errors or network requests.');
  } finally {await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
