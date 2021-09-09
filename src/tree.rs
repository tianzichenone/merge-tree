use crate::build::{OCI_WHITEOUT_OPAQUE, OCI_WHITEOUT_PREFIX, OVERLAYFS_WHITEOUT_OPAQUE};
use nix::sys::stat;
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::fs::Metadata;
use std::io;
use std::os::linux::fs::MetadataExt;
use std::path::PathBuf;
use trees::{Node, Tree};
use xattr;

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq)]
pub enum Overlay {
    None,
    Lower,
    UpperAddition,
    UpperOpaque,
    UpperRemove,
}

#[derive(PartialEq, Clone, Copy)]
pub enum WhiteoutSpec {
    /// https://github.com/opencontainers/image-spec/blob/master/layer.md#whiteouts
    Oci,
    /// "whiteouts and opaque directories" in https://www.kernel.org/doc/Documentation/filesystems/overlayfs.txt
    Overlayfs,
}

#[derive(Clone, Debug, PartialEq)]
pub enum WhiteoutType {
    OciOpaque,
    OciRemoval,
    OverlayFsOpaque,
    OverlayFsRemoval,
}

pub type XattrValue = Vec<u8>;

#[derive(Clone, Default)]
pub struct XAttrs {
    pairs: HashMap<OsString, XattrValue>,
}

impl XAttrs {
    pub fn new() -> Self {
        XAttrs {
            pairs: HashMap::new(),
        }
    }

    pub fn get(&self, key: &OsString) -> Option<&XattrValue> {
        self.pairs.get(key)
    }

    pub fn add(&mut self, key: OsString, value: XattrValue) {
        self.pairs.insert(key, value);
    }
}

// file system tree node
pub struct TreeNode {
    pub name: String,
    pub meta: Metadata,
    pub overlay: Overlay,
    pub xattrs: XAttrs,
}

impl TreeNode {
    pub fn new(name: String, meta: Metadata, overlay: Overlay) -> Self {
        TreeNode {
            name,
            meta,
            overlay,
            xattrs: XAttrs::new(),
        }
    }

    #[allow(dead_code)]
    pub fn is_directory(&self) -> bool {
        self.meta.is_dir()
    }

    #[allow(dead_code)]
    pub fn is_general_file(&self) -> bool {
        self.meta.is_file()
    }

    #[allow(dead_code)]
    pub fn is_whiteout(&self) -> bool {
        self.overlay == Overlay::UpperRemove || self.overlay == Overlay::UpperOpaque
    }

    #[allow(dead_code)]
    pub fn is_remove(&self) -> bool {
        self.overlay == Overlay::UpperRemove
    }

    #[allow(dead_code)]
    pub fn is_opaque(&self) -> bool {
        self.overlay == Overlay::UpperOpaque
    }

    pub fn is_overlayfs_whiteout(&self, spec: &WhiteoutSpec) -> bool {
        if *spec != WhiteoutSpec::Overlayfs {
            return false;
        }
        (self.meta.st_mode() & libc::S_IFMT == libc::S_IFCHR)
            && stat::major(self.meta.st_rdev()) == 0
            && stat::minor(self.meta.st_rdev()) == 0
    }

    pub fn is_overlayfs_opaque(&self, spec: &WhiteoutSpec) -> bool {
        if *spec != WhiteoutSpec::Overlayfs {
            return false;
        }

        // A directory is made opaque by setting the xattr
        // "trusted.overlay.opaque" to "y".
        if let Some(v) = self.xattrs.get(&OsString::from(OVERLAYFS_WHITEOUT_OPAQUE)) {
            if let Ok(v) = std::str::from_utf8(v.as_slice()) {
                return v == "y";
            }
        }
        false
    }

    pub fn build_node_xattrs(&mut self, path: PathBuf) -> io::Result<()> {
        let mut xattrs = xattr::list(path.clone()).unwrap().peekable();
        if xattrs.peek().is_none() {
            return Ok(());
        }
        for attr_key in xattrs {
            let value = xattr::get(path.clone(), &attr_key)?;
            self.xattrs.add(attr_key, value.unwrap_or_default())
        }
        return Ok(());
    }

    pub fn build_node_overlay(&mut self, whiteout_spec: WhiteoutSpec) {
        if whiteout_spec == WhiteoutSpec::Oci {
            if self.name == OCI_WHITEOUT_OPAQUE {
                println!("handle oci opaque");
                self.overlay = Overlay::UpperOpaque;
            } else if self.name.starts_with(OCI_WHITEOUT_PREFIX) {
                println!("handle oci whiteout");
                self.overlay = Overlay::UpperRemove;
            }
            return;
        }

        if whiteout_spec == WhiteoutSpec::Overlayfs {
            if self.is_overlayfs_whiteout(&whiteout_spec) {
                println!("handle overlayfs whiteout");
                self.overlay = Overlay::UpperRemove;
            } else if self.is_directory() && self.is_overlayfs_opaque(&whiteout_spec) {
                println!("handle overlayfs opaque");
                self.overlay = Overlay::UpperOpaque;
            }
        }
    }

    /// Get real whiteout type by spec, oci or overlayfs
    pub fn whiteout_type(&self, spec: &WhiteoutSpec) -> Option<WhiteoutType> {
        if self.overlay == Overlay::Lower {
            return None;
        }

        match spec {
            WhiteoutSpec::Oci => {
                if self.name == OCI_WHITEOUT_OPAQUE {
                    return Some(WhiteoutType::OciOpaque);
                } else if self.name.starts_with(OCI_WHITEOUT_PREFIX) {
                    return Some(WhiteoutType::OciRemoval);
                }
            }
            WhiteoutSpec::Overlayfs => {
                if self.is_overlayfs_whiteout(spec) {
                    return Some(WhiteoutType::OverlayFsRemoval);
                } else if self.is_overlayfs_opaque(spec) {
                    return Some(WhiteoutType::OverlayFsOpaque);
                }
            }
        }

        None
    }
}

pub struct FileSystemTree {
    pub data: Tree<TreeNode>,
}

impl FileSystemTree {
    pub fn build_from_file_system(
        path: PathBuf,
        overlay: Overlay,
        whiteout_spec: WhiteoutSpec,
    ) -> io::Result<FileSystemTree> {
        // Got metadata
        let meta = fs::metadata(path.clone())?;
        // Root dir replace /
        let mut node = TreeNode::new("/".to_string(), meta, overlay);
        // Build node xattrs
        node.build_node_xattrs(path.clone())?;
        let mut data = Tree::new(node);
        Self::build_file_system_subtree(&mut data, path.clone(), overlay, whiteout_spec)?;
        Ok(FileSystemTree { data })
    }

    pub fn build_file_system_subtree(
        root: &mut Tree<TreeNode>,
        path: PathBuf,
        overlay: Overlay,
        whiteout_spec: WhiteoutSpec,
    ) -> io::Result<()> {
        if path.is_dir() {
            for entry in fs::read_dir(path.clone())? {
                //1 go entry path and filename
                let entry = entry?;
                let entry_path = entry.path();
                let metadata = fs::metadata(entry_path.clone())?;
                let file_name = entry_path.file_name().unwrap().to_str().unwrap();
                //2. create node
                let mut node = TreeNode::new(String::from(file_name), metadata, overlay);
                // 2.1 build node xattr
                node.build_node_xattrs(entry_path.clone())?;
                // 2.2 build node whiteout
                if overlay != Overlay::Lower {
                    node.build_node_overlay(whiteout_spec);
                }
                let mut new_tree = Tree::new(node);

                if entry_path.is_dir() {
                    let _ = Self::build_file_system_subtree(
                        &mut new_tree,
                        entry_path,
                        overlay,
                        whiteout_spec,
                    );
                }
                //3. push back to root
                root.push_back(new_tree);
            }
        }
        Ok(())
    }

    pub fn display_file_tree(&self) {
        let level = 0;
        println!("{}", self.data.root().data().name);
        for node in self.data.iter() {
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
