# Cizrna (CoRN 4)

[![Rust tests](https://github.com/msuchane/cizrna/actions/workflows/rust-tests.yml/badge.svg)](https://github.com/msuchane/cizrna/actions/workflows/rust-tests.yml)

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

* On any system that has the Docker or Podman container platform, you can use Cizrna as a container.

    On Fedora, RHEL, and CentOS, replace `docker` with `podman` in the following commands.

    1. Download the image:

        ```
        $ docker pull quay.io/msuchane/cizrna
        ```
    
    2. Configure a command alias. Save this line in your shell configuration file, such as in the `~/.bashrc` file:

        ```
        alias cizrna="docker run -it -e BZ_API_KEY -e JIRA_API_KEY -v .:/mnt/cizrna:Z msuchane/cizrna cizrna"
        ```
    
    3. Open a new terminal to reload the shell configuration.

* On any system, including different Linux distributions and macOS, you can build Cizrna from source:

    1. Clone this Git repository.

    2. Install the Rust toolchain se described at <https://rustup.rs/>.

    3. Build and install Cizrna from the cloned repository:

        ```
        $ cargo install --path ./cizrna
        ```

        If the build fails due to a missing dependency, install the missing dependency on your system and start the build again.


## Generating release notes

_TODO_: Provide information for platforms other than Fedora and RHEL, and explain setting up the release notes project.

1. Switch to an existing directory with a release notes project configuration.

2. Generate the release notes:

    ```
    $ cizrna build
    ```

3. Build an HTML preview:

    ```
    $ asciidoctor --safe -vn main.adoc
    ```

4. Open the HTML preview:

    ```
    $ gio open main.html
    ```