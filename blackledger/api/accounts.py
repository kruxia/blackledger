from decimal import Decimal
from typing import Annotated, Optional

from fastapi import APIRouter, Depends, Request
from pydantic import Field, field_validator

from blackledger.db import queries
from blackledger.domain import model, types

from ._search import SearchFilters, SearchParams

router = APIRouter(prefix="/accounts", tags=["accounts"])


class AccountFilters(SearchFilters):
    # allow partial match where applicable
    id: Optional[list[model.IDField]] = None
    name: Optional[types.NameFilter] = None
    tenant_id: Optional[model.IDField] = Field(default=None, alias="tenant")
    parent_id: Optional[model.IDField] = Field(default=None, alias="parent")
    number: Optional[int] = None
    normal: Optional[model.NormalField] = None
    version: Optional[model.IDField] = None

    @field_validator("id", mode="before")
    def convert_list_fields(cls, val):
        if isinstance(val, str):
            return [v.strip() for v in val.split(",")]


@router.get("", response_model=list[model.Account])
async def search_accounts(
    req: Request, filters: Annotated[AccountFilters, Depends(AccountFilters)]
):
    """
    Search for and list accounts.
    """
    params = SearchParams.from_query(req.query_params)
    query = req.app.sql.queries.SELECT(
        "account", filters=filters.select_filters(), **params.select_params()
    )
    async with req.app.pool.connection() as conn:
        results = await req.app.sql.select_all(
            conn, query, filters.query_data(), Constructor=model.Account
        )
    return results


@router.post("", response_model=model.Account)
async def edit_account(req: Request, item: model.Account):
    """
    Insert/update account.
    """
    sql = req.app.sql
    data = item.model_dump(exclude_unset=True)
    query = sql.queries.UPSERT("account", fields=data, key=["id"], returning=True)
    async with req.app.pool.connection() as conn:
        result = await sql.select_one(conn, query, data, Constructor=model.Account)
    return result


@router.get("/balances")
async def get_balances(
    req: Request, filters: Annotated[AccountFilters, Depends(AccountFilters)]
):
    params = SearchParams.from_query(req.query_params)
    async with req.app.pool.connection() as conn:
        results = await queries.select_balances(conn, req.app.sql, filters, params)

    balances = {}
    for result in results:
        account = model.Account(**result)
        if account.id not in balances:
            balances[account.id] = {
                "account": account.model_dump(exclude_none=True),
                "balances": {},
            }
        amount = (
            (result.get("dr") or Decimal(0)) - (result.get("cr") or Decimal(0))
        ) * account.normal
        balances[account.id]["balances"][result["curr"]] = str(amount)

    return list(balances.values())
