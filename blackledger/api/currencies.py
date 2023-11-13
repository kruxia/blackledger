from http import HTTPStatus

from fastapi import APIRouter, Request
from sqly import Q

from blackledger.domain import model

router = APIRouter(prefix="/currencies")


@router.get("")
async def search_currencies(req: Request):
    """
    Search for and list currencies.
    """
    sql = req.app.sql
    async with req.app.pool.connection() as conn:
        results = await sql.select_all(conn, sql.queries.SELECT("currency"))

    return results


@router.post("", status_code=HTTPStatus.ACCEPTED)
async def edit_currencies(req: Request, item: model.Currency):
    """
    Insert/update supported currencies.
    """
    sql = req.app.sql
    fields = model.Currency.model_fields
    async with req.app.pool.connection() as conn:
        result = await sql.select_one(
            conn,
            f"""
            INSERT INTO currency ({Q.fields(fields)})
            VALUES ({Q.params(fields)})
            ON CONFLICT (code) DO
            UPDATE SET {Q.assigns(fields)}
            RETURNING *
            """,
            item.dict(),
            Constructor=model.Currency,
        )

    return result
