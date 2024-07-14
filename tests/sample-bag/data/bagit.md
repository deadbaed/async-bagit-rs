# BagIt

> From Wikipedia, the free encyclopedia

BagIt is a set of hierarchical file system conventions designed to support disk-based storage and network transfer of arbitrary digital content. A "bag" consists of a "payload" (the arbitrary content) and "tags," which are metadata files intended to document the storage and transfer of the bag. A required tag file contains a manifest listing every file in the payload together with its corresponding checksum. The name, BagIt, is inspired by the "enclose and deposit" method, sometimes referred to as "bag it and tag it."

Bags are ideal for digital content normally kept as a collection of files. They are also well-suited to the export, for archival purposes, of content normally kept in database structures that receiving parties are unlikely to support. Relying on cross-platform (Windows and Unix) filesystem naming conventions, a bag's payload may include any number of directories and sub-directories (folders and sub-folders). A bag can specify payload content indirectly via a "fetch.txt" file that lists URLs for content that can be fetched over the network to complete the bag; simple parallelization (e.g. running 10 instances of Wget) can exploit this feature to transfer large bags very quickly. Benefits of bags include:

- Wide adoption in digital libraries (e.g. the Library of Congress).
- Easy to implement using ubiquitous and ordinary filesystem tools.
- Content that originates as files need only be copied to the payload directory.
- Compared to XML wrapping, content need not be encoded (e.g. Base64), which saves time and storage space.
- Received content is ready-to-go in a familiar filesystem tree.
- Easy to implement fast network transfer by running ordinary transfer tools in parallel.

## Specification

BagIt is currently defined in RFC 8493. It defines a simple file naming convention used by the digital curation community for packaging up arbitrary digital content, so that it can be reliably transported via both physical media (hard disk drive, CD-ROM, DVD) and network transfers (FTP, HTTP, rsync, etc.). BagIt is also used for managing the digital preservation of content over time. Discussion about the specification and its future directions takes place on the Digital Curation discussion list.

The BagIt specification is organized around the notion of a "bag." A bag is a named file system directory that minimally contains:

- A "data" directory that includes the payload, or data files that comprise the digital content being preserved. Files can also be placed in subdirectories, but empty directories are not supported.
- At least one manifest file that itemizes the filenames present in the "data" directory, as well as their checksums. The particular checksum algorithm is included as part of the manifest filename. For instance, a manifest file with MD5 checksums is named "manifest-md5.txt."
- A "bagit.txt" file that identifies the directory as a bag, the version of the BagIt specification that it adheres to, and the character encoding used for tag files.

On receipt of a bag, a piece of software can examine the manifest file to make sure that the payload files are present and that their checksums are correct. This allows for accidentally removed or corrupted files to be identified. Below is an example of a minimal bag "myfirstbag" that encloses two files of payload. The contents of the tag files are included below their filenames.

```
myfirstbag/
|-- data
|   \-- 27613-h
|       \-- images
|           \-- q172.png
|           \-- q172.txt
|-- manifest-md5.txt
|     49afbd86a1ca9f34b677a3f09655eae9 data/27613-h/images/q172.png
|     408ad21d50cef31da4df6d9ed81b01a7 data/27613-h/images/q172.txt
\-- bagit.txt
      BagIt-Version: 0.97
      Tag-File-Character-Encoding: UTF-8
```

In this example the payload happens to consist of a Portable Network Graphics image file and an Optical Character Recognition text file. In general the identification and definition of file formats is out of the scope of the BagIt specification; file attributes are likewise out of scope.

The specification allows for several optional tag files (in addition to the manifest). Their character encoding must be identified in "bagit.txt," which itself must always be encoded in UTF-8. The specification defines the following optional tag files:

- A "bag-info.txt" file which details metadata for the bag, using colon-separated key/value pairs (similar to HTTP headers)
- A tag manifest file which lists tag files and their associated checksums (e.g. "tagmanifest-md5.txt")
- A "fetch.txt" that lists URLs where payload files can be retrieved from in addition or to replace payload files in the "data" directory

Until version 15, the draft also described how to serialize a bag in an archive file, such as ZIP or TAR. From version 15 on, the serialization is no longer part of the specifications, not because of technical reasons but because of the scope and focus of the specification.

## History

The BagIt specification emerged from a collaboration between the Library of Congress and the California Digital Library while transferring digital content created as part of the National Digital Information Infrastructure and Preservation Program. The origins of the idea date back to work done at the University of Tsukuba on the "enclose and deposit" model, for mutually depositing archived resources to enable long-term digital preservation. The practice of using manifests and checksums is fairly common practice as evidenced by their use in ZIP (file format), the Deb (file format), as well as on public FTP sites.

In 2007, the California Digital Library needed to transfer several terabytes of content (largely Web archiving data) to the Library of Congress. The BagIt specification allowed the content to be packaged up in "bags" with package metadata and a manifest that detailed file checksums, which were later verified on receipt of the bags. The specification was written up as an IETF draft by John Kunze in December 2008, where it has seen several revisions before being issued as an RFC. In 2009, the Library of Congress produced a video that describes the specification and the use cases around it. In 2018, version 1.0 was published as an RFC by the Internet Engineering Task Force.

