// Isolated browser fixture only. Never use production account credentials.
// Checks the read-only Topology, PPPoE, and IPoE views: navigation, evidence, tree selection,
// verified links, shared subscriber details, plan counts, narrow layouts, and GET-only traffic.
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const assert=require('node:assert/strict');
const fs=require('node:fs');
(async()=>{
  const browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH,headless:true,args:['--no-sandbox']});
  const out=process.env.ROUTERS_EVIDENCE_DIR||'/tmp/lqosync-topology-evidence';fs.mkdirSync(out,{recursive:true});
  for(const scheme of ['dark','light']){
    const page=await browser.newPage({viewport:{width:1440,height:1000},colorScheme:scheme});
    const errors=[];page.on('pageerror',e=>errors.push(e.message));
    // TQ-01: entry, view changes, and filters send only GET requests.
    const writes=[];page.on('request',r=>{if(r.url().includes('/api/v1/topology')&&r.method()!=='GET')writes.push(`${r.method()} ${r.url()}`)});
    await page.goto('http://127.0.0.1:19581');
    await page.getByLabel('Username',{exact:true}).fill('browser-test');
    await page.getByLabel('Password',{exact:true}).fill(process.env.BROWSER_TEST_PASSWORD);
    await page.getByRole('button',{name:'Sign in',exact:true}).click();
    const nav=page.getByRole('navigation',{name:'Main navigation'});

    // Builder: evidence, tree in hierarchy order, and node details on selection.
    await nav.getByRole('link',{name:'Topology'}).click();
    await page.waitForURL('**/topology');
    await page.getByRole('heading',{name:'Topology Manager',level:1}).waitFor();
    assert.equal(await nav.getByRole('link',{name:'Topology'}).getAttribute('aria-current'),'page');
    const evidence=page.getByLabel('Topology evidence');
    await evidence.getByText('Revision 42').waitFor();
    await evidence.getByText('Topology · apply').waitFor();
    assert.match(await evidence.innerText(),/r42 · \d+s ago/);
    // Files, import, and native verification stay separate facts.
    assert.match(await evidence.innerText(),/Files \/ import\s+r42 · import confirmed/);
    assert.match(await evidence.innerText(),/Native verification\s+r42 · waiting/);
    const tree=page.getByRole('list',{name:'Topology nodes'});
    const names=await tree.locator('.tree-node strong').allInnerTexts();
    assert.deepEqual(names,['WAN North','PPPoE access','WAN South','DHCP access']);
    const node=tree.getByRole('button',{name:/PPPoE access/});
    assert.equal(await node.getAttribute('aria-pressed'),'false');
    await node.click();
    assert.equal(await node.getAttribute('aria-pressed'),'true');
    const details=page.getByRole('region',{name:'Details'});
    await details.getByText('Automatic · plan sum').waitFor();
    await details.getByText('400 / 200 Mbps').first().waitFor();
    if(scheme==='dark')await page.screenshot({path:`${out}/topology-builder-${scheme}.png`,fullPage:true});

    // Changes: the latest write without paths, and the web change history.
    await page.getByRole('navigation',{name:'Topology views'}).getByRole('link',{name:'Changes'}).click();
    await page.waitForURL('**/topology?tab=changes');
    const latest=page.getByRole('region',{name:'Latest write'});
    assert.match(await latest.innerText(),/w-42-1[\s\S]*import confirmed[\s\S]*42[\s\S]*Not accepted yet/);
    assert.match(await latest.getByRole('list',{name:'Written files'}).innerText(),/ShapedDevices\.csv\s+written[\s\S]*network\.json\s+unchanged/);
    assert.doesNotMatch(await latest.innerText(),/\/opt\//);
    assert.match(await page.getByRole('region',{name:'Web change history'}).innerText(),/Create plan Access 100[\s\S]*browser-test[\s\S]*saved[\s\S]*42 from 41/);
    await page.getByRole('navigation',{name:'Topology views'}).getByRole('link',{name:'Builder'}).click();

    // Assignments: every saved record, offline included; a deep link opens one subscriber.
    await page.getByRole('navigation',{name:'Topology views'}).getByRole('link',{name:'Assignments'}).click();
    await page.waitForURL('**/topology?tab=assignments');
    const table=page.getByRole('region',{name:'Subscriber assignments'});
    await table.locator('tbody tr').first().waitFor();
    assert.equal(await table.locator('tbody tr').count(),4);
    await page.getByText('4 subscribers match.',{exact:false}).waitFor();
    await page.getByLabel('State').selectOption('offline');
    await page.waitForFunction(()=>document.querySelectorAll('[aria-label="Subscriber assignments"] tbody tr').length===1);
    await table.getByRole('link',{name:'tm-sub-0000000000000004'}).click();
    await page.waitForURL('**/topology?tab=assignments&subscriber=tm-sub-0000000000000004');
    const panel=page.getByRole('region',{name:'Selected subscriber'});
    await panel.getByText('Not active in the preview. No current endpoint.').waitFor();
    assert.match(await panel.innerText(),/topology pin: WAN North/);

    // PPPoE Users: verified links, observation-only rows, the shared subscriber record.
    await nav.getByRole('link',{name:'PPPoE'}).click();
    await page.waitForURL('**/pppoe');
    await page.getByRole('heading',{name:'PPPoE',level:1}).waitFor();
    const ppp=page.getByRole('region',{name:'PPPoE identity records'});
    await ppp.locator('tbody tr').nth(3).waitFor();
    assert.equal(await ppp.locator('tbody tr').count(),4);
    assert.match(await ppp.locator('tbody tr').nth(3).innerText(),/Unlinked[\s\S]*No policy actions/);
    assert.match(await ppp.locator('tbody tr').nth(2).innerText(),/Offline[\s\S]*No current endpoint[\s\S]*Pinned: WAN North/);
    await page.getByLabel('Identity link').selectOption('verified');
    await page.waitForFunction(()=>document.querySelectorAll('[aria-label="PPPoE identity records"] tbody tr').length===3);
    await ppp.getByRole('button',{name:'tm-sub-0000000000000002'}).click();
    const shared=page.getByRole('region',{name:'Selected subscriber'});
    await shared.getByText('2 verified identities',{exact:false}).waitFor();
    assert.match(await shared.innerText(),/Access 100 · 100 \/ 50 Mbps/);
    if(scheme==='dark')await page.screenshot({path:`${out}/pppoe-${scheme}.png`,fullPage:true});

    // IPoE Clients: the same subscriber resolves to the same stable ID.
    await nav.getByRole('link',{name:'IPoE / DHCP'}).click();
    await page.waitForURL('**/ipoe');
    const dhcp=page.getByRole('region',{name:'DHCP client records'});
    await dhcp.locator('tbody tr').nth(2).waitFor();
    assert.equal(await dhcp.locator('tbody tr').count(),3);
    await dhcp.getByRole('button',{name:'tm-sub-0000000000000002'}).click();
    await page.getByRole('region',{name:'Selected subscriber'}).getByText('2 verified identities',{exact:false}).waitFor();

    // Local plans: source identities and global unique subscribers stay separate.
    await page.getByRole('navigation',{name:'IPoE / DHCP views'}).getByRole('link',{name:'Local plans'}).click();
    await page.waitForURL('**/ipoe?view=plans');
    const plans=page.getByRole('region',{name:'Local plans'});
    await plans.getByRole('columnheader',{name:'DHCP identities'}).waitFor();
    assert.match(await plans.locator('tbody tr').innerText(),/Access 100[\s\S]*100 \/ 50\s+2\s+4/);
    if(scheme==='dark')await page.screenshot({path:`${out}/ipoe-plans-${scheme}.png`,fullPage:true});

    // Narrow layouts: no page-level horizontal scroll; tables scroll in their own region.
    for(const [path,w] of [['/topology',390],['/pppoe',320],['/ipoe',320]]){
      await page.setViewportSize({width:w,height:900});
      await page.goto(`http://127.0.0.1:19581${path}`);
      await page.locator('h1').waitFor();
      await page.waitForTimeout(300);
      assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),`${path} fits ${w}px`);
    }
    assert.deepEqual(writes,[],'no topology request other than GET');

    // A Builder change: draft, service review, save. Writes happen only after explicit clicks.
    await page.setViewportSize({width:1440,height:1000});
    await page.goto('http://127.0.0.1:19581/topology');
    const create=page.getByRole('button',{name:'Create node'});
    await create.click();
    const dialog=page.getByRole('dialog',{name:'Create node'});
    await dialog.waitFor();
    // Escape closes the draft; the browser returns focus to the invoker.
    await page.keyboard.press('Escape');
    await dialog.waitFor({state:'hidden'});
    assert.equal(await page.evaluate(()=>document.activeElement?.textContent),'Create node');
    await create.click();
    // An empty required name stays in the draft (native validation).
    await dialog.getByRole('button',{name:'Review change'}).click();
    assert.equal(await dialog.getByLabel('Name').evaluate(e=>e.matches(':user-invalid')),true);
    assert.equal(writes.length,0);
    await dialog.getByLabel('Name').fill('Tower 7');
    await dialog.getByLabel('Parent').selectOption('tm-node-wan-north');
    await dialog.getByRole('button',{name:'Review change'}).click();
    await dialog.getByRole('heading',{name:'Impact'}).waitFor({timeout:15000});
    const impact=await dialog.innerText();
    assert.match(impact,/Subscribers that move\s+1/);
    assert.match(impact,/added\s+Tower 7/);
    assert.match(impact,/account-03[\s\S]*PPPoE access to WAN South/);
    if(scheme==='dark')await page.screenshot({path:`${out}/builder-review-${scheme}.png`});
    await dialog.getByRole('button',{name:'Save change'}).click();
    await dialog.getByText('Saved as policy revision 43.',{exact:false}).waitFor({timeout:15000});
    await dialog.getByRole('button',{name:'Close',exact:true}).last().click();
    await dialog.waitFor({state:'hidden'});
    const paths=writes.map(w=>w.replace(/[0-9a-f]+-[0-9a-f]{13}-[0-9a-f]{13}/g,'KEY'));
    assert.deepEqual(paths,[
      'POST http://127.0.0.1:19581/api/v1/topology/proposals',
      'POST http://127.0.0.1:19581/api/v1/topology/proposals/KEY/confirm',
    ]);
    // Explicit reads: WAN discovery lists each router with its own state; a free interface
    // opens Create WAN, which goes through the same review and save.
    await page.getByRole('button',{name:'Discover WAN interfaces'}).click();
    const wans=page.getByRole('list',{name:'WAN interfaces'});
    await wans.waitFor({timeout:15000});
    const wanText=await wans.innerText();
    assert.match(wanText,/pppoe-out1[\s\S]*In use/);
    assert.match(wanText,/Access-02[\s\S]*Read failed[\s\S]*the router did not answer/);
    await wans.getByRole('button',{name:'Create WAN'}).click();
    const wanDialog=page.getByRole('dialog',{name:'Create WAN on Core-01 pppoe-out3'});
    await wanDialog.getByLabel('Download, Mbps').fill('500');
    await wanDialog.getByLabel('Upload, Mbps').fill('100');
    await wanDialog.getByRole('button',{name:'Review change'}).click();
    await wanDialog.getByRole('heading',{name:'Impact'}).waitFor({timeout:15000});
    await page.keyboard.press('Escape');
    await wanDialog.waitFor({state:'hidden'});
    await page.getByRole('button',{name:'Discover sources'}).click();
    const found=page.getByRole('list',{name:'New sources'});
    await found.getByText('Hotspot Access-01').waitFor({timeout:15000});
    await page.getByRole('button',{name:'Refresh observations'}).click();
    await page.getByText('Observations refreshed.').waitFor({timeout:15000});
    const reads=writes.filter(w=>w.endsWith('/api/v1/topology/reads')).length;
    assert.equal(reads,3,'one read POST per click');

    // Plans: Create plan goes through the same review, which lists rate and capacity changes.
    await page.getByRole('navigation',{name:'Topology views'}).getByRole('link',{name:'Policies'}).click();
    await page.getByRole('button',{name:'Create plan'}).click();
    const planDialog=page.getByRole('dialog',{name:'Create plan'});
    await planDialog.getByLabel('Name').fill('Access 200');
    await planDialog.getByLabel('Download, Mbps').fill('200');
    await planDialog.getByLabel('Upload, Mbps').fill('100');
    await planDialog.getByRole('button',{name:'Review change'}).click();
    await planDialog.getByRole('heading',{name:'Rate changes: 1 subscribers'}).waitFor({timeout:15000});
    assert.match(await planDialog.innerText(),/account-02[\s\S]*min 50 \/ 25 Mbps, max 100 \/ 50 Mbps to min 100 \/ 50 Mbps/);
    assert.match(await planDialog.innerText(),/Capacity changes[\s\S]*PPPoE access[\s\S]*400 \/ 200 Mbps to 500 \/ 250 Mbps/);
    await planDialog.getByRole('button',{name:'Save change'}).click();
    await planDialog.getByText('Saved as policy revision 43.',{exact:false}).waitFor({timeout:15000});
    await page.keyboard.press('Escape');
    await planDialog.waitFor({state:'hidden'});
    await page.getByRole('button',{name:'Edit plan Access 100'}).waitFor();
    // Speed rules, source factors, and fallback use the same review.
    const speedText=await page.getByRole('region',{name:'Speed rules'}).innerText();
    assert.match(speedText,/Upload maximum\s+100 percent, imported rounded rule/);
    assert.match(speedText,/1 profiles have their own rules/);
    await page.getByRole('button',{name:'Edit speed rules'}).click();
    const speed=page.getByRole('dialog',{name:'Edit speed rules'});
    assert.match(await speed.innerText(),/resets the own rules of 1 profiles/);
    await speed.getByLabel('Download minimum',{exact:true}).fill('25');
    await speed.getByRole('button',{name:'Review change'}).click();
    await speed.getByRole('heading',{name:'Rate changes: 1 subscribers'}).waitFor({timeout:15000});
    await speed.getByRole('button',{name:'Save change'}).click();
    await speed.getByText('Saved as policy revision 43.',{exact:false}).waitFor({timeout:15000});
    await page.keyboard.press('Escape');
    await speed.waitFor({state:'hidden'});
    await page.getByRole('button',{name:'Edit factor of PPPoE access'}).click();
    const factor=page.getByRole('dialog',{name:'Edit factor of PPPoE access'});
    await factor.getByRole('button',{name:'Review change'}).click();
    await factor.getByText('Change a factor first.').waitFor();
    await factor.getByLabel('Download factor').fill('0.4');
    await factor.getByRole('button',{name:'Review change'}).click();
    await factor.getByRole('heading',{name:'Impact'}).waitFor({timeout:15000});
    await page.keyboard.press('Escape');
    await factor.waitFor({state:'hidden'});
    assert.equal(await page.getByRole('button',{name:'Clear fallback'}).isDisabled(),true,'no fallback to clear');
    await page.getByRole('button',{name:'Set fallback'}).click();
    const fallback=page.getByRole('dialog',{name:'Set fallback'});
    await fallback.getByLabel('DHCP access').check();
    await fallback.getByLabel('Default download, Mbps').fill('10');
    await fallback.getByLabel('Default upload, Mbps').fill('5');
    await fallback.getByRole('button',{name:'Review change'}).click();
    await fallback.getByRole('heading',{name:'Impact'}).waitFor({timeout:15000});
    await page.keyboard.press('Escape');
    await fallback.waitFor({state:'hidden'});
    // A subscriber plan and a pin start from the shared subscriber record.
    await nav.getByRole('link',{name:'PPPoE'}).click();
    const users=page.getByRole('region',{name:'PPPoE identity records'});
    await users.getByRole('button',{name:'tm-sub-0000000000000002'}).click();
    await page.getByRole('region',{name:'Selected subscriber'}).getByRole('button',{name:'Change plan'}).click();
    const choose=page.getByRole('dialog',{name:'Change plan of tm-sub-0000000000000002'});
    await choose.getByLabel('Local plan').selectOption('');
    await choose.getByRole('button',{name:'Review change'}).click();
    await choose.getByRole('heading',{name:'Impact'}).waitFor({timeout:15000});
    await page.keyboard.press('Escape');
    await choose.waitFor({state:'hidden'});
    await page.getByRole('region',{name:'Selected subscriber'}).getByRole('button',{name:'Pin to WAN'}).click();
    const pin=page.getByRole('dialog',{name:'Pin tm-sub-0000000000000002 to a WAN'});
    assert.match(await pin.innerText(),/writes no router rule/);
    await pin.getByLabel('WAN').selectOption('tm-node-wan-south');
    await pin.getByRole('button',{name:'Review change'}).click();
    await pin.getByRole('heading',{name:'Impact'}).waitFor({timeout:15000});
    await pin.getByRole('button',{name:'Save change'}).click();
    await pin.getByText('Saved as policy revision 43.',{exact:false}).waitFor({timeout:15000});
    await page.keyboard.press('Escape');
    await pin.waitFor({state:'hidden'});
    const sent=writes.map(w=>w.replace(/^POST http:\/\/127\.0\.0\.1:19581\/api\/v1\/topology\//,'').replace(/[0-9a-f]+-[0-9a-f]{13}-[0-9a-f]{13}/g,'KEY'));
    assert.ok(sent.every(p=>/^(proposals|proposals\/KEY\/(confirm|cancel)|reads)$/.test(p)),sent.join(' '));
    await page.goto('http://127.0.0.1:19581/topology');

    // Remove: review, then Back cancels the kept proposal.
    await page.getByRole('list',{name:'Topology nodes'}).getByRole('button',{name:/DHCP access/}).click();
    await page.getByRole('button',{name:'Remove node'}).click();
    const remove=page.getByRole('dialog',{name:'Remove DHCP access'});
    await remove.getByRole('button',{name:'Review change'}).click();
    await remove.getByRole('heading',{name:'Impact'}).waitFor({timeout:15000});
    await remove.getByRole('button',{name:'Back to the change'}).click();
    await remove.getByRole('button',{name:'Review change'}).waitFor();
    await page.waitForTimeout(300);
    assert.ok(writes.some(w=>/proposals\/[^/]+\/cancel$/.test(w)),'Back cancels the review');
    await page.keyboard.press('Escape');
    await remove.waitFor({state:'hidden'});
    assert.deepEqual(errors,[]);
    await page.close();
  }
  await browser.close();
  console.log('PASS: Topology Builder change review and save, Builder/Assignments, PPPoE Users, IPoE Clients, Local plans, shared subscriber record, both themes, 320-390px, GET only.');
})().catch(e=>{console.error(e);process.exit(1);});
