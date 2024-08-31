from typing import Annotated, Optional

from fastapi import APIRouter, Depends, Request

from blackledger import model, types
from blackledger.db import queries

from ..search import SearchParams

router = APIRouter(prefix="/currencies", tags=["currencies"])


class CurrencyParams(SearchParams):
    code: Optional[types.CurrencyFilter] = None


@router.get("", response_model=list[model.Currency])
async def search_currencies(
    req: Request, params: Annotated[CurrencyParams, Depends(CurrencyParams)]
):
    """
    Search for and list currencies.
    """
    async with req.app.pool.connection() as conn:
        results = await queries.select_currencies(conn, req.app.sql, params)

    return results


@router.post("", response_model=model.Currency)
async def save_currency(req: Request, item: model.Currency):
    """
    Save (insert or update) a currency.
    """
    sql = req.app.sql
    data = item.model_dump(exclude_unset=True)
    query = sql.queries.UPSERT("currency", fields=data, key=["code"], returning=True)
    async with req.app.pool.connection() as conn:
        result = await sql.select_one(conn, query, data, Constructor=model.Currency)

    return result
