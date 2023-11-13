from fastapi import APIRouter, Request
from sqly import Q

from blackledger.domain import model

router = APIRouter(prefix="/accounts")


@router.get("", response_model=list[model.Account])
async def search_accounts(req: Request):
    """
    Search for and list accounts.
    """
    async with req.app.pool.connection() as conn:
        results = await req.app.sql.select_all(
            conn, req.app.sql.queries.SELECT("account"), Constructor=model.Account
        )

    return results


@router.post("")
async def edit_accounts(req: Request, item: model.Account):
    """
    Insert/update account.
    """
    sql = req.app.sql
    data = item.dict(exclude_none=True)
    async with req.app.pool.connection() as conn:
        result = await sql.select_one(
            conn,
            f"""
            INSERT INTO account ({Q.fields(data)})
            VALUES ({Q.params(data)})
            ON CONFLICT (id)
            DO UPDATE SET {Q.assigns(data)}
            RETURNING *
            """,
            data,
            Constructor=model.Account,
        )

    return result
