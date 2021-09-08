mod build;
mod tree;

use crate::build::BuildTree;
use crate::tree::{FileSystemTree, Overlay, WhiteoutSpec};
use std::path::PathBuf;

///               basedir
///                 /
///    /a                     /b
///   /a/file1 /a/file2     /b/c/ /b/file3
///
///
///               upperdir
///               /
/// /a         b    /c
/// /a/file1 /a/file-upperdir       /c/file4
///
///
///              mergedir
/// /a

fn main() {
    let base_path =
        PathBuf::from("/Users/tianzichen/CLionProjects/merge-tree/file-example/example2/base-dir");
    let base_tree =
        FileSystemTree::build_from_file_system(base_path, Overlay::Lower, WhiteoutSpec::Oci)
            .unwrap();

    let upper_path =
        PathBuf::from("/Users/tianzichen/CLionProjects/merge-tree/file-example/example2/upper-dir");
    let upper_tree =
        FileSystemTree::build_from_file_system(upper_path, Overlay::None, WhiteoutSpec::Oci)
            .unwrap();

    let mut build = BuildTree::new(base_tree);
    build.apply_tree_by_dfs(upper_tree.data.root(), 0, WhiteoutSpec::Oci);
    build.display_file_tree()
}
