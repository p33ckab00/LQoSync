// Synthetic loopback fixture only. No production account, router, or shaping write.
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const assert=require('node:assert/strict');
const fs=require('node:fs');
(async()=>{
  const browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH,headless:true,args:['--no-sandbox']});
  const out=process.env.ROUTERS_EVIDENCE_DIR||'/tmp/lqosync-topology-workflow';fs.mkdirSync(out,{recursive:true});
  try{
    const page=await browser.newPage({viewport:{width:1440,height:1050},colorScheme:'dark'});
    const errors=[],writes=[];page.on('pageerror',e=>errors.push(e.message));
    page.on('request',r=>{if(r.url().includes('/api/v1/topology')&&r.method()!=='GET')writes.push(r.url());});
    await page.route('**/api/v1/topology/operations/*',async route=>{
      const response=await route.fetch();
      if(response.ok()){
        const op=await response.json();
        if(op.discovery?.routers?.length)op.discovery.routers.push({router_id:'wan-only-empty',name:'WAN-only empty router',state:'ok',error:null,candidates:[]});
        await route.fulfill({response,json:op});
      }else await route.fulfill({response});
    });
    await page.goto('http://127.0.0.1:19581');
    await page.getByLabel('Username',{exact:true}).fill('browser-test');
    await page.getByLabel('Password',{exact:true}).fill(process.env.BROWSER_TEST_PASSWORD);
    await page.getByRole('button',{name:'Sign in',exact:true}).click();
    await page.getByRole('navigation',{name:'Main navigation'}).getByRole('link',{name:'Topology',exact:true}).click();
    const tree=page.getByRole('list',{name:'Topology nodes',exact:true});await tree.locator('.tree-node').nth(3).waitFor();
    assert.equal(await tree.locator('ul').count(),2,'hierarchy has semantic nested child lists');
    const north=tree.locator('.tree-node').filter({has:page.getByText('WAN North',{exact:true})});
    const ppp=tree.locator('.tree-node').filter({has:page.getByText('PPPoE access',{exact:true})});
    await north.click();
    await page.getByRole('button',{name:'Toggle WAN North branch',exact:true}).click();
    assert.equal(await ppp.isVisible(),false);
    await page.getByRole('button',{name:'Reload view',exact:true}).click();
    assert.equal(await north.getAttribute('aria-pressed'),'true');
    assert.equal(await ppp.isVisible(),false,'reload preserves branch disclosure');
    await page.getByLabel('Find a node',{exact:true}).fill('PPPoE');
    await ppp.waitFor();assert.equal(await north.isVisible(),true,'search keeps the ancestor');
    assert.equal(await tree.locator('.tree-node').filter({has:page.getByText('WAN South',{exact:true})}).isVisible(),false);
    await page.getByLabel('Find a node',{exact:true}).fill('');
    assert.equal(await ppp.isVisible(),false,'clearing search restores prior disclosure');
    await page.getByRole('button',{name:'Expand all',exact:true}).click();await ppp.click();
    const details=page.getByRole('region',{name:'Details',exact:true});
    assert.match(await details.innerText(),/Parent\s+WAN North/,'details resolve the parent name');
    await north.click();await details.getByRole('button',{name:'Add child',exact:true}).click();
    const child=page.getByRole('dialog',{name:'Create child node',exact:true});await child.waitFor();
    assert.equal(await child.getByLabel('Parent',{exact:true}).inputValue(),'tm-node-wan-north');
    await child.getByLabel('Name',{exact:true}).fill('Access distribution');
    const refresh=page.waitForResponse(r=>r.url().endsWith('/api/v1/topology')&&r.request().method()==='GET');
    await refresh;
    assert.equal(await child.getByLabel('Name',{exact:true}).inputValue(),'Access distribution');
    assert.equal(await page.evaluate(()=>document.activeElement.id),'change-name','cache polling preserves draft focus');
    await page.keyboard.press('Escape');await child.waitFor({state:'hidden'});
    await page.getByRole('button',{name:'Attach PPPoE access',exact:true}).click();
    const attach=page.getByRole('dialog',{name:'Attach source PPPoE access',exact:true});await attach.waitFor();
    assert.equal(await attach.getByLabel('Parent',{exact:true}).inputValue(),'tm-node-wan-north');
    await page.keyboard.press('Escape');await attach.waitFor({state:'hidden'});
    assert.equal(writes.length,0,'selection, drafts and polling issue no writes or router reads');
    await page.getByRole('button',{name:'Discover WAN interfaces',exact:true}).click();
    const discovery=page.getByRole('list',{name:'WAN interfaces',exact:true});
    await discovery.getByText('WAN-only empty router',{exact:true}).waitFor();
    assert.match(await discovery.innerText(),/WAN-only empty router[\s\S]*Read complete[\s\S]*0 interfaces/);
    assert.match(await discovery.innerText(),/Access-02[\s\S]*Read failed/);
    assert.equal(writes.length,1,'one explicit WAN discovery request');
    for(const [theme,width] of [['dark',1440],['light',1440],['dark',1024],['light',768],['dark',390],['light',320]]){
      await page.getByLabel('Color theme').selectOption(theme);await page.setViewportSize({width,height:1050});
      const inventory=page.getByRole('button',{name:'Toggle source inventory',exact:true});
      if(width<=900&&await inventory.getAttribute('aria-expanded')==='true')await inventory.click();
      assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),`topology fits ${width}px`);
      await page.evaluate(()=>scrollTo(0,0));await page.waitForTimeout(100);
      await page.screenshot({path:`${out}/builder-${theme}-${width}.png`,fullPage:true});
    }
    await page.route('**/api/v1/topology',async route=>{
      const response=await route.fetch();
      if(response.ok()){const data=await response.json();data.writer='legacy';await route.fulfill({response,json:data});}
      else await route.fulfill({response});
    });
    await page.getByRole('button',{name:'Reload view',exact:true}).click();
    await page.getByText('Enable automatic topology sync',{exact:true}).click();
    await page.getByText('lqosync3 topology activate --yes --config PATH',{exact:true}).waitFor();
    assert.equal(writes.length,1,'activation guidance cannot activate a writer from the browser');
    assert.deepEqual(errors,[]);
    console.log('PASS: nested hierarchy, disclosure/search/selection, parent names, contextual actions, cache polling/draft focus, explicit WAN discovery including empty/failed routers, themes and 320–1440px layouts.');
  }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
