use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct MergeTreeOpt {
    /// Base dir path
    #[structopt(short = "b", long = "base-path", required = true)]
    pub base_path: PathBuf,

    /// Upper dir path list, can multiply
    #[structopt(short = "u", long = "upper-path", required = true)]
    pub upper_path_list: Vec<PathBuf>,

    /// Whiteout type
    // 0 is OCI, 1 is Overlayfs, default is 0
    #[structopt(short = "w", long = "whiteout-type", default_value = "0")]
    pub whiteout: u32,
}
