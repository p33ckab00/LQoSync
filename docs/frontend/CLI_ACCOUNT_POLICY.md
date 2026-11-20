# CLI-only superadmin accounts

Status: owner direction, 2026-11-20. Implementation tasks: F04 and F14.
This policy supersedes the Users picture and the earlier viewer/operator/admin proposal.
The application has not implemented these commands yet.

## Ownership and web boundary

Manage account usernames, passwords, and access state only through the local privileged CLI.
Require operating-system root access for every account administration command.
A web `superadmin` role does not grant shell access or permission to execute CLI commands through HTTP.

Allow multiple superadmin accounts. Do not impose a fixed account-count limit.
Each account needs a unique username and stable internal ID.
All initial accounts use the `superadmin` role. Viewer and operator roles are outside the initial scope.

The browser supplies sign-in and sign-out only for account access.
Do not implement signup, invitations, account lists, account editing, password changes, or password reset through HTTP.
Remove Users from the navigation and route registry.
Keep the generated Users image only as a withdrawn historical concept.

The sign-in response can identify the current account without exposing other accounts.
Operational audit reads must not become an account directory or expose authentication secrets.

## Proposed CLI commands

The binary name follows the existing planned local bootstrap tool.

```text
sudo lqosync3-web-admin user create <username>
sudo lqosync3-web-admin user list
sudo lqosync3-web-admin user show <username>
sudo lqosync3-web-admin user rename <username> <new-username>
sudo lqosync3-web-admin user passwd <username>
sudo lqosync3-web-admin user disable <username>
sudo lqosync3-web-admin user enable <username>
sudo lqosync3-web-admin user delete <username>
```

`create` and `passwd` prompt for a password twice without terminal echo.
Do not accept passwords in command arguments or environment variables.
Do not print passwords, password hashes, or session tokens.
Noninteractive secret input needs a separately reviewed file-descriptor contract before implementation.

`list` and `show` return safe account metadata only.
`delete` requires explicit confirmation. It never deletes operational audit evidence.
Reject duplicate usernames and invalid names before any write.
Keep the stable account ID after a username change.

Create the first superadmin locally before web sign-in can succeed.
Do not ship default credentials or a public bootstrap endpoint.
Root can create another superadmin for recovery without knowing an existing password.
Reject deletion or disable of the final active superadmin.
Serialize concurrent account changes so two commands cannot bypass that protection.

## Storage and sessions

The root-owned `web-accounts.db` contains account IDs, names, password hashes, access state, and credential versions.
Only the local CLI and privileged broker access this store.
The unprivileged gateway cannot modify it or open it directly.
Keep database sidecar files and backups under the same permission boundary.

The gateway stores opaque web sessions separately in `web-auth.db`.
The broker verifies credentials through typed private IPC with bounded input and authentication rate limits.
Never forward authentication secrets to the synchronization service or record them in logs.

Each session binds the stable account ID and credential version.
The broker checks current account state/version for every protected request.
If verification is unavailable, reject the request. Do not trust an old cached authorization result.

Increase the credential version after password change, rename, disable, enable, or deletion.
Reject old sessions on their next protected request. Enabling an account does not restore old sessions.
Already dispatched operational actions keep their durable outcome record and are not silently reversed.

Store account changes and their audit entry in one transaction.
Use a reviewed password hash implementation and a fresh salt for each password.
Record the local operator UID, target account ID, action, timestamp, and outcome without secret contents.

## Required proof

- Create and use several independent superadmin accounts.

- Reject account changes from a non-root process.

- Reject all account CRUD HTTP routes, including direct requests from a signed-in superadmin.

- Reject old sessions after account or password changes, also across gateway restart.

- Preserve the final active superadmin during concurrent delete/disable requests.

- Keep credentials out of arguments, terminal output, logs, audit records, and browser responses.

- Preserve account and audit consistency after disk-full errors, interrupted writes, and failed transactions.
