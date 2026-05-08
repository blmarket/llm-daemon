from .bihyung import *
from pathlib import Path
import warnings

# it won't work when package is archived within a package
# we might need to create temp files...
_server_path = (Path(__file__).parent / "server").absolute().as_posix()


def daemon_from_model(model: bihyung.Model) -> bihyung.DaemonHandle:
    warnings.warn(
        "daemon_from_model is deprecated; use daemon_from_hf instead",
        DeprecationWarning,
        stacklevel=2,
    )
    return bihyung._daemon_from_model(model, _server_path)


def daemon_from_gguf(path: str) -> bihyung.DaemonHandle:
    warnings.warn(
        "daemon_from_gguf is deprecated; use daemon_from_hf instead",
        DeprecationWarning,
        stacklevel=2,
    )
    return bihyung._daemon_from_gguf(path, _server_path)

__doc__ = bihyung.__doc__
if hasattr(bihyung, "__all__"):
    __all__ = bihyung.__all__
else:
    __all__ = []

