use crate::tree::{FileSystemTree, Overlay, TreeNode};
use log;
use trees::{Node, Tree};

pub const OCI_WHITEOUT_PREFIX: &str = ".wh.";
pub const OCI_WHITEOUT_OPAQUE: &str = ".wh..wh..opq";
pub const OVERLAYFS_WHITEOUT_OPAQUE: &str = "trusted.overlay.opaque";

pub struct BuildTree {
    pub base_tree: FileSystemTree,
}

impl BuildTree {
    pub fn new(base_tree: FileSystemTree) -> Self {
        BuildTree { base_tree }
    }

    // apply upper tree to base tree
    pub fn apply_tree_by_dfs(&mut self, node: &Node<TreeNode>, level: u32) {
        // travel base tree by dfs and then to merge
        Self::merge_tree_dfs(self.base_tree.data.root_mut().get_mut(), 0, node, level);

        if !node.has_no_child() {
            for n in node.iter() {
                self.apply_tree_by_dfs(n, level + 1)
            }
        }
    }

    // travel base tree to
    fn merge_tree_dfs(
        base_node: &mut Node<TreeNode>,
        level: u32,
        upper_node: &Node<TreeNode>,
        upper_level: u32,
    ) {
        // skip upper addition
        if base_node.data().overlay == Overlay::UpperAddition {
            log::trace!("travel upper node, skip {}", base_node.data().name)
        }

        // case0, whether to handle root dir? we should think
        if upper_level <= 0 {
            return;
        }

        // case1, don't search if base path recursive depth out of upper level
        if level > upper_level {
            return;
        }

        let mut parent_name: String = String::from("");
        let node_name: String = base_node.data().name.to_string();
        match base_node.parent() {
            None => {}
            Some(parent) => parent_name = parent.data().name.to_string(),
        }

        let mut upper_parent_name: String = String::from("");
        let upper_node_name = upper_node.data().name.to_string();
        match upper_node.parent() {
            None => {}
            Some(upper_parent) => upper_parent_name = upper_parent.data().name.to_string(),
        }

        //case2, whiteout handle
        if upper_node.data().is_whiteout() {
            let found;
            found = Self::handle_whiteout(
                base_node,
                level,
                node_name.clone(),
                upper_node,
                upper_node_name.clone(),
                upper_level,
                upper_parent_name.clone(),
            );
            if found {
                println!(
                    "already handle whiteout for upper node {}, return",
                    upper_node_name
                );
                return;
            }
        }

        // case3, lower level is the parent of upper node
        if level + 1 == upper_level && node_name == upper_parent_name {
            // addition, node is upper
            log::trace!(
                "node_parent_name{}, node_name {}, upper_parent_name {}, upper_node_name {}",
                parent_name,
                node_name,
                upper_parent_name,
                upper_node_name
            );

            // travel child of base node to find upper node
            let mut found = false;
            for mut node_child in base_node.iter_mut() {
                if node_child.data().name == upper_node_name {
                    // case3.1 upper is gen file，handle modification
                    found = true;
                    if !upper_node.data().is_directory() {
                        node_child.detach();
                        found = false;
                        break;
                    }
                }
            }

            // case3.2 handle addition
            if !found {
                let new_tree = Tree::new(TreeNode::new(
                    upper_node.data().name.to_string(),
                    upper_node.data().meta.clone(),
                    Overlay::UpperAddition,
                ));
                base_node.push_back(new_tree)
            }
        }

        // current node is dir, should recurse
        if !base_node.has_no_child() {
            for n in base_node.iter_mut() {
                Self::merge_tree_dfs(n.get_mut(), level + 1, upper_node, upper_level)
            }
        }
    }

    fn handle_whiteout(
        base_node: &mut Node<TreeNode>,
        base_level: u32,
        base_node_name: String,
        upper_node: &Node<TreeNode>,
        upper_node_name: String,
        upper_level: u32,
        upper_parent_name: String,
    ) -> bool {
        if base_level + 1 == upper_level && base_node_name == upper_parent_name {
            for mut child in base_node.iter_mut() {
                //Case2.1  OCI .wh.
                println!(
                    "got upper origin name {},got child name {}",
                    upper_node_name.as_str()[OCI_WHITEOUT_PREFIX.len()..].to_string(),
                    child.data().name,
                );
                // origin name equal base file name
                if upper_node_name.as_str()[OCI_WHITEOUT_PREFIX.len()..].to_string()
                    == child.data().name
                    && upper_node.data().is_remove()
                {
                    child.detach();
                    return true;
                }
            }
            //Case3.2 OCI opaque
            if upper_node.data().is_opaque() {
                base_node.detach();
                return true;
            }
        }
        // current node is dir, should recurse
        if !base_node.has_no_child() {
            for n in base_node.iter_mut() {
                if Self::handle_whiteout(
                    n.get_mut(),
                    base_level + 1,
                    base_node_name.clone(),
                    upper_node,
                    upper_node_name.clone(),
                    upper_level,
                    upper_parent_name.clone(),
                ) {
                    return true;
                }
            }
        }
        false
    }

    pub fn display_file_tree(&self) {
        let level = 0;
        println!("{}", self.base_tree.data.root().data().name);
        for node in self.base_tree.data.iter() {
            Self::display_file_tree_inner(node, level);
        }
    }

    fn display_file_tree_inner(node: &Node<TreeNode>, level: usize) {
        let mut prefix = "├──".to_string();
        let mut i = 0;
        while i < level {
            prefix = format!("│  {}", prefix);
            i += 1;
        }

        println!("{} {}", prefix, node.data().name,);

        if !node.has_no_child() {
            for child in node.iter() {
                Self::display_file_tree_inner(child, level + 1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::build::BuildTree;
    use crate::tree::{FileSystemTree, Overlay};
    use std::path::PathBuf;

    #[test]
    fn test_common_merge() {
        let base_path = PathBuf::from(
            "/Users/tianzichen/CLionProjects/merge-tree/file-example/example1/base-dir",
        );
        let base_tree = FileSystemTree::build_from_file_system(base_path, Overlay::Lower).unwrap();

        let upper_path = PathBuf::from(
            "/Users/tianzichen/CLionProjects/merge-tree/file-example/example1/upper-dir",
        );
        let upper_tree = FileSystemTree::build_from_file_system(upper_path, Overlay::None).unwrap();

        let mut build = BuildTree::new(base_tree);
        build.apply_tree_by_dfs(upper_tree.data.root(), 0);
        build.display_file_tree()
    }

    #[test]
    fn test_upper_file_to_replace_base() {
        let base_path = PathBuf::from(
            "/Users/tianzichen/CLionProjects/merge-tree/file-example/example2/base-dir",
        );
        let base_tree = FileSystemTree::build_from_file_system(base_path, Overlay::Lower).unwrap();

        let upper_path = PathBuf::from(
            "/Users/tianzichen/CLionProjects/merge-tree/file-example/example2/upper-dir",
        );
        let upper_tree = FileSystemTree::build_from_file_system(upper_path, Overlay::None).unwrap();

        let mut build = BuildTree::new(base_tree);
        build.apply_tree_by_dfs(upper_tree.data.root(), 0);
        build.display_file_tree()
    }

    #[test]
    fn test_upper_dir_to_replace_base() {
        let base_path = PathBuf::from(
            "/Users/tianzichen/CLionProjects/merge-tree/file-example/example3/base-dir",
        );
        let base_tree = FileSystemTree::build_from_file_system(base_path, Overlay::Lower).unwrap();

        let upper_path = PathBuf::from(
            "/Users/tianzichen/CLionProjects/merge-tree/file-example/example3/upper-dir",
        );
        let upper_tree = FileSystemTree::build_from_file_system(upper_path, Overlay::None).unwrap();

        let mut build = BuildTree::new(base_tree);
        build.apply_tree_by_dfs(upper_tree.data.root(), 0);
        build.display_file_tree()
    }

    #[test]
    fn test_oci_upper_remove_base() {
        let base_path = PathBuf::from(
            "/Users/tianzichen/CLionProjects/merge-tree/file-example/example4/base-dir",
        );
        let base_tree = FileSystemTree::build_from_file_system(base_path, Overlay::Lower).unwrap();

        let upper_path = PathBuf::from(
            "/Users/tianzichen/CLionProjects/merge-tree/file-example/example4/upper-dir",
        );
        let upper_tree = FileSystemTree::build_from_file_system(upper_path, Overlay::None).unwrap();

        let mut build = BuildTree::new(base_tree);
        build.apply_tree_by_dfs(upper_tree.data.root(), 0);
        build.display_file_tree()
    }

    #[test]
    fn test_oci_upper_dir_opaque_base() {
        let base_path = PathBuf::from(
            "/Users/tianzichen/CLionProjects/merge-tree/file-example/example5/base-dir",
        );
        let base_tree = FileSystemTree::build_from_file_system(base_path, Overlay::Lower).unwrap();

        let upper_path = PathBuf::from(
            "/Users/tianzichen/CLionProjects/merge-tree/file-example/example5/upper-dir",
        );
        let upper_tree = FileSystemTree::build_from_file_system(upper_path, Overlay::None).unwrap();

        let mut build = BuildTree::new(base_tree);
        build.apply_tree_by_dfs(upper_tree.data.root(), 0);
        build.display_file_tree()
    }

    #[test]
    fn test_multi_upper_dir_merge() {
        //base tree and create build
        let base_path = PathBuf::from(
            "/Users/tianzichen/CLionProjects/merge-tree/file-example/example6/base-dir",
        );
        let base_tree = FileSystemTree::build_from_file_system(base_path, Overlay::Lower).unwrap();

        let mut build = BuildTree::new(base_tree);

        for i in 1..4 {
            let path = format!(
                "/Users/tianzichen/CLionProjects/merge-tree/file-example/example6/upper{}-dir",
                i
            );
            let upper_path = PathBuf::from(path);
            let upper_tree =
                FileSystemTree::build_from_file_system(upper_path, Overlay::None).unwrap();
            build.apply_tree_by_dfs(upper_tree.data.root(), 0);
        }
        build.display_file_tree()
    }
}
