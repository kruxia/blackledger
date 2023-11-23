from fastapi import APIRouter, Request

from blackledger.domain import model

from ._search import SearchFilters, SearchParams

router = APIRouter(prefix="/currencies")


class CurrencyFilters(SearchFilters):
    code: str = None


@router.get("", response_model=list[model.Currency])
async def search_currencies(req: Request):
    """
    Search for and list currencies.
    """
    filters = CurrencyFilters.from_query(req.query_params)
    search_params = SearchParams.from_query(req.query_params).select_params()
    query = req.app.sql.queries.SELECT(
        "currency", filters=filters.select_filters(), **search_params
    )
    async with req.app.pool.connection() as conn:
        results = await req.app.sql.select_all(
            conn,
            query,
            filters.model_dump(exclude_none=True),
            Constructor=model.Currency,
        )

    return results


@router.post("", response_model=model.Currency)
async def edit_currencies(req: Request, item: model.Currency):
    """
    Insert/update currency.
    """
    sql = req.app.sql
    data = item.model_dump(exclude_none=True)
    query = sql.queries.UPSERT("currency", fields=data, key=["code"], returning=True)
    async with req.app.pool.connection() as conn:
        result = await sql.select_one(conn, query, data, Constructor=model.Currency)

    return result
