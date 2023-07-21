import json
import os
from pathlib import Path

from setuptools import find_packages, setup

PATH = Path(os.path.dirname(os.path.abspath(__file__)))

with open(PATH / "setup.json") as f:
    CONFIG = json.load(f)

with open(PATH / "README.md") as f:
    README = f.read()


if __name__ == "__main__":
    setup(
        long_description=README,
        long_description_content_type="text/markdown",
        packages=find_packages(exclude=["contrib", "docs", "tests"]),
        include_package_data=True,
        install_requires=[
            "base58~=2.1.1",
            "orjson~=3.9.2",
            "psycopg[binary,pool]~=3.1.9",
            "pydantic~=2.0.3",
            "sqly[migration] @ git+https://github.com/kruxia/sqly@00f46ee136e9acc286a2b734413a91dd8a6a8102#egg=sqly",  # noqa
            "python-ulid~=1.1.0",
        ],
        extras_require={
            "dev": [
                "black~=23.3.0",
                "ipython~=8.12.0",
                "isort~=5.12.0",
                "twine~=4.0.2",
            ],
            "test": [
                "black~=23.3.0",
                "flake8~=6.0.0",
                "pytest~=7.3.1",
                "pytest-cov~=4.0.0",
            ],
        },
        **CONFIG
    )
