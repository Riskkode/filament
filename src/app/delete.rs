use super::App;
use crate::models::node::Node;

impl App {
    pub fn delete_selected(&mut self) {
        if !self.has_selection() { return; }
        let target      = self.selected;
        let orig_parent = self.nodes[target].parent;
        let to_delete   = self.collect_subtree(target);

        if let Some(p) = orig_parent {
            self.nodes[p].children.retain(|&c| c != target);
        }

        let n = self.nodes.len();
        let mut remap: Vec<Option<usize>> = vec![None; n];
        let mut next = 0usize;
        for i in 0..n {
            if !to_delete.contains(&i) { remap[i] = Some(next); next += 1; }
        }

        let mut new_nodes: Vec<Node> = Vec::new();
        for i in 0..n {
            if remap[i].is_some() {
                let node = &self.nodes[i];
                new_nodes.push(Node {
                    label:     node.label.clone(),
                    parent:    node.parent.and_then(|p| remap[p]),
                    children:  node.children.iter().filter_map(|&c| remap[c]).collect(),
                    links:     node.links.iter().filter_map(|&l| remap[l]).collect(),
                    collapsed: node.collapsed,
                    row:       usize::MAX,
                    world_x:     node.world_x,
                    world_y:     node.world_y,
                    world_x_end: 0,
                });
            }
        }

        self.nodes = new_nodes;
        self.selected = orig_parent.and_then(|p| remap[p]).unwrap_or(0);
    }
}
