"""Lamina public-dataset validation framework (implementation package).

See validation/SPEC.md. All functionality lives in submodules; this file only
carries the version so that both import contexts (repo root via the outer shim,
or CWD=validation/) behave identically.
"""

from ._version import __version__  # noqa: F401
