from enum import IntEnum
from random import random
from typing import TypeAlias

from pydantic import constr

CurrencyCode = constr(pattern=r"^[A-Z][A-Z0-9\.\-_]*[A-Z0-9]$")
CurrencyFilter = constr(pattern=r"^[\^\$\*\?A-Za-z0-9\.\-_]+$")
NameString = constr(pattern=r"^[\w\-\. ]+$")
NameFilter = constr(pattern=r"^[\^\$\*\?\w\-\. ]+$")

BigID: TypeAlias = int


def make_bigid():
    return int(random() * 1_000)


class NormalType(IntEnum):
    DR = 1
    CR = -1

    @classmethod
    def from_str(cls, value):
        if isinstance(value, str):
            if value not in cls.__members__:
                raise ValueError("Not a valid NormalType value")
            value = cls.__members__[value]
        return value

    @classmethod
    def to_str(cls, value):
        return value.name
