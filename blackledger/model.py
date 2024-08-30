from datetime import datetime
from decimal import Decimal
from random import randint
from typing import Optional

import orjson
from pydantic import (
    BaseModel,
    ConfigDict,
    Field,
    PlainSerializer,
    field_validator,
    model_validator,
)
from pydantic.functional_validators import BeforeValidator
from typing_extensions import Annotated

from . import types

CurrencyField = Annotated[types.CurrencyCode, Field(examples=["USD", "CAD", "GOOG"])]

BigIDField = Annotated[
    int,
    Field(examples=randint(1001, 1_000_000)),
]
BigIDSearchField = Annotated[
    str,
    Field(pattern=r"^[0-9]+(,[0-9]+)*$"),
    PlainSerializer(lambda val: [int(v) for v in val.split(",")] if val else None),
]

NormalField = Annotated[
    types.NormalType,
    Field(examples=[types.NormalType.DR.name]),
    BeforeValidator(types.NormalType.from_str),
    PlainSerializer(types.NormalType.to_str),
]
NameField = Annotated[
    types.NameString,
    Field(examples=["Some-Name"]),
]


class Model(BaseModel):
    model_config = ConfigDict(arbitrary_types_allowed=True)

    def json(self, **kwargs):
        return orjson.dumps(self.model_dump(**kwargs))


class Currency(Model):
    code: CurrencyField


class Account(Model):
    id: Optional[BigIDField] = None
    ledger_id: BigIDField
    parent_id: Optional[BigIDField] = None
    name: NameField
    normal: NormalField
    number: Optional[int] = None
    version: Optional[BigIDField] = None


class Entry(Model):
    id: Optional[BigIDField] = None
    ledger_id: BigIDField
    tx: Optional[BigIDField] = None
    acct: BigIDField
    acct_name: Optional[str] = None
    dr: Optional[Decimal] = None
    cr: Optional[Decimal] = None
    curr: CurrencyField

    @field_validator("dr", "cr", mode="before")
    def convert_dr_cr(cls, value):
        """
        Convert int and float amounts to Decimal
        """
        if isinstance(value, (int, float)):
            value = Decimal(str(value))
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
            return self.dr * types.NormalType.DR
        else:
            return self.cr * types.NormalType.CR


class NewEntry(Entry):
    acct_version: Optional[BigIDField] = None


class Transaction(Model):
    id: Optional[BigIDField] = None
    ledger_id: BigIDField
    posted: Optional[datetime] = None
    effective: Optional[datetime] = None
    memo: Optional[str] = None
    meta: Optional[dict] = None
    entries: list[Entry] = Field(default_factory=list)

    @model_validator(mode="after")
    def balanced_entries(self):
        """
        For each used currency, the sum of entries in the transaction must be 0.
        """
        sums = {}
        for e in self.entries:
            sums[e.curr] = sums.setdefault(e.curr, Decimal("0")) + e.amount()
        assert all(
            v == Decimal("0") for v in sums.values()
        ), "transaction is unbalanced"
        return self

    @model_validator(mode="after")
    def one_ledger(self):
        """
        A transaction and its entries must belong to a single ledger.
        """
        assert all(
            e.ledger_id == self.ledger_id for e in self.entries
        ), "transaction entries must belong to the same ledger as the transaction"
        return self


class NewTransaction(Transaction):
    entries: list[NewEntry] = Field(default_factory=list)


class Ledger(Model):
    id: Optional[BigIDField] = None
    name: NameField
    created: Optional[datetime] = None
