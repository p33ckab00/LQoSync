// Synthetic fixture with intercepted snapshots. Core API tests verify durable state.
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const out=process.env.TOPOLOGY_EVIDENCE_DIR||'/tmp/lqosync-topology-initialization';fs.mkdirSync(out,{recursive:true});
(async()=>{
 const browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH,headless:true,args:['--no-sandbox']});
 try{
  const page=await browser.newPage({viewport:{width:1280,height:1000}});
  const errors=[],requests=[];let empty=true,initialized=false,placed=false;
  page.on('pageerror',e=>errors.push(e.message));
  page.on('request',r=>{if(r.method()==='POST'&&r.url().includes('/api/v1/topology'))requests.push({url:r.url(),body:r.postDataJSON()});});
  await page.route('**/api/v1/topology',async route=>{
   const response=await route.fetch(),s=await response.json();
   if(empty)Object.assign(s,{registry_revision:initialized?'43':null,enrolled:false,frontend_initialized:initialized,writer:'legacy',nodes:[],sources:[],plans:[],factors:[],fallback:null,subscribers_total:0,subscribers_active:0,ppp_total:0,dhcp_total:0,observed:null,publication:null,history:[]});
   // Before the new link the PPPoE source has one placement; the fixture has two.
   else if(!placed){s.nodes=s.nodes.filter(n=>n.id!=='tm-node-ppp-south');s.flows=s.flows.filter(f=>!(f.from==='tm-node-wan-south'&&f.to==='source:access-ppp'));}
   await route.fulfill({response,json:s});
  });
  await page.route('**/api/v1/topology/operations/*',async route=>{
   const response=await route.fetch(),op=await response.json();
   if(empty&&op.review)Object.assign(op.review,{base_revision:'0',nodes:[],moves:0,moved:[],cleared:0,assigned:0,new_records:0,sources:[],rates_changed:0,rates:[],capacities:[]});
   if(op.state==='saved'){if(empty)initialized=true;else placed=true;}
   await route.fulfill({response,json:op});
  });
  await page.goto('http://127.0.0.1:19581');
  await page.getByLabel('Username',{exact:true}).fill('browser-test');
  await page.getByLabel('Password',{exact:true}).fill(process.env.BROWSER_TEST_PASSWORD);
  await page.getByRole('button',{name:'Sign in',exact:true}).click();
  await page.getByRole('navigation',{name:'Main navigation'}).getByRole('link',{name:'Topology',exact:true}).click();
  const start=page.getByRole('button',{name:'Start empty topology',exact:true});await start.waitFor();
  const canvas=page.getByRole('region',{name:/^Topology canvas\./});
  assert.equal(await canvas.locator('.canvas-node').count(),0);
  assert.equal(requests.length,0,'page entry cannot initialize');
  await page.screenshot({path:`${out}/empty-topology.png`,fullPage:true});
  // One click starts the empty draft automatically; there is no review step.
  await start.click();
  await page.getByText('Saved revision 43.',{exact:false}).first().waitFor();
  assert.equal(requests.filter(r=>r.url.includes('/proposals')).length,0,'no review request');
  const init=requests.find(r=>r.url.endsWith('/edits')).body;
  assert.deepEqual(init.edit,{kind:'topology.initialize'});assert.equal(init.expected_revision,0);
  await start.waitFor({state:'detached'});
  assert.equal(await canvas.locator('.canvas-node').count(),0,'saved draft has no default nodes');
  empty=false;await page.getByRole('button',{name:'Reload view',exact:true}).click();
  const ppp=canvas.getByRole('button',{name:/^PPPoE access,/});
  await ppp.click();await ppp.press('Enter');
  const dialog=page.getByRole('dialog',{name:'PPPoE access',exact:true});await dialog.waitFor();
  assert.equal(await dialog.getByText('Add another',{exact:false}).count(),0,'no Add another parent field');
  const select=dialog.getByLabel('Add link from');assert.equal(await select.inputValue(),'');
  assert.equal(await select.locator('option[value="tm-node-wan-north"]').count(),0,'the current parent is not offered');
  await page.screenshot({path:`${out}/add-parent.png`,fullPage:true});
  await select.selectOption('tm-node-wan-south');
  await page.waitForFunction(()=>[...document.querySelectorAll('[role=status]')].some(s=>/Saved revision 43/.test(s.textContent)));
  assert.deepEqual(requests.filter(r=>r.url.endsWith('/edits')).at(-1).body.edit,{kind:'instance.add',source_id:'access-ppp',parent:'tm-node-wan-south',name:null});
  await dialog.getByText('WAN South · 2 subscribers',{exact:false}).waitFor({timeout:15000});
  await dialog.getByText('WAN North · 3 subscribers',{exact:false}).waitFor();
  assert.equal(await dialog.getByLabel('Add link from').count(),0,'no parent is left to add');
  assert.equal(await canvas.locator('.canvas-node').filter({hasText:'PPPoE access'}).count(),1,'one card with two lines');
  // Live ribbons show traffic, not links: the dialog above lists both links as text.
  assert.ok(await canvas.locator('path.sankey-ribbon[data-to="source:access-ppp"]').count()>=1,'the card gets its live stream');
  await page.screenshot({path:`${out}/shared-source-two-parents.png`,fullPage:true});
  assert.deepEqual(errors,[]);
  console.log('PASS: empty setup, automatic save, second WAN link on one card, original link retained.');
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
