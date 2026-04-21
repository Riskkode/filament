use crate::models::node::Node;
use chrono::{Local, Utc};
use chrono_english::{parse_date_string, Dialect};

pub fn evaluate(query: &str, node: &Node) -> bool {
    if query.trim().is_empty() { return true; }
    
    // Split by AND for now
    let parts: Vec<&str> = query.split(" AND ").collect();
    for part in parts {
        if !evaluate_single(part.trim(), node) {
            return false;
        }
    }
    true
}

fn evaluate_single(part: &str, node: &Node) -> bool {
    if part.contains(':') {
        // Tag match
        let subparts: Vec<&str> = part.splitn(2, ':').collect();
        let key = subparts[0].trim();
        let val = subparts[1].trim();
        return node.tags.get(key).map(|v| v == val).unwrap_or(false);
    }
    
    if part.contains('<') || part.contains('>') {
        // Time match
        let op = if part.contains('<') { '<' } else { '>' };
        let subparts: Vec<&str> = part.splitn(2, op).collect();
        let key = subparts[0].trim();
        let time_str = subparts[1].trim();
        
        if let Some(tag) = node.times.get(key) {
            let now = Local::now();
            if let Ok(target_dt) = parse_date_string(time_str, now, Dialect::Uk) {
                let target_ts = target_dt.with_timezone(&Utc).timestamp();
                if op == '<' {
                    return tag.timestamp < target_ts;
                } else {
                    return tag.timestamp > target_ts;
                }
            }
        }
        return false;
    }
    
    // Label search
    node.label.to_lowercase().contains(&part.to_lowercase())
}
