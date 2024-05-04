from .bihyung import *
from pathlib import Path

# it won't work when package is archived within a package
# we might need to create temp files...
server_path = (Path(__file__).parent / "server").absolute().as_posix()

__doc__ = bihyung.__doc__
if hasattr(bihyung, "__all__"):
    __all__ = bihyung.__all__
else:
    __all__ = []

