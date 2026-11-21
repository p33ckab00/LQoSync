// Isolated browser fixture only. Never use production account credentials.
// The fixture answers list edits on its synthetic inventory; no config file or router is touched.
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const assert=require('node:assert/strict');
const fs=require('node:fs');
(async()=>{
  const browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH,headless:true,args:['--no-sandbox']});
  const page=await browser.newPage({viewport:{width:1440,height:1000},colorScheme:'dark'});
  const errors=[];page.on('pageerror',e=>errors.push(e.message));
  const sent=[];page.on('request',r=>{if(r.method()==='PATCH')sent.push(JSON.parse(r.postData()).edit);});
  const out=process.env.ROUTERS_EVIDENCE_DIR||'/tmp/lqosync-routers-evidence';fs.mkdirSync(out,{recursive:true});
  await page.goto('http://127.0.0.1:19581/routers/lq3-00000000000000a1?tab=sources');
  await page.getByLabel('Username',{exact:true}).fill('browser-test');
  await page.getByLabel('Password',{exact:true}).fill(process.env.BROWSER_TEST_PASSWORD);
  await page.getByRole('button',{name:'Sign in',exact:true}).click();
  const lists=page.locator('.router-lists');
  await lists.getByRole('heading',{name:'Exclusions'}).waitFor();
  assert.match(await lists.innerText(),/PPP profiles to skip\s*staff[\s\S]*IP addresses and networks to skip\s*100\.64\.9\.0\/24/);
  await lists.getByText('02:00:00:00:0A:01').waitFor();
  const notice=page.locator('.saved-notice');

  // Exclusions: a bad IP line is refused before the save; only the changed list is sent.
  await page.getByRole('button',{name:'Edit exclusions'}).click();
  const ips=page.getByLabel('IP addresses and networks to skip');
  await ips.fill('100.64.9.0/24\n10.0.0.0/33');
  await page.getByRole('button',{name:'Save exclusions'}).click();
  await page.getByText('These lines are not IPv4 addresses or networks: 10.0.0.0/33.').waitFor();
  assert.equal(sent.length,0,'no request for a bad list');
  await ips.fill('100.64.9.0/24\n10.20.0.0/16\n\n10.20.0.0/16');
  await page.getByRole('button',{name:'Save exclusions'}).click();
  await notice.getByText('Exclusions saved.').waitFor();
  await notice.getByText('The service loaded the change.').waitFor({timeout:15000});
  assert.deepEqual(sent[0].exclude_ips,['100.64.9.0/24','10.20.0.0/16'],'trimmed, without repeats');
  assert.equal(sent[0].exclude_ppp_profiles,null,'an unchanged list is not sent');
  await lists.getByText('100.64.9.0/24, 10.20.0.0/16').waitFor();

  // Client overrides: a repeated MAC is a row error; a new row gets focus; a removal saves.
  await page.getByRole('button',{name:'Edit client overrides'}).click();
  await page.getByRole('button',{name:'Add override'}).click();
  await page.waitForFunction(()=>/^client-mac-/.test(document.activeElement.id),{},{timeout:3000});
  const row=page.locator('.client-row').nth(1);
  await row.getByLabel('MAC address').fill('02-00-00-00-0a-01');
  await row.getByLabel('Download Mbps').fill('30');
  await row.getByLabel('Upload Mbps').fill('10');
  await page.getByRole('button',{name:'Save client overrides'}).click();
  await page.getByRole('alert').getByText('Row 2: This MAC address is in the list twice.').waitFor();
  assert.equal(await row.getByLabel('MAC address').getAttribute('aria-invalid'),'true');
  await row.getByLabel('MAC address').fill('02-00-00-00-0a-02');
  await row.getByLabel('Name (optional)').fill('Shop');
  await lists.screenshot({path:`${out}/lists-clients-edit.png`});
  await page.getByRole('button',{name:'Save client overrides'}).click();
  await notice.getByText('Client overrides saved.').waitFor();
  await lists.getByText('02:00:00:00:0A:02').waitFor({timeout:15000});
  assert.equal(sent.at(-1).dhcp_clients[1].mac,'02:00:00:00:0A:02','the MAC is normalized before the save');
  await page.getByRole('button',{name:'Edit client overrides'}).click();
  await page.getByRole('button',{name:'Remove the override 02:00:00:00:0A:01'}).click();
  await page.getByRole('button',{name:'Save client overrides'}).click();
  await page.waitForFunction(()=>!document.querySelector('.router-lists').innerText.includes('02:00:00:00:0A:01'),{},{timeout:15000});
  assert.equal(sent.at(-1).dhcp_clients.length,1);
  await lists.screenshot({path:`${out}/lists.png`});

  // DHCP server settings: the source order stays; the mode rule is a row error; a new server saves.
  await lists.getByText('lease_comment, settings').waitFor();
  await page.getByRole('button',{name:'Edit DHCP server settings'}).click();
  assert.equal(await page.getByLabel('Mode of a server without its own settings').inputValue(),'per_site');
  await page.getByRole('button',{name:'Add server settings'}).click();
  await page.waitForFunction(()=>/^server-name-/.test(document.activeElement.id),{},{timeout:3000});
  const server=page.locator('.server-row').nth(1);
  await server.getByLabel('Server name').fill('GUEST');
  await server.getByLabel('Speed sources, in order').fill('lease_comment');
  await page.getByRole('button',{name:'Save DHCP server settings'}).click();
  await page.getByRole('alert').getByText('Row 2: The per_site mode cannot use lease_comment.').waitFor();
  await server.getByLabel('Mode').selectOption('per_plan');
  await server.getByLabel('Speed sources, in order').fill('settings, lease_comment');
  await server.getByLabel('Plan down Mbps').fill('25');
  await page.getByLabel('Mode of a server without its own settings').selectOption('per_plan');
  await page.getByRole('button',{name:'Save DHCP server settings'}).click();
  await notice.getByText('DHCP server settings saved.').waitFor();
  await lists.getByText('settings, lease_comment').waitFor({timeout:15000});
  const last=sent.at(-1);
  assert.equal(last.dhcp_server_mode,'per_plan');
  assert.deepEqual(last.dhcp_servers.map(s=>[s.name,s.mode,s.speed_sources]),[['LAN','per_plan',['lease_comment','settings']],['GUEST','per_plan',['settings','lease_comment']]],'the order of each list stays');
  await lists.screenshot({path:`${out}/lists-servers.png`});

  await page.setViewportSize({width:320,height:900});
  await page.getByRole('button',{name:'Edit client overrides'}).click();
  assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),'fits 320px');
  assert.deepEqual(errors,[]);
  await browser.close();
  console.log('PASS: exclusions with an IP check and only the changed list sent, client overrides with a repeated-MAC row error, focus on a new row, normalized MACs, a removal, DHCP server settings with the source order and the mode rule, and 320px layout.');
})().catch(e=>{console.error(e);process.exit(1);});
