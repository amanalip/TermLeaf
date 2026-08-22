#!/usr/bin/env python3
"""Generate TermLeaf's deterministic minimal EPUB 2 and EPUB 3 fixtures.

Both books carry identical reading content so semantic differences between
the formats stay attributable to packaging alone. Every archive member uses
a fixed timestamp so output bytes are reproducible across machines.
"""

from __future__ import annotations

import hashlib
import sys
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "tests" / "fixtures" / "epub"
STAMP = (2026, 8, 21, 12, 0, 0)

MIMETYPE = "application/epub+zip"

CONTAINER = """<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>
"""

CHAPTER_ONE = """<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>Chapter One</title></head>
<body>
  <h1>Chapter One</h1>
  <p>The garden gate opens onto a quiet lane.</p>
  <p>Two lines of prose share one paragraph here.</p>
</body>
</html>
"""

CHAPTER_TWO = """<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>Chapter Two</title></head>
<body>
  <h1>Chapter Two</h1>
  <p>The lane ends at an orchard wall.</p>
</body>
</html>
"""


def chapter(title: str, body_paragraph: str) -> str:
    return f"""<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>{title}</title></head>
<body>
  <h1>{title}</h1>
  <p>{body_paragraph}</p>
</body>
</html>
"""


def epub2_opf() -> str:
    return """<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf"
         xmlns:dc="http://purl.org/dc/elements/1.1/"
         xmlns:opf="http://www.idpf.org/2007/opf"
         unique-identifier="bookid"
         version="2.0">
  <metadata>
    <dc:title>TermLeaf Fixture EPUB 2</dc:title>
    <dc:creator opf:role="aut">TermLeaf Fixture Press</dc:creator>
    <dc:language>en</dc:language>
    <dc:identifier id="bookid">urn:uuid:termleaf-fixture-epub2</dc:identifier>
  </metadata>
  <manifest>
    <item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>
    <item id="c1" href="ch1.xhtml" media-type="application/xhtml+xml"/>
    <item id="c2" href="ch2.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine toc="ncx">
    <itemref idref="c1"/>
    <itemref idref="c2"/>
  </spine>
</package>
"""


def epub2_ncx() -> str:
    return """<?xml version="1.0" encoding="UTF-8"?>
<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/" version="2005-1">
  <head>
    <meta name="dtb:uid" content="urn:uuid:termleaf-fixture-epub2"/>
    <meta name="dtb:depth" content="1"/>
    <meta name="dtb:totalPageCount" content="0"/>
    <meta name="dtb:maxPageNumber" content="0"/>
  </head>
  <docTitle><text>TermLeaf Fixture EPUB 2</text></docTitle>
  <navMap>
    <navPoint id="n1" playOrder="1">
      <navLabel><text>Chapter One</text></navLabel>
      <content src="ch1.xhtml"/>
    </navPoint>
    <navPoint id="n2" playOrder="2">
      <navLabel><text>Chapter Two</text></navLabel>
      <content src="ch2.xhtml"/>
    </navPoint>
  </navMap>
</ncx>
"""


def epub3_opf() -> str:
    return """<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf"
         xmlns:dc="http://purl.org/dc/elements/1.1/"
         unique-identifier="bookid"
         version="3.0">
  <metadata>
    <dc:title>TermLeaf Fixture EPUB 3</dc:title>
    <dc:creator>TermLeaf Fixture Press</dc:creator>
    <dc:language>en</dc:language>
    <dc:identifier id="bookid">urn:uuid:termleaf-fixture-epub3</dc:identifier>
    <meta property="dcterms:modified">2026-08-21T12:00:00Z</meta>
  </metadata>
  <manifest>
    <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
    <item id="c1" href="ch1.xhtml" media-type="application/xhtml+xml"/>
    <item id="c2" href="ch2.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine>
    <itemref idref="c1"/>
    <itemref idref="c2"/>
  </spine>
</package>
"""


def epub3_nav() -> str:
    return """<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml"
      xmlns:epub="http://www.idpf.org/2007/ops">
<head><title>Navigation</title></head>
<body>
  <nav epub:type="toc">
    <h1>Contents</h1>
    <ol>
      <li><a href="ch1.xhtml">Chapter One</a></li>
      <li><a href="ch2.xhtml">Chapter Two</a></li>
    </ol>
  </nav>
  <nav epub:type="landmarks" hidden="hidden">
    <ol>
      <li><a epub:type="bodymatter" href="ch1.xhtml">Start of Reading</a></li>
    </ol>
  </nav>
</body>
</html>
"""


def write_epub(path: Path, members: list[tuple[str, str]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(path, "w") as archive:
        for index, (name, payload) in enumerate(members):
            info = zipfile.ZipInfo(name, date_time=STAMP)
            info.compress_type = zipfile.ZIP_STORED if index == 0 else zipfile.ZIP_DEFLATED
            if index == 0:
                info.external_attr = 0o444 << 16
            archive.writestr(info, payload.encode("utf-8"))


def main() -> int:
    epub2 = FIXTURES / "minimal-epub2.epub"
    epub3 = FIXTURES / "minimal-epub3.epub"

    write_epub(
        epub2,
        [
            ("mimetype", MIMETYPE),
            ("META-INF/container.xml", CONTAINER),
            ("OEBPS/content.opf", epub2_opf()),
            ("OEBPS/toc.ncx", epub2_ncx()),
            ("OEBPS/ch1.xhtml", CHAPTER_ONE),
            ("OEBPS/ch2.xhtml", CHAPTER_TWO),
        ],
    )
    write_epub(
        epub3,
        [
            ("mimetype", MIMETYPE),
            ("META-INF/container.xml", CONTAINER),
            ("OEBPS/content.opf", epub3_opf()),
            ("OEBPS/nav.xhtml", epub3_nav()),
            ("OEBPS/ch1.xhtml", CHAPTER_ONE),
            ("OEBPS/ch2.xhtml", CHAPTER_TWO),
        ],
    )

    for path in (epub2, epub3):
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        print(f"{path.name}: sha256={digest}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
