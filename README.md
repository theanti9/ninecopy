# ninecopy
----------

Ninecopy is a fast, miltithreaded directory copy utility.

Individual file copies are not supported as it's effectively the same as cp/copy. Ninecopy is meant to
copy large / deep directory structures quickly and will happily fully saturate CPU and Disk/Network utilization.

## Usage

```
$ ./ninecopy.exe --help
Fast, multithreaded directory copy utility

Usage: ninecopy.exe [OPTIONS] <SOURCE> <DESTINATION>

Arguments:
  <SOURCE>
          The folder you want to copy.

          e.x. "C:\MyFolder"

  <DESTINATION>
          The location you want to copy SOURCE to.

          e.x. "D:\MyFolder"

Options:
  -o, --overwrite
          Overwrite existing files.

          If this is false, the process will exit if existing files at the destination are encountered.

  -p, --progress
          Periodically log progress

  -t, --threads <THREADS>
          The number of threads to use for search and copy.

          Defaults to one per core.

          Transfers with mostly large files may benefit from thread counts higher than one per core, depending on the core count and disk throughput.

  -h, --help
          Print help information (use `-h` for a summary)

  -V, --version
          Print version information
```