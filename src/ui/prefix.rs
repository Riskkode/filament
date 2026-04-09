use crate::models::node::Node;

pub fn box_prefix(trail: &[bool]) -> String {
    if trail.is_empty() { return String::new(); }
    let mut s = String::new();
    for &last in &trail[..trail.len() - 1] {
        s.push_str(if last { "  " } else { "│ " });
    }
    s.push_str(if *trail.last().unwrap() { "╰─" } else { "├─" });
    s
}

pub fn compute_insert_trail(nodes: &[Node], parent: usize) -> Vec<bool> {
    let mut path: Vec<bool> = Vec::new();
    let mut cur = parent;
    while let Some(p) = nodes[cur].parent {
        path.push(nodes[p].children.last() == Some(&cur));
        cur = p;
    }
    path.reverse();
    path.push(true);
    path
}

pub fn compute_is_last(order: &[(usize, usize)]) -> Vec<bool> {
    let n = order.len();
    let mut is_last = vec![false; n];
    for i in 0..n {
        let depth = order[i].1;
        if depth == 0 { is_last[i] = true; continue; }
        let next = order[i + 1..].iter().find(|&&(_, d)| d <= depth);
        is_last[i] = match next { None => true, Some(&(_, d)) => d < depth };
    }
    is_last
}
