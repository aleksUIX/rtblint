"""
rtblint: OpenRTB bid request / bid response linter.

The Python binding is not implemented yet. The Rust CLI (crates.io: rtblint),
the npm WASM package (npm: rtblint), and the MCP server (crates.io:
rtblint-mcp) are the working surfaces today.
"""

__version__ = "0.1.0"


def validate(input: str) -> dict:
    """Validate an OpenRTB JSON payload (bid request or response).

    Args:
        input: Raw JSON string.

    Returns:
        dict with keys ``valid`` (bool) and ``issues`` (list).

    Raises:
        NotImplementedError: The Python binding is not implemented yet. Use
            the Rust CLI or the npm package in the meantime.
    """
    raise NotImplementedError(
        "The rtblint Python binding is not implemented yet. "
        "Use the Rust CLI (cargo install rtblint) or the npm package (npm i rtblint)."
    )
