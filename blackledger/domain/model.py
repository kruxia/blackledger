from datetime import datetime
from decimal import Decimal
from typing import Optional

import orjson
from pydantic import (
    BaseModel,
    Field,
    field_serializer,
    field_validator,
    model_validator,
)
from pydantic.functional_validators import BeforeValidator
from typing_extensions import Annotated

from . import types

ModelID = Annotated[types.ID, BeforeValidator(types.ID.field_converter)]


class Model(BaseModel):
    class Config:
        arbitrary_types_allowed = True

    def json(self):
        return orjson.dumps(self.dict())


class Currency(Model):
    code: types.CurrencyCode


class Account(Model):
    id: Optional[ModelID] = None
    name: str
    parent_id: Optional[ModelID] = None
    num: Optional[int] = None
    normal: types.Normal
    curr: Optional[types.CurrencyCode] = None
    version: Optional[ModelID] = None

    @field_validator("normal", mode="before")
    def convert_normal(cls, value):
        if isinstance(value, str):
            if value not in types.Normal.__members__:
                raise ValueError(value)
            value = types.Normal[value]
        return value

    @field_serializer("normal")
    def serialize_normal(self, val: types.Normal):
        return val.name


class Entry(Model):
    id: Optional[ModelID] = None
    tx: Optional[ModelID] = None
    acct: ModelID
    acct_version: Optional[ModelID] = None
    dr: Optional[Decimal] = None
    cr: Optional[Decimal] = None
    curr: types.CurrencyCode

    @field_validator("dr", "cr", mode="before")
    def validate_dr_cr(cls, value):
        if isinstance(value, float):
            raise ValueError(
                "floating point data is not allowed: use a Decimal or string"
            )
        return value

    @model_validator(mode="before")
    @classmethod
    def check_amounts(cls, data):
        assert data.get("dr") or data.get("cr"), "either dr or cr must be defined"
        assert not (
            data.get("dr") and data.get("cr")
        ), "both dr and cr cannot be defined"
        return data

    @field_validator("cr", "dr", mode="after")
    def check_amounts_valid(cls, value):
        assert (
            value is None or value > 0
        ), "amount must be greater than zero: accountants hate negatives"
        return value

    def amount(self):
        """
        Return self.dr or self.cr as a positive or negative decimal. Convention:

        * Debits (DR) are positive
        * Credits (CR) are negative
        """
        if self.dr:
            return self.dr * types.Normal.DR
        if self.cr:
            return self.cr * types.Normal.CR

        raise ValueError("Invalid Entry: dr and cr are both undefined")


class Transaction(Model):
    id: Optional[ModelID] = None
    posted: Optional[datetime] = None
    memo: Optional[str] = None
    entries: list[Entry] = Field(default_factory=list)

    # @model_validator(mode="before")
    # @classmethod
    # def check_entries(cls, data):
    #     assert (
    #         data.get("entries") and len(data["entries"]) >= 2
    #     ), "transactions must have at least two entries"
    #     if data.get("id") and data.get("entries"):
    #         for entry in data["entries"]:
    #             entry.tx = data["id"]
    #     return data

    @model_validator(mode="after")
    def balanced_entries(self):
        assert sum(e.amount() for e in self.entries) == 0, "transaction is unbalanced"
