"""
rtblint — OpenRTB bid request / response linter.

Stub release — full implementation coming in 0.1.0.
"""

__version__ = "0.0.1"


def validate(input: str) -> dict:
    """Validate an OpenRTB JSON payload (bid request or response).

    Args:
        input: Raw JSON string.

    Returns:
        dict with keys ``valid`` (bool) and ``issues`` (list).

    Raises:
        NotImplementedError: This is a stub release.
    """
    raise NotImplementedError(
        "rtblint 0.0.1 is a stub — full implementation coming in 0.1.0"
    )
