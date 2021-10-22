[![Crate](https://img.shields.io/crates/v/mft2bodyfile.svg)](https://crates.io/crates/mft2bodyfile)
![Crates.io (latest)](https://img.shields.io/crates/dv/mft2bodyfile)
![Codecov](https://img.shields.io/codecov/c/github/janstarke/mft2bodyfile)

# mft2bodyfile
parses an $MFT file (and optionally the corresponding `$UsnJrnl`) to bodyfile

## Installation

```shell
cargo install mft2bodyfile
```

## Usage

```
Usage:
  mft2bodyfile [OPTIONS] MFT_FILE

parses an $MFT file to bodyfile (stdout)

Positional arguments:
  mft_file              path to $MFT

Optional arguments:
  -h,--help             Show this help message and exit
  -J,--journal JOURNAL  path to $UsnJrnl $J file (optional)
  --journal-long-flags  don't remove the USN_REASON_ prefix from the $UsnJrnl
                        reason output
```

## Example

```shell
mft2bodyfile '$MFT' -J '$UsnJrnl_$J' >mft.bodyfile
mactime -b mft.bodyfile -d >mft.csv
```
# Why did i start this project?

Until now, me and my team used `analyze_mft.py` to extract data from the `$MFT`, when we got triage data from a customer. Unfortunately, `analyze_mft.py` has some disadvantages:
* python2 is required
* either the `$STANDARD_INFORMATION` or the `$FILE_NAME` attribute used to generate the timestamps, bot not both of them at the same time. This always required us to merge both outputs, which is a little bit messy
* from time to time we had problems parsing the `$MFT`

So, at first we started to work on `analyze_mft.py` to fix our complaints, but we soon got stuck when we discovered one additional disadvantage:
* If a file has its `$FILE_NAME` attribute not stored in its base entry, but in some nonbase entry which is refered by an `$ATTRIBUTE_LIST` attribute, then this file is not shown in the bodyfile.

You might think that "non-base MFT entries do not have the `$FILE_NAME` and `$STANDARD_INFORMATION` attributes in them", as Brian Carrier has stated in his great book. But we found that this does happen. Further investigation showed us that nearly all fast and simple tools have the same problem. So this was the last bit that led us write a tool for our own.

# What are the advantages of this tool?

* way more faster than `analyze_mft.py`
* correct handling of entries with nonbase attributes, even if they are stored in nonpersistent `$ATTRIBUTE_LIST` entries.
* can combine `$MFT` and `$UsnJrnl` data

# Which information are shown?

## `$UsnJrnl` Records

The file `$UsnJrnl` (abbreviation for Update Sequence Number Journal) contains a list of entries, where every entry documents changes of metadata for a file.

The following data are shown:

|Field|Description|
|-|----|
|`$UsnJrnl`|*shown if* for every entry extracted from the `$UsnJrnl` file |
|`filename`|*shown if* the filename found in the `$MFT` is different from the filename found in `$UsnJrnl` *or* if the `$MFT` does not contain a `$FILENAME`attribute for this file|
|`parent`|*shown if* the parent reference in the `$MFT` is different from the parent reference in the `$FILENAME` attribute *or* if the `$MFT` does not contain a `$FILENAME`attribute for this file|
|`reason`| *The flags that identify reasons for changes that have accumulated in this file or directory journal record since the file or directory opened.* ([https://docs.microsoft.com/de-de/windows/win32/api/winioctl/ns-winioctl-usn_record_v2](https://docs.microsoft.com/de-de/windows/win32/api/winioctl/ns-winioctl-usn_record_v2))

### Example: a File has been renamed

```
Tue Aug 31 2021 10:48:42,16,macb,0,0,0,335695,"/Users/tmpadmin/AppData/Local/Microsoft/Edge/User Data/Default/Local Storage/leveldb/CURRENT ($UsnJrnl filename=000001.dbtmp reason=RENAME_OLD_NAME)"
```

### Example: a File has been moved to a different folder (and renamed)

```
Tue Aug 31 2021 10:49:50,0,macb,0,0,0,336826,"/Users/tmpadmin/AppData/Local/Microsoft/Edge/User Data/CertificateRevocation/6498.2021.5.1 ($UsnJrnl filename=8244_1963452067 parent='/Users/tmpadmin/AppData/Local/Temp' reason=CLOSE+FILE_CREATE)"
Tue Aug 31 2021 10:49:50,0,macb,0,0,0,336826,"/Users/tmpadmin/AppData/Local/Microsoft/Edge/User Data/CertificateRevocation/6498.2021.5.1 ($UsnJrnl filename=8244_1963452067 parent='/Users/tmpadmin/AppData/Local/Temp' reason=FILE_CREATE)"
Tue Aug 31 2021 10:49:51,0,macb,0,0,0,336826,"/Users/tmpadmin/AppData/Local/Microsoft/Edge/User Data/CertificateRevocation/6498.2021.5.1 ($UsnJrnl filename=8244_1963452067 parent='/Users/tmpadmin/AppData/Local/Temp' reason=RENAME_OLD_NAME)"
```

# What are the limits of this tool?

Consider the following situation: You have a file, which has a list of attributes which is so long that not all attributes can be stored in the base entry. Then, one or more additional entries are used. If such a file is deleted and the base entry is reused for another file, we can only see that there once a file has existed (using the nonbase entry), but we cannot see the original filename. In addition, if we cannot see the `$FILENAME` attribute, we also cannot see the `$STANDARD_INFORMATION` attribute, which has a lower attribute id. So, we see traces that some files once existed, but we neither see its name nor any timestamps.

If you provide a `$UsnJrnl:$J` file, chances are good that `mft2bodyfile` can find a filename and some timestamps even from deleted files.

# References

- [Forensic analysis of deleted `$MFT` entries](https://janstarke.github.io/2021/10/22/mft_entry_sequence)
