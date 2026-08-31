#!/usr/bin/env python3
"""Generate and validate TermLeaf's machine-readable test manifests."""

from __future__ import annotations

import hashlib
import json
import re
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CATALOG = ROOT / "testcases.md"
OVERRIDES = ROOT / "tests" / "case_registry.overrides.toml"
REGISTRY = ROOT / "tests" / "case_registry.toml"
PROFILES = ROOT / "tests" / "profiles.toml"
GATES = ROOT / "tests" / "phase_gates.toml"
ENVIRONMENTS = ROOT / "tests" / "environments.toml"
FIXTURES = ROOT / "tests" / "fixtures.toml"

CASE_ID = re.compile(r"^[A-Z][A-Z0-9]*-[0-9]{3}$")
CASE_ROW = re.compile(r"^\|\s*`(?P<id>[A-Z][A-Z0-9]*-[0-9]{3})`\s*\|")
REFERENCE = re.compile(r"\b(?:FX-[A-Z0-9-]+|ENV-[A-Z0-9-]+|DEC-TEST-[0-9]{3})\b")
RUST_TEST = re.compile(r"#\[test\]\s*(?:#\[[^\]]+\]\s*)*fn\s+([a-zA-Z0-9_]+)")
SHA256 = re.compile(r"^[0-9a-f]{64}$")
SPDX_LICENSE = re.compile(
    r"^[A-Za-z0-9][A-Za-z0-9.+-]*(?: (?:AND|OR) [A-Za-z0-9][A-Za-z0-9.+-]*)*$"
)

PROFILE_IDS = (
    "pr-core",
    "pr-render",
    "native-pty",
    "security",
    "scheduled",
    "weekly",
    "release",
)

PHASE_BY_PREFIX = {
    "QG": 0,
    "APP": 0,
    "CLI": 0,
    "TERM": 0,
    "KEY": 1,
    "TXT": 1,
    "MODEL": 1,
    "LAY": 1,
    "NAV": 1,
    "THEME": 1,
    "RENDER": 1,
    "STATUS": 1,
    "ERR": 1,
    "MD": 2,
    "EPUB": 2,
    "SEC": 2,
    "IMG": 2,
    "CON": 2,
    "CFG": 3,
    "STATE": 3,
    "RECENT": 3,
    "SEARCH": 3,
    "ANN": 3,
    "HELP": 3,
    "UI": 3,
    "A11Y": 3,
    "LINK": 4,
    "PRIV": 4,
    "PERF": 4,
    "SUP": 5,
    "REL": 5,
    "PROP": 1,
    "FUZZ": 2,
}

PHASE_EXCEPTIONS = {
    "CLI-003": 1,
    "CLI-007": 1,
    "CLI-008": 3,
    "CLI-009": 3,
    "TERM-006": 1,
    "TERM-007": 1,
    "TERM-009": 2,
    "TERM-010": 5,
    "TERM-013": 5,
    "HELP-001": 1,
    "ERR-002": 0,
    "SUP-001": 0,
    "SUP-002": 0,
    "SUP-003": 0,
    "SUP-004": 0,
    "SUP-006": 0,
    "SUP-007": 0,
    "SUP-008": 0,
}

PROFILE_EXCEPTIONS = {
    "QG-005": "pr-core",
}

IMPLEMENTS = {
    "PROP-001": ["LAY-001", "LAY-008"],
    "PROP-002": ["LAY-002"],
    "PROP-003": ["LAY-006"],
    "PROP-004": ["NAV-001", "NAV-004"],
    "PROP-005": ["SEARCH-005"],
    "PROP-006": ["STATE-001"],
    "PROP-007": ["SEC-003", "SEC-004", "SEC-005"],
    "PROP-008": ["IMG-006", "IMG-007"],
    "PROP-009": ["CON-002"],
    "PROP-010": ["HELP-002", "NAV-009"],
    "FUZZ-001": ["TXT-004", "TXT-008"],
    "FUZZ-002": ["SEC-001", "SEC-010"],
    "FUZZ-003": ["EPUB-001", "EPUB-002", "EPUB-009"],
    "FUZZ-004": ["EPUB-005", "MD-008"],
    "FUZZ-005": ["MD-009", "MD-010"],
    "FUZZ-006": ["IMG-003", "IMG-004", "IMG-015", "IMG-016"],
    "FUZZ-007": ["IMG-001", "IMG-005", "IMG-007"],
    "FUZZ-008": ["STATE-004", "STATE-005", "STATE-012"],
    "FUZZ-009": ["CFG-003"],
    "FUZZ-010": ["PROP-010"],
    "FUZZ-011": ["LINK-001", "LINK-006", "LINK-007"],
    "FUZZ-012": ["SEC-001", "SEC-011"],
}


@dataclass(frozen=True)
class CatalogCase:
    id: str
    section: str
    cells: tuple[str, ...]


def quote(value: str) -> str:
    return json.dumps(value, ensure_ascii=True)


def array(values: list[str] | tuple[str, ...]) -> str:
    return "[" + ", ".join(quote(value) for value in values) + "]"


def clean_cell(cell: str) -> str:
    return re.sub(r"\s+", " ", cell.strip().replace("`", ""))


def parse_catalog(source: str) -> list[CatalogCase]:
    section = ""
    cases: list[CatalogCase] = []
    for line in source.splitlines():
        if line.startswith("## "):
            section = line[3:].strip()
            continue
        match = CASE_ROW.match(line)
        if match is None:
            continue
        cells = tuple(clean_cell(cell) for cell in line.strip().strip("|").split("|"))
        cases.append(CatalogCase(match.group("id"), section, cells))
    return cases


def phase_for(case_id: str) -> int:
    if case_id in PHASE_EXCEPTIONS:
        return PHASE_EXCEPTIONS[case_id]
    return PHASE_BY_PREFIX[case_id.split("-", 1)[0]]


def direct_profile(case: CatalogCase) -> str:
    if case.id in PROFILE_EXCEPTIONS:
        return PROFILE_EXCEPTIONS[case.id]
    if case.id.startswith("FUZZ-"):
        return "weekly"
    joined = " ".join(case.cells).lower()
    for profile in PROFILE_IDS:
        if profile in joined:
            return profile
    if case.id.startswith("PROP-"):
        return "scheduled"
    return "pr-core"


def primary_layer(case: CatalogCase) -> str:
    joined = " ".join(case.cells).lower()
    for layer in ("property", "render", "integration", "pty", "fuzz", "benchmark", "unit", "tool"):
        if re.search(rf"\b{layer}\b", joined):
            return layer
    if re.search(r"\b(?:ci|review|static|scheduled)\b", joined):
        return "tool"
    if case.id.startswith("PROP-"):
        return "property"
    raise ValueError(f"{case.id}: catalog row has no automated test layer")


def priority(case: CatalogCase) -> str:
    for cell in case.cells:
        if cell in {"P0", "P1", "P2"}:
            return cell
    return "P1"


def title(case: CatalogCase) -> str:
    for cell in case.cells[1:]:
        if cell not in {"P0", "P1", "P2"}:
            return cell
    raise ValueError(f"{case.id}: no title cell")


def references(case: CatalogCase, prefix: str) -> list[str]:
    return sorted({item for item in REFERENCE.findall(" ".join(case.cells)) if item.startswith(prefix)})


def load_overrides() -> dict[str, dict[str, object]]:
    with OVERRIDES.open("rb") as handle:
        data = tomllib.load(handle)
    return data.get("cases", {})


def fixture_links() -> dict[str, set[str]]:
    with FIXTURES.open("rb") as handle:
        fixtures = tomllib.load(handle).get("fixture", [])
    links: dict[str, set[str]] = {}
    for fixture in fixtures:
        fixture_id = str(fixture["id"])
        for case_id in fixture.get("served_case_ids", []):
            links.setdefault(str(case_id), set()).add(fixture_id)
    return links


def build_registry(cases: list[CatalogCase], source_hash: str) -> str:
    overrides = load_overrides()
    served_fixtures = fixture_links()
    known_ids = {case.id for case in cases}
    unknown_overrides = sorted(set(overrides) - known_ids)
    if unknown_overrides:
        raise ValueError(f"unknown case overrides: {', '.join(unknown_overrides)}")

    lines = [
        "# Generated by tools/case_registry.py; do not edit directly.",
        "schema_version = 1",
        'generated_from = "testcases.md"',
        f"source_sha256 = {quote(source_hash)}",
        "",
    ]
    for case in cases:
        override = overrides.get(case.id, {})
        joined = " ".join(case.cells)
        decision_ids = references(case, "DEC-TEST-")
        inferred_blocked = "Remains Blocked" in joined
        status = str(override.get("status", "Blocked" if inferred_blocked else "Planned"))
        locations = list(override.get("location", []))
        evidence = list(override.get("last_evidence", []))
        owner_phase = phase_for(case.id)
        profiles = [direct_profile(case)]
        if not case.id.startswith("FUZZ-"):
            profiles.extend(f"phase-gate-{phase}" for phase in range(owner_phase, 6))

        lines.extend(
            [
                "[[case]]",
                f"id = {quote(case.id)}",
                f"title = {quote(title(case))}",
                f"priority = {quote(priority(case))}",
                f"layer = {quote(primary_layer(case))}",
                f"catalog_section = {quote(case.section)}",
                f"status = {quote(status)}",
                f"owner_phase = {owner_phase}",
                'responsible = "termleaf-maintainers"',
                f"implements = {array(IMPLEMENTS.get(case.id, []))}",
                f"location = {array(locations)}",
                f"profiles = {array(profiles)}",
                f"environments = {array(references(case, 'ENV-'))}",
                f"fixtures = {array(sorted(set(references(case, 'FX-')) | served_fixtures.get(case.id, set())))}",
                f"last_evidence = {array(evidence)}",
                f"decisions = {array(decision_ids)}",
                f"evidence_method = {quote(case.cells[-1])}",
            ]
        )
        if status == "Blocked":
            decision = decision_ids[0] if decision_ids else ""
            lines.extend(
                [
                    f"blocked_reason = {quote(str(override.get('blocked_reason', title(case))))}",
                    'compensating_evidence = "No behavior is claimed while the policy remains unresolved."',
                    f"removal_condition = {quote(str(override.get('removal_condition', f'Resolve {decision}' if decision else 'Resolve the named catalog dependency.')))}",
                    'review_date = "2026-09-20"',
                ]
            )
        lines.append("")
    return "\n".join(lines)


def profile_commands(profile: str) -> list[list[str]]:
    commands = {
        "pr-core": [
            ["python3", "tools/case_registry.py", "check"],
            ["cargo", "fmt", "--check"],
            ["cargo", "clippy", "--all-targets", "--all-features", "--locked", "--", "-D", "warnings"],
            ["cargo", "test", "--locked"],
            ["cargo", "test", "--doc", "--locked"],
            ["cargo", "deny", "check"],
        ],
        "pr-render": [["cargo", "test", "--locked", "--test", "render"]],
        "native-pty": [["cargo", "test", "--locked", "--test", "pty_native", "--", "--test-threads=1"]],
        "security": [["cargo", "test", "--locked", "--test", "security"]],
        "scheduled": [["cargo", "test", "--locked", "--all-targets", "--all-features"]],
        "weekly": [["cargo", "test", "--locked", "--all-targets", "--all-features"]],
        "release": [["cargo", "build", "--release", "--locked"]],
    }
    return commands[profile]


def build_profiles(cases: list[CatalogCase]) -> str:
    lines = ["# Generated by tools/case_registry.py; do not edit directly.", "schema_version = 1", ""]
    for profile in PROFILE_IDS:
        profile_cases = [case for case in cases if direct_profile(case) == profile]
        case_ids = [case.id for case in profile_cases if not case.id.startswith("FUZZ-")]
        optional_case_ids = [case.id for case in profile_cases if case.id.startswith("FUZZ-")]
        includes = ["pr-core"] if profile in {"pr-render", "native-pty", "security"} else []
        lines.extend(
            [
                "[[profile]]",
                f"id = {quote(profile)}",
                f"status = {quote('Active' if profile in {'pr-core', 'pr-render', 'native-pty'} else 'Planned')}",
                f"includes = {array(includes)}",
                f"case_ids = {array(case_ids)}",
                f"optional_case_ids = {array(optional_case_ids)}",
                'runner = "native-host"',
                f"timeout_seconds = {30 if profile == 'native-pty' else 600}",
                'retry_policy = "none"',
                f"parallelism = {quote('1' if profile == 'native-pty' else 'cargo-default')}",
                'retained_artifacts = ["command-log", "failure-output"]',
            ]
        )
        for command in profile_commands(profile):
            lines.extend(["[[profile.command]]", f"argv = {array(command)}"])
        lines.append("")
    return "\n".join(lines)


def build_gates(cases: list[CatalogCase]) -> str:
    lines = ["# Generated by tools/case_registry.py; do not edit directly.", "schema_version = 1", ""]
    for phase in range(6):
        owned_cases = [
            case
            for case in cases
            if not case.id.startswith("FUZZ-") and phase_for(case.id) <= phase
        ]
        case_ids = [case.id for case in owned_cases]
        benchmark_ids = [case.id for case in owned_cases if primary_layer(case) == "benchmark"]
        lines.extend(
            [
                "[[gate]]",
                f'id = "phase-gate-{phase}"',
                f"phase = {phase}",
                f"includes = {array([f'phase-gate-{phase - 1}'] if phase else [])}",
                f"case_ids = {array(case_ids)}",
                'required_environment_ids = ["ENV-LINUX-PTY"]',
                f"benchmark_ids = {array(benchmark_ids)}",
                'membership_status = "Frozen"',
                'approver = "termleaf-maintainers"',
                'revision = "membership-v1"',
                'date = "2026-08-20"',
                "",
            ]
        )
    return "\n".join(lines)


def expected_outputs() -> dict[Path, str]:
    source = CATALOG.read_text(encoding="utf-8")
    cases = parse_catalog(source)
    ids = [case.id for case in cases]
    if len(ids) != 336:
        raise ValueError(f"expected 336 catalog cases, found {len(ids)}")
    if len(ids) != len(set(ids)):
        raise ValueError("duplicate case IDs in testcases.md")
    manual_cases = [
        case.id
        for case in cases
        if re.search(r"\bmanual\b", " ".join(case.cells), re.IGNORECASE)
    ]
    if manual_cases:
        raise ValueError("manual test methods are not supported: " + ", ".join(manual_cases))
    source_hash = hashlib.sha256(source.encode()).hexdigest()
    return {
        REGISTRY: build_registry(cases, source_hash),
        PROFILES: build_profiles(cases),
        GATES: build_gates(cases),
    }


def rust_tests() -> set[tuple[str, str]]:
    symbols: set[tuple[str, str]] = set()
    for root in (ROOT / "src", ROOT / "tests"):
        for path in root.rglob("*.rs"):
            relative = path.relative_to(ROOT).as_posix()
            symbols.update((relative, symbol) for symbol in RUST_TEST.findall(path.read_text(encoding="utf-8")))
    return symbols


def markdown_anchor_exists(path: Path, anchor: str) -> bool:
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line.startswith("#"):
            continue
        heading = line.lstrip("#").strip().lower()
        slug = re.sub(r"[^a-z0-9 -]", "", heading).replace(" ", "-")
        if slug == anchor:
            return True
    return False


def validate_fixtures(
    fixtures: list[dict[str, object]], cases: list[dict[str, object]]
) -> dict[str, set[str]]:
    """Validate committed bytes and return registry links merged in both directions."""
    known_case_ids = {str(case["id"]) for case in cases}
    cases_by_id = {str(case["id"]): case for case in cases}
    fixtures_by_id = {str(fixture["id"]): fixture for fixture in fixtures}
    if len(fixtures_by_id) != len(fixtures):
        raise ValueError("fixture manifest contains duplicate IDs")
    fixture_ids = set(fixtures_by_id)
    merged = {
        case_id: {str(item) for item in case.get("fixtures", [])}
        for case_id, case in cases_by_id.items()
    }
    committed_paths: set[str] = set()

    for fixture in fixtures:
        fixture_id = str(fixture["id"])
        if fixture.get("storage") != "committed":
            continue
        required = (
            "path",
            "source",
            "authored_provenance",
            "license",
            "sha256",
            "size",
            "generator",
            "generator_revision",
            "generator_parameters",
            "expected_properties",
            "served_case_ids",
        )
        missing = [field for field in required if not fixture.get(field)]
        if missing:
            raise ValueError(f"{fixture_id}: committed fixture missing {', '.join(missing)}")
        if fixture.get("status") != "Committed":
            raise ValueError(f"{fixture_id}: committed storage requires Committed status")
        license_id = str(fixture["license"])
        if SPDX_LICENSE.fullmatch(license_id) is None:
            raise ValueError(f"{fixture_id}: license is not an SPDX expression: {license_id}")
        digest = str(fixture["sha256"])
        if SHA256.fullmatch(digest) is None:
            raise ValueError(f"{fixture_id}: sha256 must be 64 lowercase hexadecimal characters")

        path_text = str(fixture["path"])
        path = ROOT / path_text
        try:
            path.relative_to(ROOT)
        except ValueError as error:
            raise ValueError(f"{fixture_id}: fixture path escapes the repository") from error
        if Path(path_text).is_absolute() or ".." in Path(path_text).parts:
            raise ValueError(f"{fixture_id}: fixture path must be repository-relative")
        if not path.is_file():
            raise ValueError(f"{fixture_id}: missing committed file {path_text}")
        if path_text in committed_paths:
            raise ValueError(f"{fixture_id}: duplicate committed fixture path {path_text}")
        committed_paths.add(path_text)
        payload = path.read_bytes()
        if not isinstance(fixture["size"], int) or isinstance(fixture["size"], bool):
            raise ValueError(f"{fixture_id}: size must be an integer")
        if len(payload) != fixture["size"]:
            raise ValueError(
                f"{fixture_id}: size is {len(payload)}, manifest records {fixture['size']}"
            )
        actual_digest = hashlib.sha256(payload).hexdigest()
        if actual_digest != digest:
            raise ValueError(f"{fixture_id}: sha256 mismatch for {path_text}")

        expected_properties = fixture["expected_properties"]
        if not isinstance(expected_properties, list) or not all(
            isinstance(value, str) and value for value in expected_properties
        ):
            raise ValueError(f"{fixture_id}: expected_properties must be non-empty strings")
        served_case_ids = fixture["served_case_ids"]
        if not isinstance(served_case_ids, list) or len(served_case_ids) != len(
            set(served_case_ids)
        ):
            raise ValueError(f"{fixture_id}: served_case_ids must be a duplicate-free array")
        for case_id in served_case_ids:
            if case_id not in known_case_ids:
                raise ValueError(f"{fixture_id}: unknown served case ID {case_id}")
            merged[case_id].add(fixture_id)

    for case_id, case in cases_by_id.items():
        for fixture_id in case.get("fixtures", []):
            if fixture_id not in fixture_ids:
                continue
            fixture = fixtures_by_id[str(fixture_id)]
            if fixture.get("storage") == "committed" and case_id not in fixture["served_case_ids"]:
                raise ValueError(
                    f"{fixture_id}: registry links {case_id}, but served_case_ids does not"
                )
    return merged


def validate_registry() -> None:
    with REGISTRY.open("rb") as handle:
        registry = tomllib.load(handle)
    with PROFILES.open("rb") as handle:
        profiles_data = tomllib.load(handle)
    with ENVIRONMENTS.open("rb") as handle:
        environments_data = tomllib.load(handle)
    with FIXTURES.open("rb") as handle:
        fixtures_data = tomllib.load(handle)
    with GATES.open("rb") as handle:
        gates_data = tomllib.load(handle)
    cases = registry["case"]
    ids = [case["id"] for case in cases]
    if any(CASE_ID.fullmatch(case_id) is None for case_id in ids):
        raise ValueError("registry contains a malformed case ID")
    if len(ids) != len(set(ids)):
        raise ValueError("registry contains duplicate case IDs")

    known_ids = set(ids)
    profile_list = profiles_data["profile"]
    profile_ids = [profile["id"] for profile in profile_list]
    if len(profile_ids) != len(set(profile_ids)):
        raise ValueError("profile manifest contains duplicate IDs")
    if set(profile_ids) != set(PROFILE_IDS):
        raise ValueError("profile manifest contains unexpected profile IDs")
    profile_map = {profile["id"]: profile for profile in profile_list}
    environments = environments_data["environment"]
    environment_list = [environment["id"] for environment in environments]
    fixture_list = [fixture["id"] for fixture in fixtures_data["fixture"]]
    if len(environment_list) != len(set(environment_list)):
        raise ValueError("environment manifest contains duplicate IDs")
    for environment in environments:
        if str(environment.get("runner", "")).lower() == "manual":
            raise ValueError(f"{environment['id']}: manual environment runners are not supported")
    if len(fixture_list) != len(set(fixture_list)):
        raise ValueError("fixture manifest contains duplicate IDs")
    environment_ids = set(environment_list)
    fixture_ids = set(fixture_list)
    effective_fixture_links = validate_fixtures(fixtures_data["fixture"], cases)
    gate_list = gates_data["gate"]
    gate_ids = [gate["id"] for gate in gate_list]
    if len(gate_ids) != len(set(gate_ids)):
        raise ValueError("phase gate manifest contains duplicate IDs")
    gate_map = {gate["id"]: gate for gate in gate_list}
    if set(gate_map) != {f"phase-gate-{phase}" for phase in range(6)}:
        raise ValueError("phase gate manifest must contain exactly phases 0 through 5")
    required_gate_ids = {case_id for gate in gate_map.values() for case_id in gate["case_ids"]}
    cases_by_id = {case["id"]: case for case in cases}
    located_tests = {
        (location.partition("::")[0], location.rsplit("::", 1)[-1])
        for case in cases
        for location in case["location"]
        if ".rs::" in location
    }
    orphan_tests = sorted(rust_tests() - located_tests)
    if orphan_tests:
        formatted = ", ".join(f"{path}::{symbol}" for path, symbol in orphan_tests)
        raise ValueError(f"Rust tests without registry locations: {formatted}")

    for case in cases:
        if case["status"] not in {"Planned", "Implemented", "Passing", "Blocked", "Retired"}:
            raise ValueError(f"{case['id']}: invalid status {case['status']}")
        if case["priority"] not in {"P0", "P1", "P2"}:
            raise ValueError(f"{case['id']}: invalid priority {case['priority']}")
        if case["layer"] not in {
            "unit",
            "property",
            "render",
            "integration",
            "pty",
            "fuzz",
            "benchmark",
            "tool",
        }:
            raise ValueError(f"{case['id']}: invalid automated layer {case['layer']}")
        if case["status"] in {"Implemented", "Passing"} and not case["location"]:
            raise ValueError(f"{case['id']}: {case['status']} requires a location")
        if case["status"] in {"Implemented", "Passing"} and not case["last_evidence"]:
            raise ValueError(f"{case['id']}: {case['status']} requires evidence")
        if case["status"] == "Blocked":
            for field in ("blocked_reason", "compensating_evidence", "removal_condition", "review_date"):
                if not case.get(field):
                    raise ValueError(f"{case['id']}: Blocked requires {field}")
        for related in case["implements"]:
            if related not in known_ids:
                raise ValueError(f"{case['id']}: unknown implements ID {related}")
        if case["id"].startswith("FUZZ-"):
            if case["profiles"] != ["weekly"]:
                raise ValueError(f"{case['id']}: fuzz discovery must be optional weekly only")
            if not case["implements"]:
                raise ValueError(f"{case['id']}: fuzz discovery requires deterministic implements IDs")
            for related in case["implements"]:
                if related.startswith("FUZZ-") or related not in required_gate_ids:
                    raise ValueError(
                        f"{case['id']}: {related} must be a deterministic phase-gate case"
                    )
        for environment in case["environments"]:
            if environment not in environment_ids:
                raise ValueError(f"{case['id']}: unknown environment {environment}")
        for fixture in effective_fixture_links[case["id"]]:
            if fixture not in fixture_ids:
                raise ValueError(f"{case['id']}: unknown fixture {fixture}")
        for location in case["location"]:
            path_text, _, symbol = location.partition("::")
            path = ROOT / path_text
            if not path.is_file():
                raise ValueError(f"{case['id']}: missing location {path_text}")
            if symbol and symbol not in path.read_text(encoding="utf-8"):
                raise ValueError(f"{case['id']}: missing symbol {symbol}")
            if path.suffix == ".rs" and symbol:
                test_symbols = set(RUST_TEST.findall(path.read_text(encoding="utf-8")))
                if symbol.rsplit("::", 1)[-1] not in test_symbols:
                    raise ValueError(f"{case['id']}: location is not a Rust test {symbol}")
        for evidence in case["last_evidence"]:
            evidence_path, separator, anchor = evidence.partition("#")
            path = ROOT / evidence_path
            if not path.is_file():
                raise ValueError(f"{case['id']}: missing evidence file {evidence_path}")
            if separator and not markdown_anchor_exists(path, anchor):
                raise ValueError(f"{case['id']}: missing evidence anchor {anchor}")
        direct = case["profiles"][0]
        profile_field = "optional_case_ids" if case["id"].startswith("FUZZ-") else "case_ids"
        if case["id"] not in profile_map[direct][profile_field]:
            raise ValueError(f"{case['id']}: missing from profile {direct}")
        for gate_id in case["profiles"][1:]:
            if gate_id not in gate_map or case["id"] not in gate_map[gate_id]["case_ids"]:
                raise ValueError(f"{case['id']}: missing from gate {gate_id}")

    for profile in profile_map.values():
        if not profile["command"]:
            raise ValueError(f"{profile['id']}: profile has no commands")
        for included in profile["includes"]:
            if included not in profile_map:
                raise ValueError(f"{profile['id']}: unknown included profile {included}")
        required_ids = profile["case_ids"]
        optional_ids = profile.get("optional_case_ids", [])
        if len(required_ids) != len(set(required_ids)):
            raise ValueError(f"{profile['id']}: duplicate required case IDs")
        if len(optional_ids) != len(set(optional_ids)):
            raise ValueError(f"{profile['id']}: duplicate optional case IDs")
        if set(required_ids) & set(optional_ids):
            raise ValueError(f"{profile['id']}: cases cannot be both required and optional")
        for case_id in required_ids:
            if case_id not in known_ids:
                raise ValueError(f"{profile['id']}: unknown case ID {case_id}")
            if case_id.startswith("FUZZ-"):
                raise ValueError(f"{profile['id']}: fuzz IDs cannot be required profile cases")
        if optional_ids and profile["id"] != "weekly":
            raise ValueError(f"{profile['id']}: optional cases are only supported by weekly")
        for case_id in optional_ids:
            if case_id not in known_ids:
                raise ValueError(f"{profile['id']}: unknown optional case ID {case_id}")
            if not case_id.startswith("FUZZ-"):
                raise ValueError(f"{profile['id']}: only fuzz IDs may be optional")
    previous_ids: set[str] = set()
    for phase in range(6):
        gate = gate_map[f"phase-gate-{phase}"]
        gate_ids = set(gate["case_ids"])
        if any(case_id.startswith("FUZZ-") for case_id in gate_ids):
            raise ValueError(f"phase-gate-{phase}: fuzz IDs cannot be required gate cases")
        if "manual_procedure_ids" in gate:
            raise ValueError(f"phase-gate-{phase}: manual procedures cannot gate automation")
        if "fuzz_durations_seconds" in gate:
            raise ValueError(f"phase-gate-{phase}: must not define default fuzz durations")
        if not previous_ids.issubset(gate_ids):
            raise ValueError(f"phase-gate-{phase}: does not include the prior gate")
        unknown_environments = set(gate["required_environment_ids"]) - environment_ids
        if unknown_environments:
            raise ValueError(
                f"phase-gate-{phase}: unknown environments {sorted(unknown_environments)}"
            )
        previous_ids = gate_ids

    incomplete_foundation = sorted(
        case_id
        for case_id in gate_map["phase-gate-0"]["case_ids"]
        if cases_by_id[case_id]["status"] not in {"Implemented", "Passing"}
    )
    if incomplete_foundation:
        raise ValueError(
            "phase-gate-0 has cases without implementation evidence: "
            + ", ".join(incomplete_foundation)
        )


def generate() -> None:
    for path, content in expected_outputs().items():
        path.write_text(content, encoding="utf-8")


def check() -> None:
    for path, content in expected_outputs().items():
        if not path.exists() or path.read_text(encoding="utf-8") != content:
            raise ValueError(f"{path.relative_to(ROOT)} is stale; run tools/case_registry.py generate")
    validate_registry()


def check_fixtures() -> None:
    with REGISTRY.open("rb") as handle:
        cases = tomllib.load(handle)["case"]
    with FIXTURES.open("rb") as handle:
        fixtures = tomllib.load(handle)["fixture"]
    validate_fixtures(fixtures, cases)


def main() -> int:
    if len(sys.argv) != 2 or sys.argv[1] not in {"generate", "check", "check-fixtures"}:
        print("usage: tools/case_registry.py {generate|check|check-fixtures}", file=sys.stderr)
        return 2
    try:
        if sys.argv[1] == "generate":
            generate()
        elif sys.argv[1] == "check-fixtures":
            check_fixtures()
        else:
            check()
    except (OSError, ValueError, tomllib.TOMLDecodeError) as error:
        print(f"case registry error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
