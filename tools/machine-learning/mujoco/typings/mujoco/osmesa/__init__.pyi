from _typeshed import Incomplete

PYOPENGL_PLATFORM: Incomplete

class GLContext:
    def __init__(self, max_width, max_height) -> None: ...
    def make_current(self) -> None: ...
    def free(self) -> None: ...
    def __del__(self) -> None: ...
