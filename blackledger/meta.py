import importlib.metadata

__name__ = __name__.split(".")[0]
__version__ = importlib.metadata.version(__name__)
