from datetime import datetime, timezone
from decimal import Decimal
from typing import Optional

from pydantic import BaseModel, Field

from . import types


class Model(BaseModel):
    class Config:
        arbitrary_types_allowed = True


class Account(Model):
    id: types.ID = Field(default_factory=types.ID)
    name: str
    normal: types.AccountNormal
    parent: Optional["Account"] = None


class Transaction(Model):
    id: types.ID = Field(default_factory=types.ID)
    ts: datetime = Field(default_factory=lambda: datetime.now(tz=timezone.utc))
    memo: str = ""
    comment: str = ""
    entries: list["Entry"] = Field(default_factory=list)


class Entry(Model):
    account: Account
    amount: Decimal
    currency: types.Currency
    direction: types.EntryDirection
