from typing import Annotated, Optional

from fastapi import APIRouter, Depends, Request
from pydantic import Field

from blackledger import model, types
from blackledger.db import queries

from ..search import SearchParams

router = APIRouter(prefix="/ledgers", tags=["ledgers"])


class LedgerParams(SearchParams):
    # allow partial match where applicable
    id: Optional[model.BigIDSearchField] = Field(default=None)
    name: Optional[types.NameFilter] = Field(default=None)


@router.get("", response_model=list[model.Ledger])
async def search_ledgers(
    req: Request, params: Annotated[LedgerParams, Depends(LedgerParams)]
):
    """
    Search for and list ledgers.
    """
    async with req.app.pool.connection() as conn:
        results = await queries.select_ledgers(conn, req.app.sql, params)

    return results


@router.post("", response_model=model.Ledger)
async def save_ledger(req: Request, item: model.Ledger):
    """
    Save (insert or update) a ledger.
    """
    sql = req.app.sql
    data = item.model_dump(exclude_unset=True)
    query = sql.queries.UPSERT("ledger", fields=data, key=["id"], returning=True)
    async with req.app.pool.connection() as conn:
        result = await sql.select_one(conn, query, data, Constructor=model.Ledger)
    return result
