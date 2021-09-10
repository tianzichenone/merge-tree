use crate::tree::{FileSystemTree, Overlay, TreeNode, WhiteoutSpec, WhiteoutType};
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
    pub fn apply_tree_by_dfs(
        &mut self,
        node: &Node<TreeNode>,
        level: u32,
        whiteout_spec: WhiteoutSpec,
    ) {
        // travel base tree by dfs and then to merge
        Self::merge_tree_dfs(
            self.base_tree.data.root_mut().get_mut(),
            0,
            node,
            level,
            whiteout_spec,
        );

        if !node.has_no_child() {
            for n in node.iter() {
                self.apply_tree_by_dfs(n, level + 1, whiteout_spec)
            }
        }
    }

    // travel base tree to
    fn merge_tree_dfs(
        base_node: &mut Node<TreeNode>,
        level: u32,
        upper_node: &Node<TreeNode>,
        upper_level: u32,
        whiteout_spec: WhiteoutSpec,
    ) {
        // case0, whether to handle root dir? we should think
        if upper_level == 0 {
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
            let whiteout_type = upper_node.data().whiteout_type(&whiteout_spec).unwrap();
            found = Self::handle_whiteout(
                base_node,
                level,
                node_name.clone(),
                upper_node,
                upper_node_name.clone(),
                upper_level,
                upper_parent_name.clone(),
                whiteout_type,
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
                Self::merge_tree_dfs(
                    n.get_mut(),
                    level + 1,
                    upper_node,
                    upper_level,
                    whiteout_spec,
                )
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
        whiteout_type: WhiteoutType,
    ) -> bool {
        if base_level + 1 == upper_level && base_node_name == upper_parent_name {
            for mut child in base_node.iter_mut() {
                //Case2.1 OCI remove
                if whiteout_type == WhiteoutType::OciRemoval
                    && upper_node_name.as_str()[OCI_WHITEOUT_PREFIX.len()..] == child.data().name
                {
                    child.detach();
                    return true;
                }
                //Case2.2 Overlayfs remove
                if whiteout_type == WhiteoutType::OverlayFsRemoval
                    && upper_node_name == child.data().name
                {
                    child.detach();
                    return true;
                }
                //Case2.3 Overlayfs opaque
                if whiteout_type == WhiteoutType::OverlayFsOpaque
                    && upper_node_name == child.data().name
                {
                    child.detach();
                    return true;
                }
            }
            //Case2.4 OCI opaque
            if whiteout_type == WhiteoutType::OciOpaque {
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
                    whiteout_type.clone(),
                ) {
                    return true;
                }
            }
        }
        false
    }

    pub fn display_base_tree(&self) {
        self.base_tree.display_file_tree()
    }
}

#[cfg(test)]
mod tests {
    use crate::build::BuildTree;
    use crate::tree::{FileSystemTree, Overlay, WhiteoutSpec};
    use std::path::PathBuf;

    #[test]
    fn test_common_merge() {
        let base_path = PathBuf::from("./file-example/example1/base-dir");
        let base_tree =
            FileSystemTree::build_from_file_system(base_path, Overlay::Lower, WhiteoutSpec::Oci)
                .unwrap();
        println!("show base tree");
        base_tree.display_file_tree();

        let upper_path = PathBuf::from("./file-example/example1/upper-dir");
        let upper_tree =
            FileSystemTree::build_from_file_system(upper_path, Overlay::None, WhiteoutSpec::Oci)
                .unwrap();
        println!("show upper tree");
        upper_tree.display_file_tree();

        let mut build = BuildTree::new(base_tree);
        build.apply_tree_by_dfs(upper_tree.data.root(), 0, WhiteoutSpec::Oci);

        println!("show merge tree");
        build.display_base_tree()
    }

    #[test]
    fn test_upper_file_to_replace_base() {
        let base_path = PathBuf::from("./file-example/example2/base-dir");
        let base_tree =
            FileSystemTree::build_from_file_system(base_path, Overlay::Lower, WhiteoutSpec::Oci)
                .unwrap();
        println!("show base tree");
        base_tree.display_file_tree();

        let upper_path = PathBuf::from("./file-example/example2/upper-dir");
        let upper_tree =
            FileSystemTree::build_from_file_system(upper_path, Overlay::None, WhiteoutSpec::Oci)
                .unwrap();
        println!("show upper tree");
        upper_tree.display_file_tree();

        let mut build = BuildTree::new(base_tree);
        build.apply_tree_by_dfs(upper_tree.data.root(), 0, WhiteoutSpec::Oci);

        println!("show merge tree");
        build.display_base_tree()
    }

    #[test]
    fn test_upper_dir_to_replace_base() {
        let base_path = PathBuf::from("./file-example/example3/base-dir");
        let base_tree =
            FileSystemTree::build_from_file_system(base_path, Overlay::Lower, WhiteoutSpec::Oci)
                .unwrap();
        println!("show base tree");
        base_tree.display_file_tree();

        let upper_path = PathBuf::from("./file-example/example3/upper-dir");
        let upper_tree =
            FileSystemTree::build_from_file_system(upper_path, Overlay::None, WhiteoutSpec::Oci)
                .unwrap();
        println!("show upper tree");
        upper_tree.display_file_tree();

        let mut build = BuildTree::new(base_tree);
        build.apply_tree_by_dfs(upper_tree.data.root(), 0, WhiteoutSpec::Oci);

        println!("show merge tree");
        build.display_base_tree()
    }

    #[test]
    fn test_oci_upper_remove() {
        let base_path = PathBuf::from("./file-example/example4/base-dir");
        let base_tree =
            FileSystemTree::build_from_file_system(base_path, Overlay::Lower, WhiteoutSpec::Oci)
                .unwrap();
        println!("show base tree");
        base_tree.display_file_tree();

        let upper_path = PathBuf::from("./file-example/example4/upper-dir");
        let upper_tree =
            FileSystemTree::build_from_file_system(upper_path, Overlay::None, WhiteoutSpec::Oci)
                .unwrap();
        println!("show upper tree");
        upper_tree.display_file_tree();

        let mut build = BuildTree::new(base_tree);
        build.apply_tree_by_dfs(upper_tree.data.root(), 0, WhiteoutSpec::Oci);

        println!("show merge tree");
        build.display_base_tree()
    }

    #[test]
    fn test_oci_upper_dir_opaque() {
        let base_path = PathBuf::from("./file-example/example5/base-dir");
        let base_tree =
            FileSystemTree::build_from_file_system(base_path, Overlay::Lower, WhiteoutSpec::Oci)
                .unwrap();
        println!("show base tree");
        base_tree.display_file_tree();

        let upper_path = PathBuf::from("./file-example/example5/upper-dir");
        let upper_tree =
            FileSystemTree::build_from_file_system(upper_path, Overlay::None, WhiteoutSpec::Oci)
                .unwrap();
        println!("show upper tree");
        upper_tree.display_file_tree();

        let mut build = BuildTree::new(base_tree);
        build.apply_tree_by_dfs(upper_tree.data.root(), 0, WhiteoutSpec::Oci);

        println!("show merge tree");
        build.display_base_tree()
    }

    #[test]
    fn test_multi_upper_dir_merge() {
        //base tree and create build
        let base_path = PathBuf::from("./file-example/example6/base-dir");
        let base_tree =
            FileSystemTree::build_from_file_system(base_path, Overlay::Lower, WhiteoutSpec::Oci)
                .unwrap();
        println!("show base tree");
        base_tree.display_file_tree();

        let mut build = BuildTree::new(base_tree);

        for i in 1..4 {
            let path = format!("./file-example/example6/upper{}-dir", i);
            let upper_path = PathBuf::from(path);
            let upper_tree = FileSystemTree::build_from_file_system(
                upper_path,
                Overlay::None,
                WhiteoutSpec::Oci,
            )
            .unwrap();
            println!("show upper tree {}", i);
            upper_tree.display_file_tree();
            build.apply_tree_by_dfs(upper_tree.data.root(), 0, WhiteoutSpec::Oci);
        }
        println!("show merge tree");
        build.display_base_tree()
    }

    #[test]
    /// Create overlay whiteout first
    fn test_overlayfs_upper_remove() {
        let base_path = PathBuf::from("./file-example/example7/base-dir");
        let base_tree = FileSystemTree::build_from_file_system(
            base_path,
            Overlay::Lower,
            WhiteoutSpec::Overlayfs,
        )
        .unwrap();
        println!("show base tree");
        base_tree.display_file_tree();

        let upper_path = PathBuf::from("./file-example/example7/upper-dir");
        let upper_tree = FileSystemTree::build_from_file_system(
            upper_path,
            Overlay::None,
            WhiteoutSpec::Overlayfs,
        )
        .unwrap();
        println!("show upper tree");
        upper_tree.display_file_tree();

        let mut build = BuildTree::new(base_tree);
        build.apply_tree_by_dfs(upper_tree.data.root(), 0, WhiteoutSpec::Overlayfs);

        println!("show merge tree");
        build.display_base_tree()
    }

    #[test]
    /// Create overlay whiteout first
    fn test_overlayfs_upper_dir_opaque() {
        let base_path = PathBuf::from("./file-example/example8/base-dir");
        let base_tree = FileSystemTree::build_from_file_system(
            base_path,
            Overlay::Lower,
            WhiteoutSpec::Overlayfs,
        )
        .unwrap();
        println!("show base tree");
        base_tree.display_file_tree();

        let upper_path = PathBuf::from("./file-example/example8/upper-dir");
        let upper_tree = FileSystemTree::build_from_file_system(
            upper_path,
            Overlay::None,
            WhiteoutSpec::Overlayfs,
        )
        .unwrap();
        println!("show upper tree");
        upper_tree.display_file_tree();

        let mut build = BuildTree::new(base_tree);
        build.apply_tree_by_dfs(upper_tree.data.root(), 0, WhiteoutSpec::Overlayfs);

        println!("show merge tree");
        build.display_base_tree()
    }
}
