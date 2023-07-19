from datetime import datetime, timezone
from decimal import Decimal
from typing import Optional

from pydantic import BaseModel, Field, model_validator

from . import types


class Model(BaseModel):
    class Config:
        arbitrary_types_allowed = True


class Account(Model):
    id: types.ID = Field(default_factory=types.ID)
    name: str
    normal: types.Normal
    parent: Optional["Account"] = None
    currency: Optional[types.Currency] = None

    @model_validator(mode="before")
    def initialize_currency(cls, data):
        print(data)
        if 'currency' in data and isinstance(data["currency"], str):
            data["currency"] = types.Currency(data["currency"])

        return data


class Transaction(Model):
    id: types.ID = Field(default_factory=types.ID)
    ts: datetime = Field(default_factory=lambda: datetime.now(tz=timezone.utc))
    memo: str = ""
    comment: str = ""
    entries: list["Entry"] = Field(default_factory=list)

    def balance(self):
        return sum((e.amount.decimal * e.direction) for e in self.entries)

    def currency(self):
        return next(iter(e.amount.currency for e in self.entries), None)


class Amount(Model):
    decimal: Decimal
    currency: types.Currency

    @model_validator(mode="before")
    def initialize_currency(cls, data):
        if isinstance(data["currency"], str):
            data["currency"] = types.Currency(data["currency"])

        return data

    def __str__(self):
        return f"{self.decimal} {self.currency}"


class Entry(Model):
    account: Account
    amount: Amount
    direction: types.Direction
    # -- later --
    # rate: Optional[Amount]  # -- exchange rate
    # basis: Optional[Amount]  # -- cost basis
