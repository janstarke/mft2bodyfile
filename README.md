# mft2bodyfile
parses an $MFT file to bodyfile

## Installation

```shell
cargo install mft2bodyfile
```

## Usage

```shell
mft2bodyfile \$MFT >mft.bodyfile
mactime -b bodyfile -d >mft.csv
```
