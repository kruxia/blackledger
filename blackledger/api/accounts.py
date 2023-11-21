from decimal import Decimal
from typing import Optional

from fastapi import APIRouter, Request
from pydantic import field_serializer, field_validator

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
    query = sql.queries.UPSERT("account", fields=data, key=["id"], returning=True)
    async with req.app.pool.connection() as conn:
        result = await sql.select_one(conn, query, data, Constructor=model.Account)

    return result


@router.get("/balances")
async def get_accounts_balances(req: Request):
    query = """
        WITH balances AS (
            SELECT a.id account_id,
                e.curr, sum(e.dr) dr, sum(e.cr) cr
            FROM account a
            JOIN entry e
                ON a.id = e.acct
            GROUP BY (a.id, a.name, e.curr)
        )
        SELECT account.*,
            balances.curr, balances.dr, balances.cr
        FROM account
        JOIN balances
            ON account.id = balances.account_id
    """
    filters = AccountFilters.from_query(req.query_params)
    select_filters = filters.select_filters()
    if select_filters:
        query += f"""
        WHERE
            {" AND ".join(select_filters)}
        """
    params = SearchParams.from_query(req.query_params).select_params()
    if params.get("orderby"):
        query += f" ORDER BY {params['orderby']}"
    if params.get("limit"):
        query += f" LIMIT {params['limit']}"
    if params.get("offset"):
        query += f" OFFSET {params['offset']}"

    async with req.app.pool.connection() as conn:
        results = await req.app.sql.select_all(conn, query, filters.query_data())

    balances = {}
    for result in results:
        if result["id"] not in balances:
            account = model.Account(**result)
            print(account)
            balances[result["id"]] = {
                "account": model.Account(**result),
                "balances": {},
            }
        account = balances[result["id"]]["account"]
        amt = (
            (result.get("dr") or Decimal(0)) - (result.get("cr") or Decimal(0))
        ) * Decimal(account.normal)
        balances[result["id"]]["balances"][result["curr"]] = str(amt)

    return list(balances.values())
