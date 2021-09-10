mod build;
mod option;
mod tree;
use structopt::StructOpt;

use crate::build::BuildTree;
use crate::option::MergeTreeOpt;
use crate::tree::{FileSystemTree, Overlay, WhiteoutSpec};

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
    let opt = MergeTreeOpt::from_args();
    let mut whiteout_spec = WhiteoutSpec::Oci;
    if opt.whiteout == 1 {
        whiteout_spec = WhiteoutSpec::Overlayfs
    }

    // 1. build base tree
    let base_tree =
        FileSystemTree::build_from_file_system(opt.base_path, Overlay::Lower, whiteout_spec)
            .unwrap();

    // 2. create tree build
    let mut build = BuildTree::new(base_tree);

    // 3. build upper tree by upper path and then to merge
    for upper_path in opt.upper_path_list {
        let upper_tree =
            FileSystemTree::build_from_file_system(upper_path, Overlay::None, whiteout_spec)
                .unwrap();

        build.apply_tree_by_dfs(upper_tree.data.root(), 0, whiteout_spec);
    }

    // 4. display merge tree
    build.display_base_tree()
}
