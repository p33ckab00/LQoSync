//! The router inventory from the service cache `routers.json`. Reading it starts no router I/O.
use std::path::Path;
use web_contract::routers::{CACHE, RouterInventory, VERSION};

/// The inventory part of a broker reply stays under this size. The reply limit is 64 KiB.
pub const BUDGET: usize = 60 * 1024;
/// More routers than an ISP inventory screen needs; the byte budget usually binds first.
pub const MAX_ROUTERS: usize = 200;

pub fn load(root: &Path) -> Option<RouterInventory> {
    let inventory: RouterInventory = crate::overview::read(root, CACHE)?;
    (inventory.version == VERSION && inventory.updated > 0).then(|| bound(inventory))
}

/// Keep the routers in config order and drop from the end until the reply fits.
pub fn bound(mut inventory: RouterInventory) -> RouterInventory {
    if inventory.routers.len() > MAX_ROUTERS {
        inventory.routers.truncate(MAX_ROUTERS);
        inventory.truncated = true;
    }
    while serde_json::to_vec(&inventory).map_or(usize::MAX, |b| b.len()) > BUDGET {
        if inventory.routers.pop().is_none() {
            break;
        }
        inventory.truncated = true;
    }
    inventory
}
