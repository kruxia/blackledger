from enum import IntEnum
from random import random
from typing import Annotated, TypeAlias

from pydantic import Field

CurrencyCode = Annotated[
    str,
    Field(pattern=r"^[A-Z][A-Z0-9\.\-_]*[A-Z0-9]$", examples=["USD", "CAD", "GOOG"]),
]
CurrencyFilter = Annotated[
    str, Field(pattern=r"^[\^\$\*\?A-Za-z0-9\.\-_]+$", examples=["USD,GOOG"])
]

Name = Annotated[str, Field(pattern=r"^[\w\-\. ]+$", examples=["Black Ledger"])]
NameFilter = Annotated[
    str, Field(pattern=r"^[\^\$\*\?\w\-\. ]+$", examples=["Ledger,Software"])
]

BigID: TypeAlias = int


def new_bigid():
    return int(random() * 1_000)


class Normal(IntEnum):
    DR = 1
    CR = -1

    @classmethod
    def from_str(cls, value):
        if isinstance(value, str):
            if value not in cls.__members__:
                raise ValueError("Not a valid Normal value")
            value = cls.__members__[value]
        return value

    @classmethod
    def to_str(cls, value):
        return value.name
