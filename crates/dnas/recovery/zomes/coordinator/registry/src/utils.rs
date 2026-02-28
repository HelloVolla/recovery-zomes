use std::collections::HashMap;

use hdk::prelude::*;

/// Deduplicate links by tag, keeping the entry with the newest link timestamp
pub fn dedup_links_by_tag(links: Vec<Link>) -> Vec<Link> {
    let mut map: HashMap<Vec<u8>, Link> = HashMap::new();
    for link in links {
        let key = link.tag.clone().into_inner();
        map.entry(key)
            .and_modify(|existing| {
                if link.timestamp > existing.timestamp {
                    *existing = link.clone();
                }
            })
            .or_insert(link);
    }
    map.into_values().collect()
}
