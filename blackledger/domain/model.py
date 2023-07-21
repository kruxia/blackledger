from datetime import datetime, timezone
from decimal import Decimal
from typing import Optional

import orjson
from pydantic import BaseModel, Field, field_serializer, model_validator

from . import types


class Model(BaseModel):
    class Config:
        arbitrary_types_allowed = True

    def json(self):
        return orjson.dumps(self.dict())


class Account(Model):
    id: types.ID = Field(default_factory=types.ID)
    name: str
    normal: types.Normal
    parent: Optional["Account"] = None
    currency: Optional[types.Currency] = None

    @model_validator(mode="before")
    def convert_data(cls, data):
        if "currency" in data and isinstance(data["currency"], str):
            data["currency"] = types.Currency(data["currency"])

        if (
            not isinstance(data["normal"], types.Normal)
            and data["normal"] in types.Normal.__members__
        ):
            data["normal"] = types.Normal[data["normal"]]

        return data

    # @field_serializer("id")
    # def serialize_id(self, val: types.ID):
    #     return str(val)

    @field_serializer("normal")
    def serialize_normal(self, val: types.Normal):
        return val.name


class Transaction(Model):
    id: types.ID = Field(default_factory=types.ID)
    ts: datetime = Field(default_factory=lambda: datetime.now(tz=timezone.utc))
    memo: str = ""
    entries: list["Entry"] = Field(default_factory=list)

    def balance(self):
        return sum((e.amount.decimal * e.direction) for e in self.entries)

    def currency(self):
        return next(iter(e.amount.currency for e in self.entries), None)

    @field_serializer("id")
    def serialize_id(self, val: types.ID):
        return str(val)


class Amount(Model):
    decimal: Decimal
    currency: types.Currency

    @model_validator(mode="before")
    def initialize_currency(cls, data):
        if not isinstance(data["currency"], types.Currency):
            data["currency"] = types.Currency(data["currency"])

        return data

    @field_serializer("decimal")
    def serialize_decimal(self, val: Decimal):
        return str(val)


class Entry(Model):
    account: Account
    amount: Amount
    direction: types.Direction
    # -- later --
    # rate: Optional[Amount]  # -- exchange rate
    # basis: Optional[Amount]  # -- cost basis

    @model_validator(mode="before")
    def convert_data(cls, data):
        if (
            not isinstance(data["direction"], types.Direction)
            and data["direction"] in types.Direction.__members__
        ):
            data["direction"] = types.Direction[data["direction"]]
        return data

    @field_serializer("direction")
    def serialize_direction(self, val: types.Normal):
        return val.name
