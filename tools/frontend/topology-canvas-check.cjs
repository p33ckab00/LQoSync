// Synthetic loopback fixture only. No production account, router, or shaping write.
// Checks the Topology Sankey canvas: cards, WAN streams, popovers, automatic edits, modals, and narrow screens.
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const assert=require('node:assert/strict');
const fs=require('node:fs');
(async()=>{
  const browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH,headless:true,args:['--no-sandbox']});
  const out=process.env.ROUTERS_EVIDENCE_DIR||'/tmp/lqosync-topology-canvas';fs.mkdirSync(out,{recursive:true});
  try{
    const page=await browser.newPage({viewport:{width:1440,height:1000},colorScheme:'light'});
    const errors=[],edits=[],layouts=[],reads=[],proposals=[];
    page.on('pageerror',e=>errors.push(e.message));
    page.on('request',r=>{
      if(r.url().endsWith('/api/v1/topology/edits'))edits.push(JSON.parse(r.postData()));
      if(r.url().endsWith('/api/v1/topology/layout'))layouts.push(JSON.parse(r.postData()));
      if(r.url().endsWith('/api/v1/topology/reads'))reads.push(JSON.parse(r.postData()));
      // Only the one-time routing adoption is a reviewed change on the canvas.
      if(r.url().endsWith('/api/v1/topology/proposals')){const body=JSON.parse(r.postData());proposals.push(body.edit);
        if(body.edit.kind!=='wan.adopt')errors.push('a canvas edit used the review route');}
    });
    await page.goto('http://127.0.0.1:19581');
    await page.getByLabel('Username',{exact:true}).fill('browser-test');
    await page.getByLabel('Password',{exact:true}).fill(process.env.BROWSER_TEST_PASSWORD);
    await page.getByRole('button',{name:'Sign in',exact:true}).click();
    await page.getByRole('navigation',{name:'Main navigation'}).getByRole('link',{name:'Topology',exact:true}).click();
    const canvas=page.getByRole('region',{name:/^Topology canvas\./});
    const cards=canvas.locator('.canvas-node');await cards.nth(3).waitFor();
    assert.equal(await cards.count(),4,'one card for each node: the two PPPoE placements are one card');
    // Sankey: one filled ribbon for each WAN stream between two cards. Width = configured capacity.
    const ribbons=canvas.locator('path.sankey-ribbon');await ribbons.nth(5).waitFor();
    assert.equal(await ribbons.count(),6,'three capacity streams into the groups and three into the service tiers');
    await canvas.locator('path.sankey-live').first().waitFor({state:'attached'});
    assert.ok(await canvas.locator('path.sankey-live').count()>=1,'a live band inside the capacity ribbons');
    assert.match(await canvas.locator('path.sankey-ribbon[data-from="tm-node-wan-north"]').getAttribute('d'),/Z$/);
    for(const [d,wan] of await ribbons.evaluateAll(ps=>ps.map(p=>[p.getAttribute('d'),p.dataset.wan]))){
      assert.match(d,/Z$/,'a ribbon is a closed shape');assert.ok(wan,'each ribbon names its WAN');}
    assert.equal(await page.locator('[data-metric]').getAttribute('data-metric'),'capacity');
    // A WAN card: configured capacity first, then the allocated share (the ribbon) and its percent.
    const wanText=await canvas.locator('.sankey-card[data-node-id="tm-node-wan-north"]').innerText();
    assert.match(wanText,/1000 Mbps[\s\S]*400 Mbps allocated · 40% allocated/,wanText);
    assert.match(await canvas.locator('.sankey-card[data-node-id="tm-node-wan-north"] .sankey-value').innerText(),/^1000 Mbps$/);
    const columns=await canvas.locator('.sankey-card').evaluateAll(cs=>Object.fromEntries(cs.map(c=>[c.dataset.nodeId,+c.dataset.column])));
    assert.deepEqual([columns['tm-node-wan-north'],columns['source:access-ppp'],columns['tier:ppp']],[0,1,2],'columns follow the paths');
    const boxes=await canvas.locator('.sankey-card').evaluateAll(cs=>cs.map(c=>c.getBoundingClientRect().toJSON()));
    for(const [i,a] of boxes.entries())for(const b of boxes.slice(i+1))
      assert.ok(a.right<=b.left||b.right<=a.left||a.bottom<=b.top||b.bottom<=a.top,'no card covers another card');
    assert.equal(await canvas.locator('[data-node-id="source:access-ppp"]').count(),1);
    assert.match(await canvas.getByRole('button',{name:/^PPPoE access,/}).getAttribute('aria-label'),/5 subscribers/);
    // Toolbar: text tabs, revision chips, the live chip, and the WAN legend.
    assert.match(await page.getByRole('navigation',{name:'Topology views'}).innerText(),/Sankey[\s\S]*Assignments[\s\S]*Policies[\s\S]*Changes/,'the tabs show their text');
    const chips=page.locator('.sankey-chips');
    await chips.getByText('Saved rev 42',{exact:true}).waitFor();
    await chips.getByText(/^(Applied rev \d+|Not applied)$/).waitFor();
    await chips.getByText(/^Live · \d+s ago$/).waitFor();
    const legend=page.getByRole('list',{name:'WAN streams'});
    assert.deepEqual(await legend.locator('li').allInnerTexts(),['WAN North','WAN South']);
    const lb=await legend.boundingBox();
    for(const b of await canvas.locator('.sankey-card').evaluateAll(cs=>cs.map(c=>c.getBoundingClientRect().toJSON())))
      assert.ok(lb.x+lb.width<=b.left||b.right<=lb.x||lb.y+lb.height<=b.top||b.bottom<=lb.y,'the legend covers no card');
    // Data states, from a changed throughput reply.
    const rewrite=change=>page.route('**/api/v1/topology/throughput',async r=>{const res=await r.fetch();const j=await res.json();change(j);await r.fulfill({response:res,json:j});});
    await rewrite(j=>{j.complete=false;j.unmapped_hosts=3;});
    const partial=chips.getByText('Partial',{exact:true});await partial.waitFor({timeout:12000});
    assert.match(await partial.getAttribute('title'),/3 hosts have no subscriber/);
    await page.unroute('**/api/v1/topology/throughput');
    await rewrite(j=>{j.stale=true;j.complete=false;j.reason='the host sample is old';});
    await chips.getByText('No live traffic',{exact:true}).waitFor({timeout:12000});
    assert.equal(await page.locator('[data-metric]').getAttribute('data-metric'),'capacity','a stale sample keeps the capacity ribbons');
    assert.equal(await canvas.locator('path.sankey-ribbon').count(),6);
    assert.equal(await canvas.locator('path.sankey-live').count(),0,'no live band without a fresh sample');
    await page.unroute('**/api/v1/topology/throughput');
    await chips.getByText(/^Live · /).waitFor({timeout:12000});
    // A gateway that stops answering: the kept sample turns stale and never stays "Live".
    await page.route('**/api/v1/topology/throughput',r=>r.abort());
    await chips.getByText('No live traffic',{exact:true}).waitFor({timeout:12000});
    await page.unroute('**/api/v1/topology/throughput');
    await chips.getByText(/^Live · /).waitFor({timeout:12000});
    assert.equal(await page.getByText(/instance/i).count(),0,'the word instance never shows');
    const dock=page.getByRole('toolbar',{name:'Topology tools'});
    for(const name of ['Router inventory','Topology settings','Subscriber allocation','Preview and evidence']){
      const box=await dock.getByRole('button',{name,exact:true}).boundingBox();
      assert.ok(box.width>=44&&box.height>=44,`${name} target is at least 44 px`);
    }
    // Selection highlights the active relationship.
    const north=canvas.getByRole('button',{name:/^WAN North,/});
    await north.click();
    assert.equal(await north.getAttribute('aria-pressed'),'true');
    assert.equal(await canvas.locator('path.sankey-ribbon.active').count(),1,'the stream of WAN North stands out');
    // The popover: the card rate, one line for each WAN, and the way to the full settings.
    const pop=page.getByRole('dialog',{name:'WAN North details'});await pop.waitFor();
    await pop.getByText(/Mbps/).first().waitFor();
    await page.screenshot({path:`${out}/canvas-light.png`});
    await pop.getByRole('button',{name:'Open full settings',exact:true}).click();
    await page.getByRole('dialog',{name:'WAN North'}).waitFor();
    await page.keyboard.press('Escape');await page.getByRole('dialog',{name:'WAN North'}).waitFor({state:'hidden'});
    // Escape closes the popover and keeps the focus on the card.
    await north.focus();await north.click();await pop.waitFor();
    await page.keyboard.press('Escape');await pop.waitFor({state:'hidden'});
    assert.equal(await north.evaluate(el=>el===document.activeElement),true,'focus stays on the card');
    // The card names its popover and reads its live value. Escape inside the popover returns focus to the card.
    await north.click();await pop.waitFor();
    assert.equal(await north.getAttribute('aria-describedby'),await pop.getAttribute('id'),'the card names its popover');
    assert.match(await north.getAttribute('aria-label'),/^WAN North, [\d.]+ Mbps,/,'the label reads the live value');
    await pop.getByRole('button',{name:'Open full settings',exact:true}).focus();
    await page.keyboard.press('Escape');await pop.waitFor({state:'hidden'});
    assert.equal(await north.evaluate(el=>el===document.activeElement),true,'Escape returns focus to the card');
    // Enter on the selected card opens its modal; a Google Maps paste saves the full text.
    await north.focus();await page.keyboard.press('Enter');
    const dialog=page.getByRole('dialog',{name:'WAN North'});await dialog.waitFor();
    await dialog.getByLabel('Map type').selectOption('site');
    await page.waitForFunction(()=>true);
    const lat=dialog.getByLabel('Latitude',{exact:true});
    await lat.focus();
    await lat.evaluate(el=>{const dt=new DataTransfer();dt.setData('text','12.236199459585354, 122.02419129509887');
      el.dispatchEvent(new ClipboardEvent('paste',{clipboardData:dt,bubbles:true,cancelable:true}));});
    // Selecting Site saves the type at once; the paste then saves both coordinates together.
    for(let i=0;i<100&&!edits.some(e=>e.edit.location);i++)await page.waitForTimeout(100);
    const location=edits.find(e=>e.edit.kind==='node.location'&&e.edit.location);
    assert.ok(location,'the paste saved a location edit');
    assert.deepEqual(location.edit,{kind:'node.location',node_id:'tm-node-wan-north',geo_type:'site',
      location:{latitude:'12.236199459585354',longitude:'122.02419129509887'}});
    assert.equal(await dialog.getByLabel('Longitude',{exact:true}).inputValue(),'122.02419129509887');
    assert.equal(await dialog.getByRole('button',{name:/^(Save|Approve|Confirm)/}).count(),0,'no approval step');
    await page.screenshot({path:`${out}/node-modal.png`});
    // Escape closes the modal and returns focus to the card.
    await page.keyboard.press('Escape');await dialog.waitFor({state:'hidden'});
    assert.equal(await north.evaluate(el=>el===document.activeElement),true,'focus returns to the card');
    // An invalid coordinate stays a draft with an inline error and sends nothing.
    const sent=edits.length;
    await page.keyboard.press('Enter');await dialog.waitFor();
    await dialog.getByLabel('Latitude',{exact:true}).fill('95');
    await dialog.getByLabel('Longitude',{exact:true}).fill('121.5');
    await dialog.getByLabel('Longitude',{exact:true}).press('Tab');
    await dialog.getByRole('alert').filter({hasText:'latitude must be from -90 to 90'}).waitFor();
    assert.equal(edits.length,sent,'an invalid location is not sent');
    await dialog.getByRole('button',{name:'Close',exact:true}).last().click();await dialog.waitFor({state:'hidden'});
    const focused=await page.evaluate(()=>document.activeElement?.getAttribute('aria-label')||document.activeElement?.tagName);
    assert.match(focused,/^WAN North,/,`Close returns focus to the card, not ${focused}`);
    await page.keyboard.press('Enter');await dialog.waitFor();
    assert.equal(await dialog.getByLabel('Latitude',{exact:true}).inputValue(),'95','the invalid draft stays');
    await dialog.getByRole('alert').waitFor();
    await page.keyboard.press('Escape');
    // Inventory: discovery adds nothing until a WAN is placed.
    await dock.getByRole('button',{name:'Router inventory',exact:true}).click();
    const inventory=page.getByRole('dialog',{name:'Router inventory'});await inventory.waitFor();
    // One section for each router. Opening reads no router; Find reads only that router.
    const core=inventory.getByRole('region',{name:'Core-01',exact:true});await core.waitFor();
    await inventory.getByRole('region',{name:'Access-01',exact:true}).waitFor();
    assert.equal(reads.length,0,'opening the inventory reads no router');
    const coreWans=core.getByRole('table',{name:'WAN interfaces'});
    await coreWans.getByRole('row',{name:/pppoe-out1/}).getByText('On canvas as WAN North').waitFor();
    await coreWans.getByRole('row',{name:/pppoe-out3/}).getByText('Default route, not configured').waitFor();
    await core.getByRole('button',{name:'Find WAN interfaces',exact:true}).click();
    await inventory.getByText('WAN interfaces read.').first().waitFor({timeout:15000});
    assert.deepEqual(reads.at(-1).routers,['lq3-00000000000000c1'],'only this router is read');
    assert.equal(await coreWans.getByRole('row',{name:/pppoe-out3/}).count(),1,'an interface shows once');
    await page.screenshot({path:`${out}/inventory.png`});
    await core.getByRole('button',{name:'Add pppoe-out3 as a WAN',exact:true}).click();
    assert.equal(edits.filter(e=>e.edit.kind==='wan.create').length,0,'discovery attaches nothing');
    await inventory.getByLabel('Download Mbps').fill('300');await inventory.getByLabel('Upload Mbps').fill('100');
    await inventory.getByRole('button',{name:'Add WAN to canvas',exact:true}).click();
    await page.waitForFunction(()=>[...document.querySelectorAll('[role=status]')].some(s=>s.textContent.includes('Saved revision')));
    const wan=edits.find(e=>e.edit.kind==='wan.create');
    assert.equal(wan.edit.endpoint_ref,'pppoe-out3');assert.equal(wan.edit.down_mbps,300);
    await page.keyboard.press('Escape');
    // Allocation: the topology default and the mode of one group save one edit each.
    const ppp=canvas.getByRole('button',{name:/^PPPoE access,/});await ppp.click();
    await dock.getByRole('button',{name:'Subscriber allocation',exact:true}).click();
    const allocation=page.getByRole('dialog',{name:'Subscriber allocation'});
    const wait=async kind=>{for(let i=0;i<100&&!edits.some(e=>e.edit.kind===kind);i++)await page.waitForTimeout(100);
      return edits.filter(e=>e.edit.kind===kind).at(-1)?.edit;};
    const topo=allocation.getByRole('group',{name:'Topology default'});
    assert.equal(await topo.getByLabel(/^Equal subscriber count/).isChecked(),true,'count is the default');
    await topo.getByLabel(/^Quality/).check();
    assert.deepEqual(await wait('allocation.default'),{kind:'allocation.default',mode:'quality'});
    const own=allocation.getByLabel('Mode of PPPoE access');
    assert.equal(await own.inputValue(),'','the group follows the topology default');
    await own.selectOption('count');
    assert.deepEqual(await wait('group.allocation'),{kind:'group.allocation',source_id:'access-ppp',mode:'count'});
    const groupEdits=edits.filter(e=>e.edit.kind==='group.allocation').length;
    await own.selectOption('');
    for(let i=0;i<100&&edits.filter(e=>e.edit.kind==='group.allocation').length===groupEdits;i++)await page.waitForTimeout(100);
    assert.deepEqual(edits.filter(e=>e.edit.kind==='group.allocation').at(-1).edit,{kind:'group.allocation',source_id:'access-ppp',mode:null});
    await page.keyboard.press('Escape');await allocation.waitFor({state:'hidden'});
    // WAN health is text on the WAN cards.
    await north.locator('.canvas-node-health').filter({hasText:'Up, 87%'}).waitFor();
    assert.match(await north.getAttribute('aria-label'),/, 24 routed[,.]/,'the routed count is in the card name');
    // The WAN modal names its routing list. A change saves one node.address_list edit.
    const southCard=canvas.getByRole('button',{name:/^WAN South,/});
    await southCard.click();await page.keyboard.press('Enter');
    const southDialog=page.getByRole('dialog',{name:'WAN South'});await southDialog.waitFor();
    const list=southDialog.getByLabel('Address list');
    assert.equal(await list.inputValue(),'via-WAN-South');
    await list.fill('via-pldt');await list.press('Tab');
    for(let i=0;i<100&&!edits.some(e=>e.edit.kind==='node.address_list');i++)await page.waitForTimeout(100);
    assert.deepEqual(edits.find(e=>e.edit.kind==='node.address_list').edit,{kind:'node.address_list',node_id:'tm-node-wan-south',list:'via-pldt'});
    await page.keyboard.press('Escape');await southDialog.waitFor({state:'hidden'});
    const south=canvas.getByRole('button',{name:/^WAN South,/});
    await south.locator('.canvas-node-health').filter({hasText:'Waiting for Rebalance'}).waitFor();
    // Settings: automatic failback and the manual return save one edit each.
    await dock.getByRole('button',{name:'Topology settings',exact:true}).click();
    const settings=page.getByRole('dialog',{name:'Topology settings'});await settings.waitFor();
    const failback=settings.getByLabel('Automatic failback');
    assert.equal(await failback.isChecked(),true);
    await failback.uncheck();
    assert.deepEqual(await wait('failback.auto'),{kind:'failback.auto',enabled:false});
    await settings.getByRole('button',{name:'Return subscribers to recovered WANs',exact:true}).click();
    assert.deepEqual(await wait('failback.release'),{kind:'failback.release'});
    // Routing lists: the last write as text, and the one-time adoption as a reviewed change.
    await settings.getByText('Written · 24 entries · 3 added, 1 changed, 0 removed').waitFor();
    await page.screenshot({path:`${out}/settings-failback.png`});
    await settings.getByRole('button',{name:'Adopt current routing',exact:true}).click();
    const adoptDialog=page.getByRole('dialog',{name:'Adopt current WAN routing'});await adoptDialog.waitFor();
    await adoptDialog.getByRole('button',{name:'Review change',exact:true}).click();
    await adoptDialog.getByRole('button',{name:'Save change',exact:true}).waitFor({timeout:15000});
    assert.deepEqual(proposals,[{kind:'wan.adopt'}],'one reviewed adoption');
    await adoptDialog.getByRole('button',{name:'Back to the change',exact:true}).click();
    await adoptDialog.getByRole('button',{name:'Cancel',exact:true}).click();await adoptDialog.waitFor({state:'hidden'});
    await dock.getByRole('button',{name:'Topology settings',exact:true}).click();await settings.waitFor();
    await page.keyboard.press('Escape');await settings.waitFor({state:'hidden'});
    // Preview is read-only.
    const count=edits.length;
    await dock.getByRole('button',{name:'Preview and evidence',exact:true}).click();
    await page.getByRole('dialog',{name:'Preview and evidence'}).getByText('Read-only.',{exact:false}).waitFor();
    await page.keyboard.press('Escape');
    assert.equal(edits.length,count,'preview sends nothing');
    // Automatic placement: router badge, auto chip, router select, and Inventory status.
    const dhcp=canvas.getByRole('button',{name:/^DHCP access,/});
    await dhcp.locator('.canvas-node-router').filter({hasText:'Router Access-01'}).waitFor();
    await north.locator('.canvas-node-auto').filter({hasText:'+2 auto'}).waitFor();
    await dhcp.click();await page.keyboard.press('Enter');
    const dhcpDialog=page.getByRole('dialog',{name:'DHCP access'});await dhcpDialog.waitFor();
    const routerOf=d=>d.getByLabel('Access router');
    await routerOf(dhcpDialog).locator('option').nth(2).waitFor({state:'attached'});
    assert.deepEqual(await routerOf(dhcpDialog).locator('option').allTextContents(),['None','Access-01','Core-01'],
      'the own router and routers without a router node');
    const before2=edits.length;
    await routerOf(dhcpDialog).selectOption('');
    for(let i=0;i<100&&edits.length===before2;i++)await page.waitForTimeout(100);
    const routerEdits=edits.slice(before2).filter(e=>e.edit.kind==='node.router');
    assert.deepEqual(routerEdits.map(e=>e.edit),[{kind:'node.router',node_id:'tm-node-dhcp-south',router_id:null}],'one router edit');
    await page.keyboard.press('Escape');await dhcpDialog.waitFor({state:'hidden'});
    // A refused family edit keeps the draft and shows the service sentence at the select.
    await ppp.click();await page.keyboard.press('Enter');
    const pppDialog=page.getByRole('dialog',{name:'PPPoE access'});await pppDialog.waitFor();
    await routerOf(pppDialog).locator('option').nth(1).waitFor({state:'attached'});
    assert.deepEqual(await routerOf(pppDialog).locator('option').allTextContents(),['None','Core-01'],
      'a router with a router node elsewhere is not offered');
    await routerOf(pppDialog).selectOption('lq3-00000000000000c1');
    await pppDialog.locator('#canvas-router-error').filter({hasText:'branch of another router'}).waitFor({timeout:15000});
    await page.getByRole('status').filter({hasText:'Not saved: PPPoE access is in the branch'}).first().waitFor();
    assert.equal(await routerOf(pppDialog).inputValue(),'lq3-00000000000000c1','the draft stays');
    assert.equal(await routerOf(pppDialog).evaluate(el=>el===document.activeElement),true,'focus is on the select');
    const described=await routerOf(pppDialog).getAttribute('aria-describedby');
    assert.ok(described&&await page.locator(`#${described}`).isVisible(),'the error describes the select');
    await page.screenshot({path:`${out}/router-refused.png`});
    const refusedSent=edits.length;
    await page.keyboard.press('Escape');await pppDialog.waitFor({state:'hidden'});
    assert.equal(await ppp.evaluate(el=>el===document.activeElement),true,'Escape returns focus to the card');
    await page.waitForTimeout(600);
    assert.equal(edits.length,refusedSent,'closing does not send the refused draft again');
    await dock.getByRole('button',{name:'Router inventory',exact:true}).click();
    const inv=page.getByRole('dialog',{name:'Router inventory'});await inv.waitFor();
    // Each source has one state and one action in its router section.
    const srcs=inv.getByRole('region',{name:'Access-01',exact:true}).getByRole('table',{name:'Subscriber sources'});
    await srcs.getByRole('row',{name:/PPPoE Silver/}).getByText('Auto: WAN North',{exact:true}).waitFor();
    await srcs.getByRole('row',{name:/DHCP guest/}).getByText('Not on canvas',{exact:true}).waitFor();
    await srcs.getByRole('row',{name:/PPPoE Trial/}).getByText('Not read',{exact:true}).waitFor();
    await srcs.getByRole('row',{name:/PPPoE access/}).getByText('On canvas',{exact:true}).waitFor();
    // Discover sources: a found source joins the section of its router.
    await inv.getByRole('button',{name:'Discover sources',exact:true}).click();
    await srcs.getByRole('button',{name:'Add source Hotspot Access-01'}).waitFor({timeout:15000});
    await page.setViewportSize({width:390,height:800});
    assert.ok(await page.evaluate(()=>document.scrollingElement.scrollWidth<=390),'no sideways scroll at 390 px');
    await srcs.locator('thead').waitFor({state:'hidden'});
    assert.equal(await srcs.locator('thead').isVisible(),false,'narrow rows stack, without a header row');
    await page.screenshot({path:`${out}/inventory-390.png`});
    await page.setViewportSize({width:1440,height:1000});
    // Show selects the card on the canvas and closes the inventory.
    await srcs.getByRole('button',{name:'Show PPPoE access on the canvas'}).click();
    await inv.waitFor({state:'hidden'});
    assert.equal(await ppp.getAttribute('aria-pressed'),'true');
    // Live source canvas. One helper waits for the next edit of a kind.
    const edit=async kind=>{for(let i=0;i<100;i++){const e=edits.find(x=>x.edit.kind===kind&&!x.seen);if(e){e.seen=true;return e.edit;}await page.waitForTimeout(100);}
      throw new Error(`no ${kind} edit`);};
    // A default-route interface without a WAN node is a "not configured" card; it opens the WAN form.
    const candidate=canvas.getByRole('button',{name:/^pppoe-out3 on Core-01, WAN, not configured/});
    await candidate.click();
    const wanInv=page.getByRole('dialog',{name:'Router inventory'});await wanInv.waitFor();
    await wanInv.getByRole('heading',{name:'New WAN · pppoe-out3 on Core-01'}).waitFor();
    await page.keyboard.press('Escape');await wanInv.waitFor({state:'hidden'});
    // Add link from: the card settings add a second WAN parent (the old connector drag).
    await dhcp.click();await page.keyboard.press('Enter');await dhcpDialog.waitFor();
    await dhcpDialog.getByLabel('Add link from').selectOption({label:'WAN North'});
    assert.deepEqual(await edit('node.instance_add'),{kind:'node.instance_add',node_id:'tm-node-dhcp-south',parent:'tm-node-wan-north'});
    await page.keyboard.press('Escape');await dhcpDialog.waitFor({state:'hidden'});
    // Router card settings: exclusion from the network tree, and a link to the Routers screen.
    await dhcp.click();await page.keyboard.press('Enter');await dhcpDialog.waitFor();
    await dhcpDialog.getByRole('link',{name:'Router connection settings'}).waitFor();
    await dhcpDialog.getByLabel(/^Exclude from network tree/).check();
    assert.deepEqual(await edit('node.tree_exclude'),{kind:'node.tree_exclude',node_id:'tm-node-dhcp-south',excluded:true});
    await page.keyboard.press('Escape');await dhcpDialog.waitFor({state:'hidden'});
    // Do not read: a two-step action in the group modal.
    await ppp.click();await page.keyboard.press('Enter');await pppDialog.waitFor();
    await pppDialog.getByRole('button',{name:'Do not read',exact:true}).click();
    await pppDialog.getByRole('button',{name:'Confirm do not read',exact:true}).click();
    assert.deepEqual(await edit('source.exclude'),{kind:'source.exclude',source_id:'access-ppp',excluded:true});
    await pppDialog.waitFor({state:'hidden'});
    // The Excluded list reads a source again.
    await dock.getByRole('button',{name:'Excluded sources',exact:true}).click();
    const excluded=page.getByRole('dialog',{name:'Excluded sources'});await excluded.waitFor();
    await excluded.getByRole('button',{name:'Read PPPoE Trial again',exact:true}).click();
    assert.deepEqual(await edit('source.exclude'),{kind:'source.exclude',source_id:'access-ppp-trial',excluded:false});
    await page.keyboard.press('Escape');await excluded.waitFor({state:'hidden'});
    // A ribbon: a click shows its stream. Links change only in the card settings.
    await canvas.locator('path.sankey-ribbon[data-from="tm-node-wan-north"][data-to="source:access-ppp"]').dispatchEvent('click');
    const stream=page.getByRole('dialog',{name:'Stream details'});await stream.waitFor();
    await stream.getByText('WAN North · WAN North → PPPoE access').waitFor();
    await stream.getByText(/Mbps down/).waitFor();
    assert.equal(await page.getByRole('button',{name:'Remove link',exact:true}).count(),0,'no link removal on the canvas');
    await page.keyboard.press('Escape');await stream.waitFor({state:'hidden'});
    // By keyboard: the card settings list the lines as text, each with Remove link.
    await ppp.click();await page.keyboard.press('Enter');await pppDialog.waitFor();
    const links=pppDialog.getByRole('group',{name:'Links'});
    await links.getByText('WAN North · 3 subscribers').waitFor();
    await links.getByText('WAN South · 2 subscribers').waitFor();
    assert.equal(await pppDialog.getByText(/Add another/).count(),0,'no Add another parent field');
    await links.getByRole('button',{name:'Remove link from WAN North'}).focus();await page.keyboard.press('Enter');
    assert.deepEqual(await edit('link.remove'),{kind:'link.remove',parent:'tm-node-wan-north',child:'tm-node-ppp-north'});
    await page.keyboard.press('Escape');await pppDialog.waitFor({state:'hidden'});
    // Rebalance routing: the count of moves, then a confirmation.
    await dock.getByRole('button',{name:'Topology settings',exact:true}).click();await settings.waitFor();
    await settings.getByText('Rebalance routing moves 2 subscribers to another WAN. Their open connections break.').waitFor();
    await settings.getByRole('button',{name:'Rebalance routing',exact:true}).click();
    await settings.getByRole('button',{name:'Confirm: move 2',exact:true}).click();
    assert.deepEqual(await edit('routing.rebalance'),{kind:'routing.rebalance',apply:true});
    await page.keyboard.press('Escape');await settings.waitFor({state:'hidden'});
    // List view by keyboard: the same cards open the same modals.
    const listButton=page.getByRole('button',{name:'List view',exact:true});
    await listButton.focus();await page.keyboard.press('Enter');
    const cardList=page.getByRole('list',{name:'Topology cards'});await cardList.waitFor();
    await cardList.getByRole('button',{name:/^WAN North/}).focus();await page.keyboard.press('Enter');
    await dialog.waitFor();await page.keyboard.press('Escape');await dialog.waitFor({state:'hidden'});
    await cardList.getByRole('button',{name:/^pppoe-out3 on Core-01/}).waitFor();
    await listButton.click();await cardList.waitFor({state:'hidden'});
    // Fit keeps every card inside the viewport.
    await page.getByRole('button',{name:'Fit view',exact:true}).click();
    const view=await canvas.boundingBox();
    for(const b of await canvas.locator('.sankey-card').evaluateAll(cs=>cs.map(c=>c.getBoundingClientRect().toJSON())))
      assert.ok(b.left>=view.x-1&&b.right<=view.x+view.width+1&&b.top>=view.y-1&&b.bottom<=view.y+view.height+1,'Fit view shows every card');
    assert.equal(await page.locator('.sankey-minimap rect.sankey-minimap-card').count(),await canvas.locator('.sankey-card, .canvas-candidate').count(),'the minimap shows each card');
    // Infinite canvas: no scroll box; drag the background to pan anywhere; the wheel zooms at the cursor.
    assert.equal(await canvas.evaluate(v=>getComputedStyle(v).overflow),'hidden','no scrollbars');
    const vb=await canvas.boundingBox();
    const frameBox=await page.locator('.canvas-frame').boundingBox();
    assert.ok(vb.width>=frameBox.width-2&&vb.height>=frameBox.height-2,'the canvas fills its frame');
    const northBox=async()=>canvas.getByRole('button',{name:/^WAN North,/}).boundingBox();
    // A point on the empty background, found again before each drag because the cards move.
    const findEmpty=async()=>{const p=await canvas.evaluate(v=>{const r=v.getBoundingClientRect();
      for(let y=r.bottom-120;y>r.top+130;y-=20)for(let x=r.left+r.width*0.45;x<r.right-220;x+=20){
        const e=document.elementFromPoint(x,y);if(e&&(e===v||e.classList.contains('canvas-layer')||e.classList.contains('canvas-surface')))return {x,y};}
      return null;});assert.ok(p,'an empty point on the canvas');return p;};
    let empty=await findEmpty();
    let before=await northBox();
    await page.mouse.move(empty.x,empty.y);await page.mouse.down();
    await page.mouse.move(empty.x+300,empty.y+220,{steps:6});await page.mouse.up();
    let after=await northBox();
    assert.ok(Math.abs(after.x-before.x-300)<2&&Math.abs(after.y-before.y-220)<2,`a background drag pans the canvas ${JSON.stringify([before,after])}`);
    assert.equal(await page.getByRole('dialog',{name:/details$/}).count(),0,'a drag opens no popover');
    before=after;empty=await findEmpty();
    await page.mouse.move(empty.x,empty.y);await page.mouse.down();
    await page.mouse.move(empty.x-900,empty.y-700,{steps:6});await page.mouse.up();
    after=await northBox();
    assert.ok(Math.abs(after.x-before.x+900)<2,'the canvas pans past any edge');
    const mm=await page.locator('.sankey-minimap').boundingBox(),mv=await page.locator('.sankey-minimap-view').boundingBox();
    assert.ok(mv.x>=mm.x-1&&mv.x+mv.width<=mm.x+mm.width+1&&mv.y>=mm.y-1&&mv.y+mv.height<=mm.y+mm.height+1,'the minimap shows the view far from the cards');
    await page.getByRole('button',{name:'Fit view',exact:true}).click();
    before=await northBox();
    const cx=before.x+before.width/2,cy=before.y+before.height/2;
    await page.mouse.move(cx,cy);await page.mouse.wheel(0,-400);
    await page.waitForFunction(w=>{const b=[...document.querySelectorAll('.sankey-card')].find(c=>c.dataset.nodeId==='tm-node-wan-north').getBoundingClientRect();return b.width>w+4},before.width);
    after=await northBox();
    assert.ok(after.x<=cx&&cx<=after.x+after.width&&after.y<=cy&&cy<=after.y+after.height,'the wheel zooms at the cursor');
    const grid=await canvas.evaluate(v=>getComputedStyle(v).backgroundSize);
    assert.notEqual(grid,'24px 24px','the dot grid follows the zoom');
    // Keyboard: arrows pan, 0 fits.
    await canvas.focus();before=await northBox();
    await page.keyboard.press('ArrowRight');after=await northBox();
    assert.ok(after.x<before.x-20,'ArrowRight moves the view to the right');
    await page.keyboard.press('0');
    for(const b of await canvas.locator('.sankey-card').evaluateAll(cs=>cs.map(c=>c.getBoundingClientRect().toJSON())))
      assert.ok(b.left>=vb.x-1&&b.right<=vb.x+vb.width+1&&b.top>=vb.y-1&&b.bottom<=vb.y+vb.height+1,'0 fits every card');
    // A card that gets keyboard focus outside the view comes into view.
    empty=await findEmpty();
    await page.mouse.move(empty.x,empty.y);await page.mouse.down();
    await page.mouse.move(empty.x+1500,empty.y+900,{steps:4});await page.mouse.up();
    await canvas.locator('.sankey-card[data-node-id="tier:ppp"]').focus();
    const t=await canvas.locator('.sankey-card[data-node-id="tier:ppp"]').boundingBox();
    assert.ok(t.x>=vb.x&&t.x+t.width<=vb.x+vb.width&&t.y>=vb.y&&t.y+t.height<=vb.y+vb.height,'focus brings the card into view');
    await page.getByRole('button',{name:'Fit view',exact:true}).click();
    await page.setViewportSize({width:390,height:800});
    assert.ok(await page.evaluate(()=>document.scrollingElement.scrollWidth<=390),'no sideways scroll at 390 px');
    await page.setViewportSize({width:1440,height:1000});
    // Dark theme and a phone width: no sideways page scroll, and the modal fits.
    await page.emulateMedia({colorScheme:'dark'});await page.screenshot({path:`${out}/canvas-dark.png`});
    await page.setViewportSize({width:375,height:760});
    assert.ok(await page.evaluate(()=>document.scrollingElement.scrollWidth<=375),'no sideways scroll at 375 px');
    await north.click();await page.keyboard.press("Enter");await dialog.waitFor();
    const box=await dialog.boundingBox();assert.ok(box.width<=375&&box.x>=0,'the modal fits a phone');
    await page.keyboard.press('Escape');
    await page.setViewportSize({width:768,height:900});
    assert.ok(await page.evaluate(()=>document.scrollingElement.scrollWidth<=768),'no sideways scroll at 768 px');
    await page.emulateMedia({colorScheme:'light'});
    await page.setViewportSize({width:375,height:760});await north.click();await page.keyboard.press('Enter');await dialog.waitFor();
    await page.screenshot({path:`${out}/node-modal-375.png`});
    assert.deepEqual(errors,[]);
    console.log('topology canvas check passed');
  }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exit(1);});
