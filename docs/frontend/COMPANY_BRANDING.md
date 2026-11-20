# Company branding in global Settings

Status: owner direction. Implementation task: F19. No application code exists for this feature yet.

## Placement and behavior

Add `Company branding` to global Settings because the company identity applies across the application.
Allow a superadmin to enter a company name and upload, replace, or remove a company logo.
Show a preview and require an explicit save. Discard keeps the current saved branding.

Show the company name and logo alongside the accepted LQoSync wordmark and logo mark.
Keep the LQoSync superscript `v3`. Company branding supplements the product identity and does not replace it.
Apply the saved company identity to the sign-in branding area and shared main application header.

If no company identity is configured, show the standard LQoSync branding.
If only a company name is configured, show the name without an empty logo box.
If a saved logo cannot load, keep the company name and LQoSync identity visible.
Do not alter the sign-in form, account policy, or feature configuration ownership.

Keep both identities legible in Light, Dark, and System modes.
Use proportional image sizing without cropping or distortion.
On narrow screens, wrap or compact the identity group without hiding the theme selector or main actions.
Use visible company text as the accessible name. Avoid duplicate screen-reader announcements of the adjacent logo.

## Field and asset rules

Treat the company name as plain text, with a proposed limit of 120 Unicode characters.
Escape it when rendered. Do not accept HTML, scripts, or styling in the name.
Allow an empty name to restore the default identity.

Initially accept PNG, JPEG, and WebP logo uploads only.
Reject SVG, executable content, external image URLs, and files that do not decode as an allowed image.
Use a proposed 2 MiB upload limit and a 2048×2048 decoded dimension limit.
Enforce a decoded pixel/memory ceiling before expensive image work. Do not trust MIME headers or file extensions alone.

Decode and re-encode uploads with a reviewed Rust image library under bounded concurrency.
Strip metadata and use generated content-based asset names, not supplied file paths.
Store uploads outside executable/static source directories. Serve them with a fixed safe content type and `nosniff`.
Do not request a remote logo URL from the browser or server.

## Persistence and API boundary

Store `WebBranding` in the gateway's application settings store, with its own `branding_revision`.
It is web presentation state, not a new field in the core LQoSync TOML or topology registry.
Saving branding must not reload the sync service or start a collection/publication job.

Use authenticated, CSRF-protected administration routes and expected branding revisions.
Publish only the saved company name and generated logo asset reference before sign-in.
The public branding read must not expose operational configuration, account data, file paths, or the private settings response.

| Proposed path under `/api/v1` | Purpose |
|---|---|
| `GET /branding` | Safe public `BrandingView` for sign-in and shared identity |
| `POST /settings/branding/logo-drafts` | Authenticated bounded upload; return an opaque draft asset ID and preview reference |
| `PATCH /settings/branding` | Authenticated save of name and optional draft logo, or explicit logo removal, with expected revision |

Keep the previous valid branding active until the new asset and metadata commit succeed.
Make draft IDs private, time-limited, and bound to the uploading session.
Use a short draft lifetime and bounded cleanup. Do not delete the active or rollback asset during cleanup.
Reject stale revisions without overwriting another administrator's saved identity.
An upload preview is not a saved branding result.

## Required evidence

- Check name-only, logo/name, cleared branding, and missing-logo fallbacks.

- Check all themes, sign-in, shared header, narrow screens, long names, and accessible labels.

- Reject invalid images, oversized uploads, decompression abuse, foreign draft IDs, and stale revisions.

- Check CSRF, unauthorized mutations, escaped names, fixed content types, and absence of private data in the public read.

- Check restart, failed asset writes, failed metadata commit, draft expiry, and cleanup of abandoned uploads.

- Prove that branding changes do not call the sync service or change the unchanged TUI.
