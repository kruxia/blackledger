from typing import Annotated, Optional

from fastapi import APIRouter, Depends, Request

from blackledger.db import queries
from blackledger.domain import model, types

from ._search import SearchFilters, SearchParams

router = APIRouter(prefix="/currencies", tags=["currencies"])


class CurrencyFilters(SearchFilters):
    code: Optional[types.CurrencyFilter] = None


@router.get("", response_model=list[model.Currency])
async def search_currencies(
    req: Request, filters: Annotated[CurrencyFilters, Depends(CurrencyFilters)]
):
    """
    Search for and list currencies.
    """
    params = SearchParams.from_query(req.query_params)
    async with req.app.pool.connection() as conn:
        results = await queries.select_currencies(conn, req.app.sql, filters, params)

    return results


@router.post("", response_model=model.Currency)
async def edit_currencies(req: Request, item: model.Currency):
    """
    Insert/update currency.
    """
    sql = req.app.sql
    data = item.model_dump(exclude_unset=True)
    query = sql.queries.UPSERT("currency", fields=data, key=["code"], returning=True)
    async with req.app.pool.connection() as conn:
        result = await sql.select_one(conn, query, data, Constructor=model.Currency)

    return result
