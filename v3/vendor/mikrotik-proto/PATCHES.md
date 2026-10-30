# Local completion attribute patch

Source: cached mikrotik-proto 0.1.0, from ferrohd/mikrotik-rs. Keep the upstream dual MIT/Apache-2.0 licenses.
The original parser discards completion attributes and expects the tag first.
Preserve attributes from !done and accept the tag before or after them. Reject duplicate tags and attributes.
Expose those attributes on Event::Done so migration can use the actual returned ret ID.
Existing read callers continue to match Event::Done with an ignored attribute field.
Root RouterOS protocol tests exercise this patch without real routers.

The [RouterOS API reference](https://help.mikrotik.com/docs/spaces/ROS/pages/47579160/API) describes completion replies and attribute order.
