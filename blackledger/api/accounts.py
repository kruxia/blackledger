from typing import Optional

from fastapi import APIRouter, Request
from pydantic import field_serializer, field_validator
from sqly import Q

from blackledger.domain import model, types

from .search import SearchFilters, SearchParams

router = APIRouter(prefix="/accounts")


class AccountFilters(SearchFilters):
    # allow partial match where applicable
    id: Optional[model.ModelID] = None
    name: Optional[types.NameString] = None
    parent_id: Optional[model.ModelID] = None
    num: Optional[int] = None
    normal: Optional[types.Normal] = None  # NOT WORKING
    curr: Optional[types.CurrencyCode] = None
    version: Optional[model.ModelID] = None

    class Config:
        arbitrary_types_allowed = True

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


@router.get("", response_model=list[model.Account])
async def search_accounts(req: Request):
    """
    Search for and list accounts.
    """
    filters = AccountFilters.from_query(req.query_params)
    params = SearchParams.from_query(req.query_params).select_params()
    query = req.app.sql.queries.SELECT(
        "account", filters=filters.select_filters(), **params
    )
    async with req.app.pool.connection() as conn:
        results = await req.app.sql.select_all(
            conn, query, filters.query_data(), Constructor=model.Account
        )

    return results


@router.post("", response_model=model.Account)
async def edit_accounts(req: Request, item: model.Account):
    """
    Insert/update account.
    """
    sql = req.app.sql
    data = item.dict(exclude_none=True)
    key = ["id"]
    async with req.app.pool.connection() as conn:
        result = await sql.select_one(
            conn,
            f"""
            INSERT INTO account ({Q.fields(data)})
            VALUES ({Q.params(data)})
            ON CONFLICT ({Q.params(key)})
            DO UPDATE SET {Q.assigns(data)}
            RETURNING *
            """,
            data,
            Constructor=model.Account,
        )

    return result
