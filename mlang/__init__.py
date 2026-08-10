"""MLang — the Matrix language.

A 2D concatenative language where every operation is a single Unicode glyph,
programs are grids, columns are concurrent strands, and strands talk only
over channels.
"""

from .vm import run_text  # noqa: F401

__version__ = "0.1.0"
