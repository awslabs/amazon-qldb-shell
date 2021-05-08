# Hacking

## For Developers

### Installing Rust

Go to [Rustup](https://rustup.rs/) and follow the instructions to install rust. 

Make sure you have the necessary dependencies installed with the following commands:

* MacOS:
    ```
    brew install cmake
    ```
* Ubuntu/Mint Linux:
    ```
    sudo apt install libssl-dev cmake clang
    ```
* CentOs/Fedora Linux:
    ```
    sudo yum install -y openssl-devel cmake clang
    ```
* Debian Linux:
    ```
    sudo apt install libssl-dev pkg-config cmake clang
    ```

If you would like to modify the source code and work on the driver, after cloning the repo, simply run:

```
cargo test
```

To run the shell, use:

```
cargo run -- --ledger <ledger-name>
```

And to run the test using the release optimized compilation, use:

```
cargo run --release -- --ledger <ledger-name>
```

Note that the `"--"` indicates to Cargo that what follows are args for the shell.

To install the shell on your system, use:

```
cargo install --path .
```

### Windows installation

> Note: these instructions are relevant because of our dependency on ion-c-sys
> which uses cmake/clang to build the underlying C library.

CMake can be downloaded from https://cmake.org/download/. First, find the
*latest release* and pick the installer for your platform. At the time of
writing, the latest 64 bit installer was:
https://github.com/Kitware/CMake/releases/download/v3.19.6/cmake-3.19.6-win64-x64.msi.

When running the installer, take note at the prompt on `PATH`. The default
option is to not modify the PATH, which means that after installing CMake it
*will not be found* by `cargo`. If you have a single-user machine, you can
select "for all users".

After installing CMake, you must make a new terminal to receive the updated
PATH.

If you do not have Visual Studio, you can install it by downloading the
community edition from https://visualstudio.microsoft.com/downloads/. After
setting it up, you can install clang by going to
https://releases.llvm.org/download.html and selecting a pre-built binary for
your architecture. Once both these steps are done, you should be able to open a
"native tools command prompt".


