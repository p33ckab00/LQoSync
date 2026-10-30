# Synthetic console browser checks

Use an existing Node.js and Playwright installation. These checks do not install packages.
They save configuration on `127.0.0.1:19304`. Keep the live console on its existing port.

From `v3/`, create an isolated data directory and start the rebuilt console:

```bash
preview_data=$(mktemp -d /tmp/lqosync-browser.XXXXXX)
cp tests/browser/dynamic-fixture.toml "$preview_data/settings.toml"
LQOSYNC3_DATA="$preview_data" LQOSYNC3_BIND=127.0.0.1:19304 cargo run --locked --bin lqosync-v3
```

In another terminal, run the checks. Set `LQOSYNC_PLAYWRIGHT` to the existing module path if Node.js cannot resolve `playwright`.

```bash
node tests/browser/configuration.cjs
node tests/browser/dynamic-topology.cjs
```

The first script checks readable body fonts and three/two/one card columns at five widths.
It checks 12 routes at desktop and mobile widths, dialog layer order, keyboard focus, draft retention, validation errors, rejected tokens, and retries.
It also checks a 720px layout, equivalent to a 1440px browser at 200 percent zoom.
The second script checks promotion, demotion, removal with a replacement, impact labels, policy edits, and independent root creation and removal.
The fixture contains one root, one branch, one policy, two routing WANs, and a loopback placeholder router. Automatic collection is paused.
Stop the preview after the checks. Start with a fresh fixture before another hierarchy run.
