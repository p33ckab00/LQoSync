# WAN routing lists: setup and cutover

Status: procedure for the first setup. Version: 2026-11-22.

LQoSync chooses the WAN of each client. The routing list writer puts each client address in one list on the core router: `via-WAN1`, `via-WAN2`, and so on. Mangle rules on the core router send the traffic of each list to its WAN. Thus shaping (LibreQoS) and routing (MikroTik) use the same WAN.

## What the writer changes

The writer changes only these items:

- Entries in lists whose name starts with `via-`.
- Only entries whose comment starts with `lqosync: `.
- Only these operations: add an entry, remove an entry, change the address of an entry.

The writer does not change mangle rules, routes, other lists, or entries without the `lqosync: ` comment. You make the mangle rules one time, with this procedure.

## Before you start

- You need `full` access on the core router (FSE-CORE) and root access on the LQoSync host.
- The Topology Manager shapes the subscribers. The canvas shows the WAN nodes WAN1 and WAN2.
- Write down the current mangle rules: `/ip firewall mangle print detail where action=mark-routing`.
- Keep the current lists `WAN1-CUSTOMERS` and `WAN2-CUSTOMERS`. They stay as a safety net.

## Step 1: Router users

Do these commands on FSE-CORE.

1. Give the read user the `test` policy. The WAN monitor needs it for `/ping`:

   ```
   /user group set lqosync-read policy=read,api,test
   ```

2. Make a group and a user for the writer. Set `address` to the address of the LQoSync host, so the user cannot log in from other hosts:

   ```
   /user group add name=lqosync-write policy=read,write,api
   /user add name=lqosync-writer group=lqosync-write address=<LQoSync host address>/32 password="<new password>"
   ```

Check:

```
/user print detail where name~"lqosync"
/user group print detail where name~"lqosync"
```

`lqosync` is in `lqosync-read` with `read,api,test`. `lqosync-writer` is in `lqosync-write`.

> **CAUTION:** RouterOS cannot limit a user with `write` to one menu. Only LQoSync limits the writer to the `via-` lists. Keep the writer password secret, and keep the `address` limit.

## Step 2: LQoSync configuration

Do these steps on the LQoSync host, as root.

1. Write the writer password to a file. The file must have mode 0600 and owner root:

   ```sh
   install -m 0600 -o root -g root /dev/null /etc/lqosync3/secrets/FSE-CORE-writer.password
   printf '%s\n' '<new password>' > /etc/lqosync3/secrets/FSE-CORE-writer.password
   ```

2. In `/etc/lqosync3/lqosync3.toml`, add these two lines to the `[[routers]]` entry of FSE-CORE:

   ```toml
   writer_username = "lqosync-writer"
   writer_password_file = "/etc/lqosync3/secrets/FSE-CORE-writer.password"
   ```

3. Keep the writer off for now. The default is off:

   ```toml
   [wan_writer]
   enabled = false
   ```

4. Check the configuration, then restart the service. The writer user applies only after a restart:

   ```sh
   lqosync3 check --config /etc/lqosync3/lqosync3.toml
   systemctl restart lqosync3
   ```

Check: `lqosync3 check` shows `router FSE-CORE: writer password file is readable` and no error.

## Step 3: List names

Open **Topology**, then the canvas. Open each WAN node. The **Address list** field shows the list name, for example `via-WAN1`. An empty field uses `via-` and the WAN name.

Check: each WAN has a different list name.

## Step 4: Adopt the current routing

This step keeps the WAN that each client uses today. Do it one time, before you put the new mangle rules first.

1. Open **Topology settings** (the tool dock on the right).
2. Under **Routing lists**, click **Adopt current routing**.
3. Click **Review change**. The review lists the clients that move to a node below their current WAN.
4. Click **Save change**.

Check: the Assignments tab shows each client below the WAN of its old range list. Pinned clients stay on their pin.

## Step 5: Turn on the writer

1. Set `enabled = true` in `[wan_writer]`.
2. Reload the configuration: `systemctl reload lqosync3`.
3. Wait for one cycle (about one minute).

Check, in **Topology settings** under **Routing lists**: the text starts with `Written`. On FSE-CORE:

```
/ip firewall address-list print count-only where list~"^via-"
/ip firewall address-list print where list~"^via-" and comment~"^lqosync"
```

The count is the number of client addresses. Each client address is in one `via-` list.

If the text starts with `Held`, the writer would remove too many entries at one time. Read the reason. The same removal goes through in the next cycle. If the text starts with `Not written`, read the error. Shaping keeps the previous version until the write succeeds.

## Step 6: Put the via- rules first

Add one rule for each WAN above the current rules. Copy the conditions of the current rules exactly:

```
/ip firewall mangle add chain=prerouting action=mark-routing new-routing-mark=wan1-rt passthrough=no \
    src-address-list=via-WAN1 dst-address-list=!LOCAL-NETS dst-address-type=!local \
    in-interface-list=DOWNSTREAM comment="PBR: via-WAN1 -> wan1-rt (LQoSync)" place-before=0
/ip firewall mangle add chain=prerouting action=mark-routing new-routing-mark=wan2-rt passthrough=no \
    src-address-list=via-WAN2 dst-address-list=!LOCAL-NETS dst-address-type=!local \
    in-interface-list=DOWNSTREAM comment="PBR: via-WAN2 -> wan2-rt (LQoSync)" place-before=0
```

Check:

```
/ip firewall mangle print stats where action=mark-routing
```

The packet counters of the two new rules increase. The counters of the `WAN1-CUSTOMERS` and `WAN2-CUSTOMERS` rules stay almost the same.

## Step 7: Keep the old rules as a safety net

Keep the `WAN1-CUSTOMERS` and `WAN2-CUSTOMERS` rules below the new rules. They route a client during the short time after a reconnect, before the writer adds its new address.

Remove the old rules only when their packet counters stay at zero for one week. Reset the counters first: `/ip firewall mangle reset-counters [find comment~"PBR: WAN"]`.

## Rollback

1. On FSE-CORE, disable the new rules. The old rules route as before:

   ```
   /ip firewall mangle disable [find comment~"LQoSync"]
   ```

2. On the LQoSync host, set `enabled = false` in `[wan_writer]` and run `systemctl reload lqosync3`.
3. The `via-` lists stay on the router. They do not route traffic while their rules are disabled. To remove them:

   ```
   /ip firewall address-list remove [find list~"^via-" and comment~"^lqosync"]
   ```

## Limits

- IPv6 client addresses get no list entry.
- A change of the writer user or the router address applies after a service restart.
- A WAN failover in LQoSync moves the list entries. The fallback routes on the router (`distance=2`) also stay active.
