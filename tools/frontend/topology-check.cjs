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
