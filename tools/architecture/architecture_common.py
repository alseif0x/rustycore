"""Shared identity/error vocabulary for the architecture checker's private modules."""
import pathlib

REPO_ROOT = pathlib.Path(__file__).resolve().parents[2]


class ArchitectureError(RuntimeError):
    """A policy, metadata, or architecture-contract error."""
