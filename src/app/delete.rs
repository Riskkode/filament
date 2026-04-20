use super::App;
use crate::models::node::Node;
use crate::models::node::ManagedNodeType;
use std::collections::HashSet;

impl App {
    pub fn delete_selected(&mut self) {
        if !self.has_selection() { return; }
        
        let target = self.selected;
        let managed = self.nodes[target].managed.clone();

        // Special handling for managed subnodes
        if let Some(managed) = &managed {
            match managed {
                ManagedNodeType::TimeTag { key } => {
                    // Find the grandparent (actual node)
                    if let Some(group_idx) = self.nodes[target].parent {
                        if let Some(parent_idx) = self.nodes[group_idx].parent {
                            self.push_undo();
                            let key = key.clone();
                            self.nodes[parent_idx].times.remove(&key);
                            self.sync_managed_nodes();
                            return;
                        }
                    }
                }
                ManagedNodeType::TimeGroup => {
                    // Find the parent (actual node)
                    if let Some(parent_idx) = self.nodes[target].parent {
                        self.push_undo();
                        self.nodes[parent_idx].times.clear();
                        self.sync_managed_nodes();
                        return;
                    }
                }
            }
        }

        self.push_undo();
        let to_delete: HashSet<usize> = self.collect_subtree(target).into_iter().collect();
        self.remove_nodes(&to_delete);
    }

    /// Internal helper to remove a set of nodes and remap all indices.
    pub fn remove_nodes(&mut self, to_delete: &HashSet<usize>) {
        if to_delete.is_empty() { return; }

        let n = self.nodes.len();
        let mut remap: Vec<Option<usize>> = vec![None; n];
        let mut next = 0usize;
        for i in 0..n {
            if !to_delete.contains(&i) { remap[i] = Some(next); next += 1; }
        }

        // Clean up parent/children relationships before remapping
        for i in 0..n {
            if to_delete.contains(&i) {
                if let Some(p) = self.nodes[i].parent {
                    self.nodes[p].children.retain(|&c| !to_delete.contains(&c));
                }
            }
        }

        let mut new_nodes: Vec<Node> = Vec::new();
        for i in 0..n {
            if let Some(_new_idx) = remap[i] {
                let mut node = self.nodes[i].clone();
                node.parent   = node.parent.and_then(|p| remap[p]);
                node.children = node.children.iter().filter_map(|&c| remap[c]).collect();
                node.links    = node.links.iter().filter_map(|&l| remap[l]).collect();
                node.row      = usize::MAX; // Reset layout
                new_nodes.push(node);
            }
        }

        // Remap application state indices
        self.selected         = remap[self.selected].unwrap_or(0);
        self.last_link_origin = self.last_link_origin.and_then(|p| remap[p]);

        self.nodes = new_nodes;
    }
}
