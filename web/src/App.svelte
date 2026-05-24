<script>
  import { onMount } from 'svelte';

  const tabs = [
    { id: 'overview', label: 'Overview' },
    { id: 'config', label: 'Config JSON' },
    { id: 'services', label: 'Services' },
    { id: 'files', label: 'Generated Files' }
  ];

  let user = null;
  let authError = '';
  let loading = true;
  let loginForm = { username: 'admin', password: 'adminpass' };
  let activeTab = 'overview';
  let dashboard = null;
  let configValue = '';
  let configBaseline = '';
  let configDirty = false;
  let configMessage = '';
  let actionResult = null;
  let files = { csv: '', network: '' };
  let busy = false;

  async function api(path, options = {}) {
    const response = await fetch(path, {
      credentials: 'same-origin',
      headers: {
        'Content-Type': 'application/json',
        ...(options.headers || {})
      },
      ...options
    });

    const text = await response.text();
    let payload = {};
    try {
      payload = text ? JSON.parse(text) : {};
    } catch (_error) {
      payload = { ok: response.ok, raw: text };
    }

    if (response.status === 401) {
      user = null;
      throw new Error(payload.error || 'Authentication required.');
    }

    if (!response.ok) {
      throw new Error(payload.error || payload.code || `Request failed: ${response.status}`);
    }

    return payload;
  }

  async function loadSession() {
    try {
      const result = await api('/api/auth/me', { headers: {} });
      user = result.user;
      authError = '';
      await refreshAll();
    } catch (_error) {
      user = null;
    } finally {
      loading = false;
    }
  }

  async function refreshAll() {
    if (!user) return;
    busy = true;
    try {
      const dashboardResult = await api('/api/dashboard');
      dashboard = dashboardResult;
      if (roleAtLeast('operator')) {
        await loadConfig();
      } else {
        configValue = '';
        configBaseline = '';
        configDirty = false;
        configMessage = '';
      }
      if (activeTab === 'files') {
        await loadFiles();
      }
    } finally {
      busy = false;
    }
  }

  async function loadConfig() {
    const configResult = await api('/api/config');
    configValue = JSON.stringify(configResult.config, null, 2);
    configBaseline = configValue;
    configDirty = false;
    configMessage = '';
  }

  async function loadFiles() {
    const [csvResponse, networkResponse] = await Promise.all([
      fetch('/api/generated/csv', { credentials: 'same-origin' }),
      fetch('/api/generated/network', { credentials: 'same-origin' })
    ]);
    files = {
      csv: await csvResponse.text(),
      network: await networkResponse.text()
    };
  }

  async function login() {
    loading = true;
    authError = '';
    try {
      const result = await api('/api/auth/login', {
        method: 'POST',
        body: JSON.stringify(loginForm)
      });
      user = result.user;
      await refreshAll();
    } catch (error) {
      authError = error.message;
    } finally {
      loading = false;
    }
  }

  async function logout() {
    await api('/api/auth/logout', { method: 'POST' });
    user = null;
    dashboard = null;
    configValue = '';
    files = { csv: '', network: '' };
    actionResult = null;
  }

  async function saveConfig() {
    configMessage = '';
    try {
      const parsed = JSON.parse(configValue);
      await api('/api/config', {
        method: 'PUT',
        body: JSON.stringify({ config: parsed })
      });
      configDirty = false;
      configBaseline = JSON.stringify(parsed, null, 2);
      configMessage = 'Config saved through the Rust backend.';
      await refreshAll();
    } catch (error) {
      configMessage = error.message;
    }
  }

  async function runAction(path, label) {
    actionResult = { label, loading: true };
    try {
      const result = await api(path, { method: 'POST', body: '{}' });
      actionResult = { label, loading: false, result };
      await refreshAll();
    } catch (error) {
      actionResult = { label, loading: false, error: error.message };
    }
  }

  async function restartService(unit) {
    await runAction(`/api/services/${unit}/restart`, `Restart ${unit}`);
  }

  function activateTab(id) {
    activeTab = id;
    if (id === 'files' && user) {
      loadFiles();
    }
    if (id === 'config' && user && roleAtLeast('operator')) {
      loadConfig();
    }
  }

  function roleAtLeast(role) {
    const ranks = { viewer: 10, operator: 20, admin: 30, owner: 40 };
    return (ranks[user?.role] || 0) >= (ranks[role] || 0);
  }

  $: if (dashboard !== null) {
    configDirty = configValue !== configBaseline;
  }

  onMount(loadSession);
</script>

{#if loading}
  <main class="shell loading-shell">
    <section class="loading-card">
      <div class="eyebrow">Rust Runtime</div>
      <h1>Warming up the operator console</h1>
      <p>Bootstrapping the Svelte frontend against the Rust backend service.</p>
    </section>
  </main>
{:else if !user}
  <main class="shell auth-shell">
    <section class="login-card">
      <div class="eyebrow">Svelte + Rust</div>
      <h1>LQoSync Operator Console</h1>
      <p>The Python backend runtime is retired. Sign in to the Rust service.</p>
      <label>
        <span>Username</span>
        <input bind:value={loginForm.username} autocomplete="username" />
      </label>
      <label>
        <span>Password</span>
        <input bind:value={loginForm.password} type="password" autocomplete="current-password" />
      </label>
      <button class="primary" on:click={login}>Enter Console</button>
      {#if authError}
        <p class="error">{authError}</p>
      {/if}
    </section>
  </main>
{:else}
  <main class="shell">
    <aside class="rail">
      <div class="brand">
        <div class="eyebrow">Rust Backend</div>
        <h1>LQoSync</h1>
        <p>Operator console served directly from the Rust runtime.</p>
      </div>
      <nav>
        {#each tabs as tab}
          <button class:active={activeTab === tab.id} on:click={() => activateTab(tab.id)}>
            {tab.label}
          </button>
        {/each}
      </nav>
      <div class="session">
        <div>
          <strong>{user.username}</strong>
          <span>{user.role}</span>
        </div>
        <button class="ghost" on:click={logout}>Sign out</button>
      </div>
    </aside>

    <section class="workspace">
      <header class="hero">
        <div>
          <div class="eyebrow">Runtime Authority</div>
          <h2>Rust-only operations</h2>
          <p>Scheduler, RouterOS transport, file writes, apply execution, and this web service now run without a Python backend.</p>
        </div>
        <div class="hero-actions">
          {#if roleAtLeast('operator')}
            <button class="ghost" on:click={() => runAction('/api/actions/dry-run', 'Dry Run')}>Dry run</button>
          {/if}
          {#if roleAtLeast('admin')}
            <button class="primary" on:click={() => runAction('/api/actions/run', 'Manual Run')}>Run now</button>
          {/if}
        </div>
      </header>

      {#if activeTab === 'overview'}
        <section class="grid">
          <article class="card stat-card">
            <span>Routers</span>
            <strong>{dashboard?.summary?.routers ?? 0}</strong>
            <small>Configured in live config.json</small>
          </article>
          <article class="card stat-card">
            <span>Scheduler</span>
            <strong>{dashboard?.summary?.scheduler_enabled ? 'Enabled' : 'Disabled'}</strong>
            <small>Rust authority loop</small>
          </article>
          <article class="card stat-card">
            <span>Full Rust</span>
            <strong>{dashboard?.summary?.full_rust_backend_authority ? 'Active' : 'Off'}</strong>
            <small>Mutation and runtime authority</small>
          </article>
          <article class="card stat-card">
            <span>Audit events</span>
            <strong>{dashboard?.summary?.audit_events ?? 0}</strong>
            <small>Recent JSONL entries</small>
          </article>
        </section>

        <section class="split">
          <article class="card">
            <div class="card-head">
              <h3>Service status</h3>
              <button class="ghost" on:click={refreshAll}>Refresh</button>
            </div>
            <div class="service-list">
              {#each dashboard?.services ?? [] as service}
                <div class="service-row">
                  <div>
                    <strong>{service.unit}</strong>
                    <small>{service.description || service.sub}</small>
                  </div>
                  <span class:good={service.active === 'active'} class:warn={service.active !== 'active'}>{service.active}</span>
                </div>
              {/each}
            </div>
          </article>

          <article class="card">
            <div class="card-head">
              <h3>Rust diagnostics</h3>
            </div>
            <pre>{JSON.stringify(dashboard?.scheduler_status ?? {}, null, 2)}</pre>
          </article>
        </section>

        <section class="split">
          <article class="card">
            <div class="card-head">
              <h3>Recent audit</h3>
            </div>
            <div class="audit-list">
              {#each dashboard?.recent_audit ?? [] as event}
                <div class="audit-row">
                  <strong>{event.action}</strong>
                  <span>{event.actor}</span>
                </div>
              {/each}
            </div>
          </article>

          <article class="card">
            <div class="card-head">
              <h3>Last action</h3>
            </div>
            {#if actionResult}
              <pre>{JSON.stringify(actionResult, null, 2)}</pre>
            {:else}
              <p class="muted">Dry run and manual run results land here.</p>
            {/if}
          </article>
        </section>
      {/if}

      {#if activeTab === 'config'}
        <section class="card tall-card">
          <div class="card-head">
            <div>
              <h3>config.json</h3>
              <p>Edit the live config directly through the Rust backend.</p>
            </div>
            <div class="hero-actions">
              {#if roleAtLeast('operator')}
                <button class="ghost" on:click={loadConfig}>Reload</button>
              {/if}
              {#if roleAtLeast('admin')}
                <button class="primary" on:click={saveConfig}>Save</button>
              {/if}
            </div>
          </div>
          {#if roleAtLeast('operator')}
            <textarea bind:value={configValue} on:input={() => (configDirty = true)} spellcheck="false"></textarea>
            {#if configMessage}
              <p class="muted">{configMessage}</p>
            {/if}
          {:else}
            <p class="muted">Your role can view runtime status, but config editing is reserved for operator and admin accounts.</p>
          {/if}
        </section>
      {/if}

      {#if activeTab === 'services'}
        <section class="card">
          <div class="card-head">
            <h3>Service control</h3>
            <button class="ghost" on:click={refreshAll}>Refresh</button>
          </div>
          <table>
            <thead>
              <tr>
                <th>Unit</th>
                <th>Load</th>
                <th>Active</th>
                <th>Sub</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {#each dashboard?.services ?? [] as service}
                <tr>
                  <td>{service.unit}</td>
                  <td>{service.load}</td>
                  <td>{service.active}</td>
                  <td>{service.sub}</td>
                  <td>
                    {#if roleAtLeast('admin')}
                      <button class="ghost" on:click={() => restartService(service.unit)}>Restart</button>
                    {/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </section>
      {/if}

      {#if activeTab === 'files'}
        <section class="split">
          <article class="card tall-card">
            <div class="card-head">
              <h3>ShapedDevices.csv</h3>
              <button class="ghost" on:click={loadFiles}>Reload</button>
            </div>
            <pre>{files.csv}</pre>
          </article>
          <article class="card tall-card">
            <div class="card-head">
              <h3>network.json</h3>
            </div>
            <pre>{files.network}</pre>
          </article>
        </section>

        <section class="card">
          <div class="card-head">
            <h3>Backups</h3>
          </div>
          <div class="backup-list">
            {#each dashboard?.backups ?? [] as backup}
              <div class="backup-row">
                <strong>{backup.id}</strong>
                <small>{backup.metadata?.reason || 'snapshot'}</small>
              </div>
            {/each}
          </div>
        </section>
      {/if}
    </section>
  </main>
{/if}

<style>
  :global(body) {
    margin: 0;
    min-height: 100vh;
    font-family: "Avenir Next", "Gill Sans", "Trebuchet MS", sans-serif;
    background:
      radial-gradient(circle at top left, rgba(255, 190, 92, 0.18), transparent 34%),
      radial-gradient(circle at top right, rgba(67, 211, 186, 0.16), transparent 28%),
      linear-gradient(160deg, #111318 0%, #171f26 50%, #0d1014 100%);
    color: #eff3f5;
  }

  .shell {
    min-height: 100vh;
    display: grid;
    grid-template-columns: 280px 1fr;
  }

  .auth-shell,
  .loading-shell {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 32px;
  }

  .login-card,
  .loading-card {
    width: min(480px, 100%);
    padding: 32px;
    border-radius: 28px;
    background: rgba(12, 18, 23, 0.84);
    border: 1px solid rgba(255, 255, 255, 0.08);
    box-shadow: 0 28px 80px rgba(0, 0, 0, 0.35);
    backdrop-filter: blur(16px);
  }

  .rail {
    padding: 28px;
    border-right: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(8, 12, 16, 0.54);
    backdrop-filter: blur(14px);
    display: flex;
    flex-direction: column;
    gap: 24px;
  }

  .brand h1,
  .login-card h1,
  .hero h2 {
    margin: 0;
    font-family: "Palatino Linotype", "Book Antiqua", Georgia, serif;
    letter-spacing: 0.02em;
  }

  .eyebrow {
    text-transform: uppercase;
    letter-spacing: 0.18em;
    font-size: 0.72rem;
    color: #9fd8cb;
    margin-bottom: 10px;
  }

  nav {
    display: grid;
    gap: 10px;
  }

  nav button,
  button {
    border: 0;
    cursor: pointer;
    transition: transform 140ms ease, background 140ms ease, opacity 140ms ease;
  }

  nav button {
    text-align: left;
    border-radius: 16px;
    padding: 14px 16px;
    color: #d9e5ea;
    background: rgba(255, 255, 255, 0.04);
  }

  nav button.active {
    background: linear-gradient(135deg, rgba(255, 184, 88, 0.32), rgba(67, 211, 186, 0.18));
    color: white;
  }

  .workspace {
    padding: 28px;
    display: grid;
    gap: 20px;
  }

  .hero,
  .card,
  .session {
    border-radius: 24px;
    background: rgba(14, 19, 25, 0.8);
    border: 1px solid rgba(255, 255, 255, 0.08);
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.2);
    backdrop-filter: blur(18px);
  }

  .hero {
    padding: 26px 28px;
    display: flex;
    justify-content: space-between;
    gap: 18px;
    align-items: end;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 16px;
  }

  .split {
    display: grid;
    grid-template-columns: 1.1fr 0.9fr;
    gap: 16px;
  }

  .card {
    padding: 22px;
  }

  .tall-card {
    min-height: 420px;
  }

  .stat-card strong {
    display: block;
    font-size: 2rem;
    margin: 12px 0 8px;
    color: #ffcc7f;
  }

  .card-head {
    display: flex;
    justify-content: space-between;
    align-items: start;
    gap: 16px;
    margin-bottom: 14px;
  }

  .hero-actions,
  .session {
    display: flex;
    gap: 10px;
    align-items: center;
  }

  .session {
    margin-top: auto;
    padding: 14px 16px;
    justify-content: space-between;
  }

  .session span,
  .muted,
  small,
  p {
    color: #9aaab3;
  }

  label {
    display: grid;
    gap: 8px;
    margin-top: 14px;
  }

  input,
  textarea,
  pre {
    width: 100%;
    box-sizing: border-box;
    border-radius: 18px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(4, 8, 12, 0.72);
    color: #f4f7f8;
    font: inherit;
  }

  input,
  textarea {
    padding: 14px 16px;
  }

  textarea {
    min-height: 520px;
    font-family: "Iosevka", "Fira Code", "SFMono-Regular", monospace;
    resize: vertical;
  }

  pre {
    margin: 0;
    padding: 16px;
    overflow: auto;
    font-size: 0.84rem;
    line-height: 1.45;
    font-family: "Iosevka", "Fira Code", "SFMono-Regular", monospace;
  }

  .primary,
  .ghost {
    padding: 12px 18px;
    border-radius: 999px;
    font-weight: 700;
  }

  .primary {
    background: linear-gradient(135deg, #ffc15e, #f68b45);
    color: #16181b;
  }

  .ghost {
    background: rgba(255, 255, 255, 0.06);
    color: #edf2f3;
  }

  .service-list,
  .audit-list,
  .backup-list {
    display: grid;
    gap: 12px;
  }

  .service-row,
  .audit-row,
  .backup-row {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 14px;
    border-radius: 16px;
    background: rgba(255, 255, 255, 0.04);
  }

  .good {
    color: #62e0bc;
  }

  .warn {
    color: #ffb568;
  }

  .error {
    color: #ff8e8e;
    margin-top: 14px;
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  th,
  td {
    text-align: left;
    padding: 12px 10px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  @media (max-width: 1080px) {
    .shell {
      grid-template-columns: 1fr;
    }

    .rail {
      border-right: 0;
      border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    }

    .grid,
    .split {
      grid-template-columns: 1fr;
    }

    .hero {
      flex-direction: column;
      align-items: start;
    }
  }
</style>
