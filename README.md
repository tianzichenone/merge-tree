# Merge Tree library

used to learn how overlayfs metadata merge, include handing docker oci and overlayfs whiteout file

## cargo test overlayfs whiteout
mkdir -p ./example7/upper-dir/a
mkdir -p ./example7/upper-dir/b

cd ./example7/upper-dir/a
mknod file1 c 0 0

cd ./example7/upper-dir/b
mknod file2 c 0 0

## cargo test overlayfs opaque

mkdir -p ./example8/upper-dir/a
mkdir -p ./example8/upper-dir/b
mkdir -p ./example8/upper-dir/c

cd ./example8/upper-dir/a
mknod file1 c 0 0

cd ./example8/upper-dir/b
mknod file2 c 0 0

cd ./example8/upper-dir/
setfattr -n "trusted.overlay.opaque" -v y c/