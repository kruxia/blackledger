import logging
from decimal import Decimal
from typing import Annotated, Optional

from fastapi import APIRouter, Depends, Request
from pydantic import Field

from blackledger import model, types
from blackledger.db import queries

from ..search import SearchParams

router = APIRouter(prefix="/accounts", tags=["accounts"])

LOG = logging.getLogger(__name__)


class AccountParams(SearchParams):
    # allow partial match where applicable
    id: Optional[model.BigIDSearchField] = None
    name: Optional[types.NameFilter] = None
    ledger_id: Optional[model.BigIDSearchField] = Field(
        default=None, validation_alias="ledger"
    )
    parent_id: Optional[model.BigIDSearchField] = Field(
        default=None, validation_alias="parent"
    )
    number: Optional[int] = None
    normal: Optional[model.NormalField] = None
    version: Optional[model.BigIDSearchField] = None


@router.get("", response_model=list[model.Account])
async def search_accounts(
    req: Request, params: Annotated[AccountParams, Depends(AccountParams)]
):
    """
    Search for and list accounts.
    """
    print(f"{req.url=}")
    print(f"{params=}")
    print(f"{params.select_filters()=}")
    print(f"{params.select_params()=}")
    query = req.app.sql.queries.SELECT(
        "account", filters=params.select_filters(), **params.select_params()
    )
    print("%s\n\t%r" % req.app.sql.render(query, params.query_data()))
    async with req.app.pool.connection() as conn:
        results = await req.app.sql.select_all(
            conn, query, params.query_data(), Constructor=model.Account
        )
    print(results)
    return [account.model_dump(exclude_none=True) for account in results]


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
    req: Request, params: Annotated[AccountParams, Depends(AccountParams)]
):
    async with req.app.pool.connection() as conn:
        results = await queries.select_balances(conn, req.app.sql, params)

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
