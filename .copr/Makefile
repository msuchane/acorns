srpm:
	dnf install cargo openssl-devel -y
	cargo install cargo-rpm
	cargo rpm build
	mv target/release/rpmbuild/SRPMS/* $(outdir)
