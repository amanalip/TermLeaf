#!/usr/bin/env python3
"""Generate or verify TermLeaf's small deterministic hostile fixture corpus."""

from __future__ import annotations

import binascii
import gzip
import io
import shutil
import struct
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path

import make_epub_fixtures as epub

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "tests" / "fixtures"
REVISION = "fixture-corpus-v1"


def encoded(text: str) -> bytes:
    return text.encode("utf-8")


def gzip_bytes(payload: bytes) -> bytes:
    output = bytearray(b"\x1f\x8b\x08\x00\x00\x00\x00\x00\x00\xff")
    chunks = [
        payload[index : index + 0xFFFF]
        for index in range(0, len(payload), 0xFFFF)
    ]
    if not chunks:
        chunks = [b""]
    for index, chunk in enumerate(chunks):
        output.append(1 if index == len(chunks) - 1 else 0)
        output.extend(struct.pack("<HH", len(chunk), len(chunk) ^ 0xFFFF))
        output.extend(chunk)
    output.extend(
        struct.pack(
            "<II",
            binascii.crc32(payload) & 0xFFFFFFFF,
            len(payload) & 0xFFFFFFFF,
        )
    )
    return bytes(output)


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
    malformed_epub = epub.zip_bytes(
        common
        + [
            ("OEBPS/content.opf", encoded(epub.epub3_opf())),
            ("OEBPS/nav.xhtml", encoded(epub.epub3_nav())),
            ("OEBPS/ch1.xhtml", encoded('<html xmlns="http://www.w3.org/1999/xhtml"><body><h1>Recoverable<h1><p>Readable text &amp; trailing text</body></html>')),
            ("OEBPS/ch2.xhtml", encoded('<html xmlns="http://www.w3.org/1999/xhtml"><body><p>Missing close')),
        ],
    )
    hostile_opf = epub.epub3_opf().replace(
        '<item id="c1" href="ch1.xhtml" media-type="application/xhtml+xml"/>',
        '<item id="c1" href="../escape.xhtml" media-type="application/xhtml+xml"/>',
    )
    hostile_epub = epub.zip_bytes(
        common
        + [
            ("OEBPS/content.opf", encoded(hostile_opf)),
            ("OEBPS/nav.xhtml", encoded(epub.epub3_nav())),
            ("../escape.xhtml", encoded('<!DOCTYPE x [<!ENTITY ext SYSTEM "file:///etc/passwd">]><html xmlns="http://www.w3.org/1999/xhtml"><body><script>never()</script><img src="https://example.invalid/x"/>&ext;</body></html>')),
            ("OEBPS/ch2.xhtml", encoded(epub.CHAPTER_TWO)),
        ],
    )
    valid_epub2 = epub.zip_bytes(
        common
        + [
            ("OEBPS/content.opf", encoded(epub.epub2_opf())),
            ("OEBPS/toc.ncx", encoded(epub.epub2_ncx())),
            ("OEBPS/ch1.xhtml", encoded(epub.CHAPTER_ONE)),
            ("OEBPS/ch2.xhtml", encoded(epub.CHAPTER_TWO)),
        ],
    )
    valid_epub3 = epub.zip_bytes(
        common
        + [
            ("OEBPS/content.opf", encoded(epub.epub3_opf())),
            ("OEBPS/nav.xhtml", encoded(epub.epub3_nav())),
            ("OEBPS/ch1.xhtml", encoded(epub.CHAPTER_ONE)),
            ("OEBPS/ch2.xhtml", encoded(epub.CHAPTER_TWO)),
        ],
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


def validate_serializers() -> None:
    for size in (0, 1, 0xFFFF, 0x10000):
        payload = bytes(index & 0xFF for index in range(size))
        archive = gzip_bytes(payload)
        if archive[:10] != b"\x1f\x8b\x08\x00\x00\x00\x00\x00\x00\xff":
            raise ValueError("deterministic gzip header changed")
        if gzip.decompress(archive) != payload:
            raise ValueError(f"deterministic gzip failed to round-trip {size} bytes")

    files = generated_files()
    for relative in (
        "epub/minimal-epub2.epub",
        "epub/minimal-epub3.epub",
        "epub/malformed.epub",
        "epub/hostile.epub",
    ):
        with zipfile.ZipFile(io.BytesIO(files[relative])) as archive:
            members = archive.infolist()
            if not members or members[0].filename != "mimetype":
                raise ValueError(f"{relative}: mimetype is not the first member")
            if archive.read("mimetype") != encoded(epub.MIMETYPE):
                raise ValueError(f"{relative}: invalid mimetype payload")
            if any(member.compress_type != zipfile.ZIP_STORED for member in members):
                raise ValueError(f"{relative}: compressed member is not deterministic")
            if any(member.date_time != epub.STAMP for member in members):
                raise ValueError(f"{relative}: member timestamp changed")


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
        validate_serializers()
        generate() if sys.argv[1] == "generate" else check()
    except (OSError, subprocess.CalledProcessError, ValueError) as error:
        print(f"fixture corpus error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
