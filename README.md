[![Crate](https://img.shields.io/crates/v/mft2bodyfile.svg)](https://crates.io/crates/mft2bodyfile)

# mft2bodyfile
parses an $MFT file to bodyfile

## Installation

```shell
cargo install mft2bodyfile
```

## Usage

```shell
mft2bodyfile \$MFT >mft.bodyfile
mactime -b mft.bodyfile -d >mft.csv
```
# Why did i start this project?

Until now, me and my team used `analyze_mft.py` to extract data from the `$MFT`, when we got triage data from a customer. Unfortunately, `analyze_mft.py` has some disadvantages:
* python2 is required
* either the `$STANDARD_INFORMATION` or the `$FILE_NAME` attribute used to generate the timestamps, bot not both of them at the same time. This alrways required us to merge both outputs, which is a little bit messy
* from time to time we had problems parsing the `$MFT`

So, at first we started to work on `analyze_mft.py` to fix our complaints, but we soon got stuck when we discovered one additional disadvantage:
* If a file has its `$FILE_NAME` attribute not stored in its base entry, but in some nonbase entry which is refered by an `$ATTRIBUTE_LIST` attribute, then this file is not shown in the bodyfile.

You might think that "non-base MFT entries do not have the `$FILE_NAME` and `$STANDARD_INFORMATION` attributes in them", as Brian Carrier has stated in his great book. But we found that this does happen. Further investigation showed us that nearly all fast and simple tools have the same problem. So this was the last bit that led us write a tool for our own.

# What are the advantages of this tool?

* way more faster than `analyze_mft.py`
* all files are displayed, even if they don't have a '$FILENAME' (Really??? Files can have no filename? Yes, they can. See below)

# What are the limits of this tool?

Consider the following situation: You have a file, which has a lot of attributes. The list of attributes is so long, that it cannot be stored in an `$MFT` entry. So, the `$ATTRIBUTE_LIST` attribute is stored as a nonresident attribute, outside the `$MFT`. At the moment, `mft2bodyfile` is not able to find the corresponding `$MFT` entries, and will generate a filename.

Can we fix this? Yes, we can. If we detect such a situation, we can search the `$MFT` entries which refer to our base entry, and use those to find a `$FILE_NAME` attribute. 