from enum import Enum
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
    str, Field(pattern=r"^[\^\$\*\?\w\-\. ]+$", examples=["Black Ledger,Kruxia"])
]

BigID: TypeAlias = int


def new_bigid():
    return int(random() * 1_000)


class Normal(Enum):
    DR = "DR"
    CR = "CR"

    def __int__(self):
        """
        Provide the Normal value as an integer for calculations. Requires explicit cast.
        """
        if self == self.DR:
            return 1
        elif self == self.CR:
            return -1
