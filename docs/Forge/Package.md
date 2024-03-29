## Assumptions
- There wont be many maintainers per package
- A Package is a set of defined sources and build instructions that together can be built into a binary software that can be redistributed with either IPS or as a Tarball
- SVR4 may be important later
## Desicions
- Not much can be decided now as most questions need to be handled by the tools when they come up and when we start supporting that domain specific feature.
- Domain specific details can be known and understood by the forge since this is it's main purpose.

## Data Layout 
### Root
| Node| Data Type | Arguments | Description | Children | Note |
|-------|---------|---------|----------------|-|-|
| name | string | |Name of the package| None |
| summary | string | | Summary of the package | None |
| classification | string | | Classification of the package | None |
| project-url | string | | Main project page | None |
| license-file | string | | Path of the license file relative to the first source | None | I will probably merge this as property into the license node |
| license | string || Name of the license for display (SPDX-Identifier) | None |
| source | node | optional: Name of the source type String | A section of sources for the package | git, archive, patch, file, overlay, directory |
| configure | node | | Define the settings for a automake based build | option, flag | Some changes to flag will happen so it's more clearer how to define env variables and how to define options and arguments for the configure scripts and makefiles
| build | node || Define a build instruction with scripts (basically a catch all for all custom build systems) | script, install |

### Source Git
| Node| Data Type | Arguments | Description | Children |
|-------|---------|---------|----------------|-|
| git | node | repository: url of the git repository | define a git source | branch, tag, archive, must_stay_as_repo, directory |
| git.branch | property(string) | The Branch of the Repository to checkout | | None |
| git.tag | property(string) | The tag to checkout from the repo | mutually exclusive with branch | None |
| git.archive | property(bool) | true\|false |If true ports attempts to use git-archive to directly grab an archive from the remote. Note this must be supported by the remote (github does not) | None |
|git.must_stay_as_repo | property(bool) | true\|false | Ensure that the resulting tarball archive contains the .git folder | None |
| git.directory | property(string) | | Override the generated directory name to unpack into. Usefull if multiple git sources must be used to build but cannot be unpacked into the same directory | None |

### Source Archive
| Node| Data Type | Arguments | Description | Children |
|-------|---------|---------|----------------|-|
| archive | node | string | define the source url where to get the archive from | sha512 |
| archive.sha512 | property(string) | | The sha512 checksum of the file as generated by `sha512sum` or ports | None |

### Source File
| Node| Data Type | Arguments | Description | Children |
|-------|---------|---------|----------------|-|
| file | Tuple(PathBuf,PathBuf) | 2 | define a file that is saved in the same directory as the package.kdl file or distributed via the forge the second argument defines the name how the file will be copied to in the build directory. Can be left blank | None |

***Example:***
```kdl
file "source.txt"
file "source.txt" "target.tx"
```

### Source Directory
| Node| Data Type | Arguments | Description | Children |
|-------|---------|---------|----------------|-|
| directory | Tuple(PathBuf,PathBuf) | 2 | define a directory that is saved in the same directory as the package.kdl file or distributed via the forge the second argument defines the name how the directory will be copied to in the build directory. Can be left blank | None |

***Example:***
```kdl
directory "source"
directory "source" "target"
```

#### Source Patch
| Node| Data Type | Arguments | Description | Children |
|-------|---------|---------|----------------|-|
| patch | PathBuf | 1 | define a patch file to apply with gpatch ontop the source tree | drop_directories |
| patch.drop_directories | int | the number of directories to drop from the path inside the .patch file. Corresponds to the `-pN` option of gnu patch | None |

***Example:***
```kdl
patch "example.patch"
patch "example2.patch" drop-directories=1
```

### Source Overlay
***Deprecated*** use [Directory](#Source_Directory)

### Configure

### Build

