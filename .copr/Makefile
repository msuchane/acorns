srpm:
	dnf install cargo openssl-devel -y
	cargo install cargo-rpm --profile=dev
	cargo rpm build
	mv target/release/rpmbuild/SRPMS/* $(outdir)
