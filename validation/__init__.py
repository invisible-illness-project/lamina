"""Lamina public-dataset validation framework.

Repository-root namespace shim: this file lives at ``validation/__init__.py`` so
that ``python -m validation ...`` and ``import validation.<module>`` work with the
repository root as CWD. It redirects the package search path to the real package
directory ``validation/validation/``. See validation/SPEC.md §1.
"""

import os as _os

__path__ = [_os.path.join(_os.path.dirname(_os.path.abspath(__file__)), "validation")]

from ._version import __version__  # noqa: E402,F401
