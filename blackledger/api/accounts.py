from decimal import Decimal
from typing import Annotated, Optional

from fastapi import APIRouter, Depends, Request
from pydantic import ConfigDict, Field, constr, field_serializer, field_validator

from blackledger.domain import model, types

from ._search import SearchFilters, SearchParams

router = APIRouter(prefix="/accounts", tags=["accounts"])


class AccountFilters(SearchFilters):
    # allow partial match where applicable
    id: Optional[list[model.IDField]] = None
    name: Optional[constr(pattern=r"^\S+$")] = None
    tenant_id: Optional[model.IDField] = Field(default=None, alias="tenant")
    parent_id: Optional[model.IDField] = Field(default=None, alias="parent")
    number: Optional[int] = None
    normal: Optional[model.NormalField] = None
    version: Optional[model.IDField] = None

    model_config = ConfigDict(arbitrary_types_allowed=True)

    @field_validator("id", mode="before")
    def convert_list_fields(cls, val):
        if isinstance(val, str):
            return [v.strip() for v in val.split(",")]

    @field_validator("normal", mode="before")
    def convert_normal(cls, value):
        if isinstance(value, str):
            if value not in types.NormalType.__members__:
                raise ValueError(value)
            value = types.NormalType[value]
        return value

    @field_serializer("id")
    def serialize_ids(self, val: list[types.ID]):
        return [str(i.to_uuid()) for i in val] if val else None

    @field_serializer("normal")
    def serialize_normal(self, val: types.NormalType):
        return val.name


@router.get("", response_model=list[model.Account])
async def search_accounts(
    req: Request, filters: Annotated[AccountFilters, Depends(AccountFilters)]
):
    """
    Search for and list accounts.
    """
    params = SearchParams.from_query(req.query_params).select_params()
    # filters = AccountFilters.from_query(req.query_params)
    query = req.app.sql.queries.SELECT(
        "account", filters=filters.select_filters(), **params
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
    query = [
        "WITH balances AS (",
        "    SELECT a.id account_id,",
        "        e.curr, sum(e.dr) dr, sum(e.cr) cr",
        "    FROM account a",
        "    JOIN entry e",
        "        ON a.id = e.acct",
        "    GROUP BY (a.id, e.curr)",
        ")",
        "SELECT account.*,",
        "    balances.curr, balances.dr, balances.cr",
        "FROM account",
        "JOIN balances",
        "    ON account.id = balances.account_id",
    ]
    select_filters = filters.select_filters()
    if select_filters:
        query += [
            "WHERE",
            "\n    AND ".join(select_filters),
        ]
    select_params = SearchParams.from_query(req.query_params).select_params()
    if select_params.get("orderby"):
        query.append(f" ORDER BY {select_params['orderby']}")
    if select_params.get("limit"):
        query.append(f" LIMIT {select_params['limit']}")
    if select_params.get("offset"):
        query.append(f" OFFSET {select_params['offset']}")

    async with req.app.pool.connection() as conn:
        results = await req.app.sql.select_all(conn, query, filters.query_data())

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
