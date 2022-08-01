srpm:
	dnf install cargo -y
	cargo install cargo-rpm openssl-devel
	cargo rpm build
	mv target/release/rpmbuild/SRPMS/* $(outdir)
