"""Shared imports, paths and constants for the architecture checks.

Separated from check_architecture.py under #664; every value is unchanged.
"""

from __future__ import annotations

import argparse
import json
import pathlib
import re
import subprocess
import sys
from typing import Any
from urllib.parse import urlsplit

from architecture_common import ArchitectureError, REPO_ROOT
import physical_files
from hotspot_metrics import (
    print_hotspots,
    run_path_module_scanner_self_tests,
    run_hotspot_classifier_self_tests,
    run_hotspot_view_self_tests,
    run_hotspot_ratchet_self_tests,
    validate_hotspot_non_growth,
)


ARCHITECTURE_DIR = pathlib.Path(__file__).resolve().parent
DEFAULT_POLICY = ARCHITECTURE_DIR / "dependency-policy.json"
DEFAULT_ISSUE_LEDGER = ARCHITECTURE_DIR / "architecture-issue-ledger.json"
DEFAULT_RUNTIME_OWNERSHIP_LEDGER = ARCHITECTURE_DIR / "runtime-ownership-ledger.json"
DEFAULT_SESSION_OWNERSHIP_POLICY = ARCHITECTURE_DIR / "session-ownership-policy.json"
DEFAULT_HANDLER_MODULE_POLICY = ARCHITECTURE_DIR / "handler-module-policy.json"
DEFAULT_PHYSICAL_POLICY = ARCHITECTURE_DIR / "physical-file-policy.json"
ARCHITECTURE_DOC = REPO_ROOT / "docs" / "architecture" / "ownership-and-boundaries.md"
HANDLER_SNAPSHOT = ARCHITECTURE_DIR / "world-handler-contract.tsv"
FIXTURES_DIR = ARCHITECTURE_DIR / "fixtures"
DEBT_OWNERSHIP_FIXTURES_DIR = FIXTURES_DIR / "debt-ownership"
LEDGER_ISSUE_STATES = {"open", "closed"}
LEDGER_ISSUE_KINDS = {"epic", "slice"}
HANDLER_MODULE_CAPABILITIES = {"handler_registration", "packet_dispatcher"}
PRODUCT_DEPENDENCY_KINDS = {"normal", "build"}
IGNORED_DEPENDENCY_KINDS = {"dev"}
CRATES_IO_SOURCE = "registry+https://github.com/rust-lang/crates.io-index"
CARGO_METADATA_COMMAND = (
    "cargo",
    "metadata",
    "--locked",
    "--all-features",
    "--format-version",
    "1",
)
