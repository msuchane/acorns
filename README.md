# Cizrna (CoRN 4)

Generate an AsciiDoc release notes document from tracking tickets

## Installing Cizrna

* On Fedora, CentOS Stream, or RHEL, use [the Copr repository](https://copr.fedorainfracloud.org/coprs/mareksu/cizrna/):

    1. Enable the repository:
    
        ```
        # dnf copr enable mareksu/cizrna
        ```
    
    2. Install the `cizrna` package:

        ```
        # dnf install cizrna
        ```

* On other systems, including different Linux distributions and macOS, build Cizrna from source:

    1. Clone this Git repository.

    2. Install the Rust toolchain se described at <https://rustup.rs/>.

    3. Build and install Cizrna from the cloned repository:

        ```
        $ cargo install --path ./cizrna
        ```

        If the build fails due to a missing dependency, install the missing dependency on your system and start the build again.
