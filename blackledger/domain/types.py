import re
from enum import IntEnum
from typing import Optional
from uuid import UUID

import base58
from ulid import ULID

# TODO: ID is mutable - you can assign to ID.bytes after instantiation


class AccountNormal(IntEnum):
    DR = 1
    CR = -1


class EntryDirection(IntEnum):
    DR = -1
    CR = 1


class Currency(str):
    PATTERN = r"^[A-Z]+$"
    REGEX = re.compile(PATTERN)

    def __init__(self, value):
        if not re.match(self.REGEX, value):
            raise ValueError(f"{self.__class__.__name__} must match {self.PATTERN}")
        super().__init__()

    def __repr__(self):
        return f"{self.__class__.__name__}({self})"


class ID(UUID):
    """
    ID is a 128-bit ULID internally, represented as a base58-encoded string.
    Nevertheless it can easily be output as UUID or bytes using ULID methods.
    """

    def __init__(self, value: Optional[str] = None):
        """
        If a value is given, it is a base58 encoded string to initialize ID. Otherwise,
        a new ID is generated via ULID.
        """
        if value:
            super().__init__(ULID(base58.b58decode(value)).hex)
        else:
            super().__init__(ULID().hex)

    def __str__(self):
        """
        ID is represented as a base58-encoded string
        """
        return base58.b58encode(self.bytes).decode()

    @property
    def uuid(self):
        return UUID(self.hex)
