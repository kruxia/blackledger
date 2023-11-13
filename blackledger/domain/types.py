from enum import IntEnum
from typing import Optional
from uuid import UUID

import base58
from pydantic import constr
from ulid import ULID


class Normal(IntEnum):
    DR = 1
    CR = -1


CurrencyCode = constr(
    pattern=r"^[A-Z][A-Z0-9\.\-_]*[A-Z0-9]$",
)


class ID(UUID):
    """
    ID is generated from ULID, stored as a 128-bit UUID internally, and represented as a
    base58-encoded string. It can easily be output as UUID or bytes using ULID methods.
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

    @classmethod
    def from_bytes(cls, value: bytes):
        return cls(base58.b58encode(value))

    @classmethod
    def from_uuid(cls, value: str | UUID):
        if isinstance(value, str):
            value = UUID(value)
        return cls(base58.b58encode(value.bytes))

    @classmethod
    def field_converter(cls, value):
        if isinstance(value, str):
            value = cls(value)
        if isinstance(value, UUID) and not isinstance(value, cls):
            value = cls.from_uuid(value)
        return value

    def to_uuid(self):
        return UUID(self.hex)
