// Synthetic loopback fixture only. F0 checks real UI behavior, never production routers.
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const assert=require('node:assert/strict');
const fs=require('node:fs');
(async()=>{
  const browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH,headless:true,args:['--no-sandbox']});
  const out=process.env.ROUTERS_EVIDENCE_DIR||'/tmp/lqosync-studio-shell';fs.mkdirSync(out,{recursive:true});
  let page;
  try{
    page=await browser.newPage({viewport:{width:1440,height:1000},colorScheme:'light'});
    const errors=[],writes=[];
    page.on('pageerror',e=>errors.push(e.message));
    page.on('request',r=>{if(r.url().includes('/api/v1/')&&r.method()!=='GET'&&!r.url().endsWith('/session'))writes.push({url:r.url(),body:r.postData()})});
    await page.goto('http://127.0.0.1:19581');
    await page.getByLabel('Username',{exact:true}).fill('browser-test');
    await page.getByLabel('Password',{exact:true}).fill(process.env.BROWSER_TEST_PASSWORD);
    await page.getByRole('button',{name:'Sign in',exact:true}).click();
    await page.getByRole('navigation',{name:'Main navigation'}).getByRole('link',{name:'Topology',exact:true}).click();
    const frame=page.locator('.canvas-frame'),canvas=page.locator('.canvas-viewport');
    await canvas.locator('.canvas-node').nth(3).waitFor();
    const dock=page.getByRole('toolbar',{name:'Topology tools'});
    // Missing help, moved docks, or a viewport-sized modal would break these real interactions.
    const help=dock.getByRole('button',{name:'Configuration information',exact:true});
    await help.waitFor({timeout:5000});
    const inside=async(locator,label)=>{
      const outer=await frame.boundingBox(),inner=await locator.boundingBox();
      assert.ok(outer&&inner,`${label} has a visible box`);
      assert.ok(inner.x>=outer.x-1&&inner.y>=outer.y-1&&inner.x+inner.width<=outer.x+outer.width+1&&inner.y+inner.height<=outer.y+outer.height+1,`${label} stays inside canvas: ${JSON.stringify({outer,inner})}`);
    };
    async function tools_stay_inside_canvas(){
      for(const controls of ['.canvas-dock','.canvas-zoom','.canvas-appearance','.topology-head','.canvas-evidence'])await inside(page.locator(controls),controls);
      for(const tool of await frame.locator('.canvas-dock button,.canvas-zoom button,.canvas-appearance button').all()){
        const box=await tool.boundingBox();if(!box)continue;
        assert.ok(box.width>=44&&box.height>=44,'canvas tool has a 44 px target');
        assert.ok(await tool.getAttribute('aria-label'),'icon action has an accessible name');
        assert.equal(await tool.locator('svg').count(),1,'action uses an icon');
      }
    }
    async function card_data_survives_shell_change(){
      assert.deepEqual(await canvas.locator('.canvas-node strong').allInnerTexts(),['WAN North','PPPoE access','WAN South','DHCP access']);
      assert.match(await canvas.getByRole('button',{name:/^PPPoE access,/}).innerText(),/600 \/ 300 Mbps\s*5 subscribers/);
      assert.equal(await canvas.locator('path.sankey-ribbon').count(),6,'capacity streams into the groups and the service tiers');
      const card=await canvas.getByRole('button',{name:/^WAN North,/}).boundingBox();
      const heading=await page.locator('.topology-head').boundingBox();
      const evidence=await page.locator('.canvas-evidence').boundingBox();
      for(const overlay of [heading,evidence])assert.ok(card.y>=overlay.y+overlay.height,'viewport cards stay below fixed navigation and evidence');
    }
    async function modal_stays_inside_canvas(name){
      const dialog=page.getByRole('dialog',{name,exact:true});await dialog.waitFor();await inside(dialog,name);
      const bounds=await dialog.boundingBox();
      assert.ok(bounds.width>200&&bounds.height>120,'dialog retains usable content space');
      assert.equal(await dialog.evaluate(d=>d.scrollWidth<=d.clientWidth+1),true,'modal has no horizontal overflow');
      return dialog;
    }
    for(const width of [1440,768,390]){
      await page.setViewportSize({width,height:1000});
      await tools_stay_inside_canvas();await card_data_survives_shell_change();
      const scroll=await canvas.evaluate(v=>[v.scrollLeft,v.scrollTop]);
      await page.keyboard.press('Tab');await help.focus();
      assert.equal(await help.evaluate(e=>getComputedStyle(e,'::after').opacity),'1','keyboard focus exposes the icon tooltip');
      await page.keyboard.press('Enter');
      const info=await modal_stays_inside_canvas('Configuration workflow');
      assert.ok(await info.locator('ol li').count()>=5,'help shows the source configuration sequence');
      assert.match(await info.innerText(),/optional/i);
      if(width===1440){
        for(const resized of [768,390,1440]){
          await page.setViewportSize({width:resized,height:1000});
          await inside(info,'open help after resize');
        }
      }
      await page.screenshot({path:`${out}/studio-help-${width}.png`});
      await page.keyboard.press('Escape');
      assert.equal(await help.evaluate(e=>e===document.activeElement),true,'help returns focus to its icon');
      assert.deepEqual(await canvas.evaluate(v=>[v.scrollLeft,v.scrollTop]),scroll,'help preserves viewport');
      assert.equal(await page.getByRole('heading',{name:'Configuration workflow',exact:true}).count(),0,'workflow is absent while help is closed');
      assert.equal(writes.length,0,'help and canvas inspection send no mutation');
    }
    // Opening a card, using zoom, or switching themes must retain backend card data.
    await page.setViewportSize({width:1440,height:1000});
    const north=canvas.getByRole('button',{name:/^WAN North,/});await north.click();await page.keyboard.press('Enter');
    let node=await modal_stays_inside_canvas('WAN North');
    const name=node.getByLabel('Name',{exact:true});await name.fill('');await name.blur();
    await page.keyboard.press('Escape');
    await help.click();await modal_stays_inside_canvas('Configuration workflow');await page.keyboard.press('Escape');
    await north.focus();await page.keyboard.press('Enter');
    node=await modal_stays_inside_canvas('WAN North');assert.equal(await node.getByLabel('Name',{exact:true}).inputValue(),'','invalid card draft survives help');
    await page.keyboard.press('Escape');
    const appearance=page.getByRole('button',{name:'Canvas appearance',exact:true});
    for(const mode of ['light','dark','system']){
      await appearance.click();
      await page.locator('.canvas-appearance').getByRole('button',{name:`${mode[0].toUpperCase()+mode.slice(1)} theme`,exact:true}).click();
      assert.equal(await page.locator('html').getAttribute('data-theme'),mode);
      assert.equal(await page.locator('.main-top').getByLabel('Color theme',{exact:true}).count(),0,'Builder keeps appearance inside the canvas');
      await card_data_survives_shell_change();
      await page.screenshot({path:`${out}/studio-canvas-${mode}.png`});
    }
    await page.emulateMedia({colorScheme:'dark'});
    const dark=await frame.evaluate(e=>getComputedStyle(e).backgroundColor);
    await page.emulateMedia({colorScheme:'light'});
    assert.notEqual(await frame.evaluate(e=>getComputedStyle(e).backgroundColor),dark,'System follows runtime browser preference');
    assert.equal(writes.length,0,'appearance, help, and invalid drafts never mutate policy');
    await page.getByRole('button',{name:'Zoom in',exact:true}).click();
    await tools_stay_inside_canvas();
    assert.notEqual(await page.locator('.canvas-layer').evaluate(e=>getComputedStyle(e).transform),'matrix(1, 0, 0, 1, 0, 0)');
    assert.equal(writes.length,0,'zoom changes only the viewport');
    // A locked writer and a failed cache refresh keep all controls inside the same frame.
    await page.route('**/api/v1/topology',async route=>{
      const response=await route.fetch();const data=await response.json();data.writer='unknown';
      await route.fulfill({response,json:data});
    });
    await page.getByRole('button',{name:'Reload view',exact:true}).click();
    const locked=page.getByRole('note').filter({hasText:'The shaping writer is unknown.'});await locked.waitFor();
    await tools_stay_inside_canvas();await inside(locked,'read-only reason');
    await page.unroute('**/api/v1/topology');
    await page.route('**/api/v1/topology',route=>route.fulfill({status:503,body:''}));
    await page.getByRole('button',{name:'Reload view',exact:true}).click();
    const cacheError=page.getByRole('status').filter({hasText:'The topology view is not available yet.'});await cacheError.waitFor();
    await tools_stay_inside_canvas();await inside(cacheError,'cache refresh error');
    const errorBox=await cacheError.boundingBox(),reasonBox=await locked.boundingBox();
    assert.ok(errorBox.y+errorBox.height<=reasonBox.y||reasonBox.y+reasonBox.height<=errorBox.y,'cache error and writer reason remain separately readable');
    assert.equal(writes.length,0,'read-only and failed refresh never mutate policy');
    assert.deepEqual(errors,[]);
    console.log('PASS: tools_stay_inside_canvas; modal_stays_inside_canvas; card_data_survives_shell_change; optional help/focus/draft retention; shared dynamic themes; no mutation from view controls.');
  }catch(e){if(page)await page.screenshot({path:`${out}/studio-failure.png`,fullPage:true}).catch(()=>{});throw e}
  finally{await browser.close()}
})().catch(e=>{console.error(e);process.exit(1)});
