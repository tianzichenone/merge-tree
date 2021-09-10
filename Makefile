BUILDER_IMG:=ccr.ccs.tencentyun.com/tcr-cloud/clux-muslrust:1.53.0

docker_test:
	docker run --rm -v ${PWD}:/root/app/src \
	-v ${HOME}/.cargo/git:/root/.cargo/git \
	-v ${HOME}/.cargo/registry:/root/.cargo/registry \
	${BUILDER_IMG} cargo test --all

test:
	cargo test --all

docker_check:
	docker run --rm -v ${PWD}:/root/app/src \
	-v ${HOME}/.cargo/git:/root/.cargo/git \
	-v ${HOME}/.cargo/registry:/root/.cargo/registry \
	${BUILDER_IMG} cargo check

docker_clippy:
	docker run --rm -v ${PWD}:/root/app/src \
	-v ${HOME}/.cargo/git:/root/.cargo/git \
	-v ${HOME}/.cargo/registry:/root/.cargo/registry \
	${BUILDER_IMG} cargo clippy -- -D warnings

docker_static_release:
	docker run --rm -v ${PWD}:/root/app/src \
	-v ${HOME}/.cargo/git:/root/.cargo/git \
	-v ${HOME}/.cargo/registry:/root/.cargo/registry \
	${BUILDER_IMG} cargo build --release

bash:
	docker run -it --rm -v ${PWD}:/root/app/src \
		-v ${HOME}/.cargo/git:/root/.cargo/git \
		-v ${HOME}/.cargo/registry:/root/.cargo/registry \
		${BUILDER_IMG} bash

gen_overlayfs_test:
	@echo Generate overlayfs whiteout
	@zsh -c "mkdir -p ./example7/upper-dir/a  && mkdir -p ./example7/upper-dir/b"
	@zsh -c "cd ./example7/upper-dir/a && mknod file1 c 0 0"
	@zsh -c "cd ./example7/upper-dir/b && mknod file2 c 0 0"
	@echo Generate overlayfs opaque
	@zsh -c "mkdir -p ./example8/upper-dir/a  && mkdir -p ./example8/upper-dir/b && mkdir -p ./example8/upper-dir/c"
	@zsh -c "cd ./example7/upper-dir/a && mknod file1 c 0 0"
	@zsh -c "cd ./example8/upper-dir/ && setfattr -n "trusted.overlay.opaque" -v y c/"
