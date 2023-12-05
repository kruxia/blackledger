from datetime import datetime
from decimal import Decimal
from typing import Optional

import orjson
from pydantic import (
    BaseModel,
    ConfigDict,
    Field,
    field_serializer,
    field_validator,
    model_validator,
)
from pydantic.functional_validators import BeforeValidator
from typing_extensions import Annotated

from . import types

ID = Annotated[types.ID, BeforeValidator(types.ID.field_converter)]


class Model(BaseModel):
    model_config = ConfigDict(arbitrary_types_allowed=True)

    def json(self):
        return orjson.dumps(self.model_dump())


class Currency(Model):
    code: types.CurrencyCode


class Account(Model):
    id: Optional[ID] = None
    tenant_id: ID
    parent_id: Optional[ID] = None
    name: types.NameString
    normal: types.Normal
    number: Optional[int] = None
    version: Optional[ID] = None

    @field_validator("normal", mode="before")
    def convert_normal(cls, value):
        if isinstance(value, str):
            if value not in types.Normal.__members__:
                raise ValueError(value)
            value = types.Normal[value]
        return value

    # @field_serializer("id")
    # def serialize_id(self, val: ID):
    #     return str(val)

    @field_serializer("normal")
    def serialize_normal(self, val: types.Normal):
        return val.name


class Entry(Model):
    id: Optional[ID] = None
    tenant_id: ID
    tx: Optional[ID] = None
    acct: ID
    dr: Optional[Decimal] = None
    cr: Optional[Decimal] = None
    curr: types.CurrencyCode

    @field_validator("dr", "cr", mode="before")
    def validate_dr_cr(cls, value):
        if isinstance(value, (int, float)):
            raise ValueError(
                "integer and floating point amounts are not allowed: "
                + "use string or Decimal"
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
        else:
            return self.cr * types.Normal.CR


class NewEntry(Entry):
    acct_version: Optional[ID] = None


class Transaction(Model):
    id: Optional[ID] = None
    tenant_id: ID
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
    def one_tenant(self):
        """
        A transaction and its entries must belong to a single tenant.
        """
        assert all(
            e.tenant_id == self.tenant_id for e in self.entries
        ), "transaction entries must belong to the same tenant as the transaction"
        return self


class NewTransaction(Transaction):
    entries: list[NewEntry] = Field(default_factory=list)


class Tenant(Model):
    id: Optional[ID] = None
    name: types.NameString
    created: Optional[datetime] = None
