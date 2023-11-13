from datetime import datetime, timezone
from decimal import Decimal
from typing import ForwardRef, Optional
from uuid import UUID

import orjson
from pydantic import BaseModel, Field, field_serializer, model_validator

from . import types


class Model(BaseModel):
    class Config:
        arbitrary_types_allowed = True

    def json(self):
        return orjson.dumps(self.dict())


class Currency(Model):
    code: types.CurrencyCode


class Account(Model):
    id: types.ID = Field(default_factory=types.ID)
    name: str
    normal: types.Normal
    parent: Optional["Account"] = None
    currency: Optional[types.CurrencyCode] = None

    @model_validator(mode="before")
    def convert_data(cls, data):
        item = {k: v for k, v in data.items()}
        if "id" in data and type(data["id"]) == UUID:
            item["id"] = types.ID.from_uuid(data["id"])
        if "currency" in data and isinstance(data["currency"], str):
            item["currency"] = types.CurrencyCode(data["currency"])

        if (
            not isinstance(data["normal"], types.Normal)
            and data["normal"] in types.Normal.__members__
        ):
            item["normal"] = types.Normal[data["normal"]]

        return item

    # @field_serializer("id")
    # def serialize_id(self, val: types.ID):
    #     return str(val)

    @field_serializer("normal")
    def serialize_normal(self, val: types.Normal):
        return val.name


Entry = ForwardRef("Entry")


class Transaction(Model):
    id: types.ID = Field(default_factory=types.ID)
    ts: datetime = Field(default_factory=lambda: datetime.now(tz=timezone.utc))
    memo: str = ""
    entries: list[Entry] = Field(default_factory=list)

    @model_validator(mode="before")
    def convert_data(cls, data):
        item = {k: v for k, v in data.items()}
        if "id" in data and type(data["id"]) == UUID:
            item["id"] = types.ID.from_uuid(data["id"])
        return item

    def balance(self):
        return sum((e.amount.decimal * e.direction) for e in self.entries)

    def currency(self):
        return next(iter(e.amount.currency for e in self.entries), None)

    @field_serializer("id")
    def serialize_id(self, val: types.ID):
        return str(val)


class Amount(Model):
    decimal: Decimal
    currency: types.CurrencyCode

    @model_validator(mode="before")
    def initialize_currency(cls, data):
        if not isinstance(data["currency"], types.CurrencyCode):
            data["currency"] = types.CurrencyCode(data["currency"])

        return data

    @field_serializer("decimal")
    def serialize_decimal(self, val: Decimal):
        return str(val)


class Entry(Model):
    account: Account
    amount: Amount
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
