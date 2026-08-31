#!/usr/bin/env python3
"""Generate or verify TermLeaf's small deterministic hostile fixture corpus."""

from __future__ import annotations

import gzip
import io
import shutil
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path

import make_epub_fixtures as epub

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "tests" / "fixtures"
REVISION = "fixture-corpus-v1"
STAMP = (2026, 8, 21, 12, 0, 0)


def zip_bytes(
    members: list[tuple[str, bytes]],
    *,
    mimetype_first: bool = False,
    store_all: bool = False,
) -> bytes:
    output = io.BytesIO()
    with zipfile.ZipFile(output, "w") as archive:
        for index, (name, payload) in enumerate(members):
            info = zipfile.ZipInfo(name, date_time=STAMP)
            info.compress_type = (
                zipfile.ZIP_STORED
                if store_all or (mimetype_first and index == 0)
                else zipfile.ZIP_DEFLATED
            )
            if mimetype_first and index == 0:
                info.external_attr = 0o444 << 16
            archive.writestr(info, payload)
    return output.getvalue()


def encoded(text: str) -> bytes:
    return text.encode("utf-8")


def gzip_bytes(payload: bytes) -> bytes:
    output = io.BytesIO()
    with gzip.GzipFile(filename="", mode="wb", fileobj=output, mtime=0) as stream:
        stream.write(payload)
    return output.getvalue()


def generated_files() -> dict[str, bytes]:
    utf16_text = "TermLeaf UTF-16 fixture.\nCafe and snowman: \u2603\n"
    safe_svg = encoded(
        '<svg xmlns="http://www.w3.org/2000/svg" width="2" height="2" viewBox="0 0 2 2">'
        '<rect width="2" height="2" fill="#dc2814"/></svg>\n'
    )
    hostile_svg = encoded(
        '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" '
        'width="2" height="2"><script>alert(1)</script><image width="2" height="2" '
        'xlink:href="file:///etc/passwd"/><image href="https://example.invalid/pixel.png"/></svg>\n'
    )
    common = [
        ("mimetype", encoded(epub.MIMETYPE)),
        ("META-INF/container.xml", encoded(epub.CONTAINER)),
    ]
    malformed_epub = zip_bytes(
        common
        + [
            ("OEBPS/content.opf", encoded(epub.epub3_opf())),
            ("OEBPS/nav.xhtml", encoded(epub.epub3_nav())),
            ("OEBPS/ch1.xhtml", encoded('<html xmlns="http://www.w3.org/1999/xhtml"><body><h1>Recoverable<h1><p>Readable text &amp; trailing text</body></html>')),
            ("OEBPS/ch2.xhtml", encoded('<html xmlns="http://www.w3.org/1999/xhtml"><body><p>Missing close')),
        ],
        mimetype_first=True,
    )
    hostile_opf = epub.epub3_opf().replace(
        '<item id="c1" href="ch1.xhtml" media-type="application/xhtml+xml"/>',
        '<item id="c1" href="../escape.xhtml" media-type="application/xhtml+xml"/>',
    )
    hostile_epub = zip_bytes(
        common
        + [
            ("OEBPS/content.opf", encoded(hostile_opf)),
            ("OEBPS/nav.xhtml", encoded(epub.epub3_nav())),
            ("../escape.xhtml", encoded('<!DOCTYPE x [<!ENTITY ext SYSTEM "file:///etc/passwd">]><html xmlns="http://www.w3.org/1999/xhtml"><body><script>never()</script><img src="https://example.invalid/x"/>&ext;</body></html>')),
            ("OEBPS/ch2.xhtml", encoded(epub.CHAPTER_TWO)),
        ],
        mimetype_first=True,
        store_all=True,
    )
    valid_epub2 = zip_bytes(
        common
        + [
            ("OEBPS/content.opf", encoded(epub.epub2_opf())),
            ("OEBPS/toc.ncx", encoded(epub.epub2_ncx())),
            ("OEBPS/ch1.xhtml", encoded(epub.CHAPTER_ONE)),
            ("OEBPS/ch2.xhtml", encoded(epub.CHAPTER_TWO)),
        ],
        mimetype_first=True,
    )
    valid_epub3 = zip_bytes(
        common
        + [
            ("OEBPS/content.opf", encoded(epub.epub3_opf())),
            ("OEBPS/nav.xhtml", encoded(epub.epub3_nav())),
            ("OEBPS/ch1.xhtml", encoded(epub.CHAPTER_ONE)),
            ("OEBPS/ch2.xhtml", encoded(epub.CHAPTER_TWO)),
        ],
        mimetype_first=True,
    )
    return {
        "txt/utf8.txt": encoded("ASCII and cafe.\nAccents: caf\u00e9 and cafe\u0301.\nCJK: \u4e66\u8449. Emoji: \U0001f343.\n\nFinal paragraph.\n"),
        "txt/utf16le.txt": b"\xff\xfe" + utf16_text.encode("utf-16le"),
        "txt/utf16be.txt": b"\xfe\xff" + utf16_text.encode("utf-16be"),
        "txt/malformed.bin": b"valid-prefix\n\xf0\x28\x8c\x28\n\xff\xfe\x00",
        "markdown/semantic.md": encoded("# Fixture heading\n\nParagraph with *emphasis*, **strong**, [link](#fixture-heading), and ![leaf](leaf.png).\n\n> A quotation.\n\n1. ordered\n2. second\n\n- unordered\n  - nested\n\n| Key | Value |\n| --- | --- |\n| alpha | beta |\n\n---\n\nInline `code` and <span>safe semantic text</span>.\n"),
        "markdown/code.md": encoded("# Code fixture\n\nInline ``a `tick` here``.\n\n```rust\nfn main() {\n\tprintln!(\"leaf\");\n}\n\n// preserved blank line above\n```\n\n    indented();\n\n```console\n$ termleaf book.epub\nstatus: ready\n```\n\nLong: 0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789abcdefghijklmnopqrstuvwxyz\n"),
        "markdown/hostile.md": encoded("# Hostile but inert\n\n<script>fetch('https://example.invalid/')</script>\n<img src=\"file:///etc/passwd\" onerror=\"alert(1)\">\n<a href=\"javascript:alert(1)\">visible link text</a>\n<!-- unclosed comment\n\n````unterminated\n[broken](<https://example.invalid/ space>)\n"),
        "epub/minimal-epub2.epub": valid_epub2,
        "epub/minimal-epub3.epub": valid_epub3,
        "epub/malformed.epub": malformed_epub,
        "epub/hostile.epub": hostile_epub,
        "images/safe.svg": safe_svg,
        "images/hostile.svg": hostile_svg,
        "images/safe.svgz": gzip_bytes(safe_svg),
        "images/malformed.svgz": gzip_bytes(safe_svg)[:-5],
    }


def build(destination: Path) -> None:
    for relative, payload in generated_files().items():
        path = destination / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(payload)
    subprocess.run(
        ["cargo", "run", "--quiet", "--locked", "--example", "fixture_rasters", "--", str(destination / "images")],
        cwd=ROOT,
        check=True,
    )


def generate() -> None:
    with tempfile.TemporaryDirectory(prefix="termleaf-fixtures-") as temporary:
        generated = Path(temporary)
        build(generated)
        for source in sorted(path for path in generated.rglob("*") if path.is_file()):
            destination = FIXTURES / source.relative_to(generated)
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, destination)


def check() -> None:
    with tempfile.TemporaryDirectory(prefix="termleaf-fixtures-") as temporary:
        generated = Path(temporary)
        build(generated)
        expected = {path.relative_to(generated) for path in generated.rglob("*") if path.is_file()}
        committed = {path.relative_to(FIXTURES) for path in FIXTURES.rglob("*") if path.is_file()}
        extras = sorted(committed - expected)
        if extras:
            raise ValueError(
                "unregistered fixture files: " + ", ".join(path.as_posix() for path in extras)
            )
        for relative in sorted(expected):
            committed = FIXTURES / relative
            if not committed.is_file():
                raise ValueError(f"missing fixture: {committed.relative_to(ROOT)}")
            if committed.read_bytes() != (generated / relative).read_bytes():
                raise ValueError(f"stale fixture: {committed.relative_to(ROOT)}")
        print(f"fixture corpus {REVISION}: {len(expected)} deterministic files verified")


def main() -> int:
    if len(sys.argv) != 2 or sys.argv[1] not in {"generate", "check"}:
        print("usage: tools/fixture_corpus.py {generate|check}", file=sys.stderr)
        return 2
    try:
        generate() if sys.argv[1] == "generate" else check()
    except (OSError, subprocess.CalledProcessError, ValueError) as error:
        print(f"fixture corpus error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
