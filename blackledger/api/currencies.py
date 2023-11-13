from fastapi import APIRouter, Request
from sqly import Q

from blackledger.domain import model

router = APIRouter(prefix="/currencies")


@router.get("", response_model=list[model.Currency])
async def search_currencies(req: Request):
    """
    Search for and list currencies.
    """
    async with req.app.pool.connection() as conn:
        results = await req.app.sql.select_all(
            conn, req.app.sql.queries.SELECT("currency"), Constructor=model.Currency
        )

    return results


@router.post("", response_model=model.Currency)
async def edit_currencies(req: Request, item: model.Currency):
    """
    Insert/update currency.
    """
    sql = req.app.sql
    fields = model.Currency.model_fields
    async with req.app.pool.connection() as conn:
        result = await sql.select_one(
            conn,
            f"""
            INSERT INTO currency ({Q.fields(fields)})
            VALUES ({Q.params(fields)})
            ON CONFLICT (code)
            DO UPDATE SET {Q.assigns(fields)}
            RETURNING *
            """,
            item.dict(),
            Constructor=model.Currency,
        )

    return result
