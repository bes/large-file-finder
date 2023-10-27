# large-file-finder
A small utility in rust to find the largest files and directories for a given path

## Install

You need to have rust & cargo installed - See [rustup.rs](https://rustup.rs/).

```shell
cargo install --git https://github.com/bes/large-file-finder
```

## Usage

Given a directory structure like this

```
lff% eza -lT
      .
      ├── big_file
  23G │  └── large_file.dat
      └── small_file
  71M    └── small_file.dat
```

We can run `lff` to find the largest directories

```
lff% lff .
21 GiB   d .
21 GiB   d ./big_file
21 GiB   f ./big_file/large_file.dat
Total size: 21 GiB
Largest child: 21 GiB
```

## Options

```
lff% lff --help
Large file finder 1.0

USAGE:
    lff [OPTIONS] <DIRECTORY>

FLAGS:
    -h, --help
            Prints help information

    -V, --version
            Prints version information


OPTIONS:
    -p, --percent <percent>
            Show all files and directories that are larger than X% of the largest found file. [env: PERCENT=]  [default:
            50]

ARGS:
    <DIRECTORY>
            The directory to scan for files and directories
```
