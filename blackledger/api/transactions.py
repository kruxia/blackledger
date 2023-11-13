import traceback

from fastapi import APIRouter, HTTPException, Request
from sqly import Q

from blackledger.domain import model, types

router = APIRouter(prefix="/transactions")


@router.get("")
async def search_transactions(req: Request):
    """
    Search for and list transactions.
    """
    async with req.app.pool.connection() as conn:
        results = req.app.sql.select(
            conn,
            """
            SELECT
                tx.id tx_id, tx.posted, tx.memo,
                e.*,
                a.version acct_version
            FROM transaction tx
            JOIN entry e
                ON tx.id = e.tx
            JOIN account a
                ON a.id = e.acct
            """,
        )

    # build data with Transaction.id as key
    tx_map = {}
    async for item in results:
        print(item)
        if item["tx_id"] not in tx_map:
            tx_map[item["tx_id"]] = {
                "id": item["tx_id"],
                "posted": item["posted"],
                "memo": item["memo"],
                "entries": [],
            }
            print(tx_map[item["tx_id"]])
        entry = model.Entry(**item)
        print(entry)
        tx_map[item["tx_id"]]["entries"].append(entry)

    return list(tx_map.values())


@router.post("")
async def post_transaction(req: Request):
    """
    Post transaction.
    """
    try:
        item = model.Transaction(**(await req.json()))
        # the input item has already been validated -- just post it
        sql = req.app.sql
        async with req.app.pool.connection() as conn:  # (creates a db tx context)
            # create the transaction
            tx_data = item.dict(exclude=["entries", "id", "posted"])
            tx = await sql.select_one(
                conn,
                sql.queries.INSERT("transaction", tx_data, returning=True),
                tx_data,
            )

            # create each entry
            entries = []
            for entry_item in item.entries:
                # ensure that each entry's account.version is equal to the latest entry
                # for that account (optimistic locking / concurrency control)
                acct = await sql.select_one(
                    conn,
                    sql.queries.SELECT(
                        "account", fields=["version"], filters=[Q.filter("id")]
                    ),
                    {"id": entry_item.acct},
                )
                assert (
                    acct["version"] == entry_item.acct_version
                ), "entry account_version is out of date"

                entry_item.tx = types.ID.from_uuid(tx["id"])
                entry_data = entry_item.dict(
                    exclude=["acct_version"], exclude_none=True
                )
                entry = await sql.select_one(
                    conn,
                    sql.queries.INSERT("entry", entry_data, returning=True),
                    entry_data,
                    Constructor=model.Entry,
                )
                entries.append(entry)

                # update the associated account - account.version is the entry.id
                await sql.execute(
                    conn,
                    sql.queries.UPDATE(
                        "account", fields=["version"], filters=[Q.filter("id")]
                    ),
                    {"id": entry.acct, "version": entry.id},
                )

    except Exception as exc:
        print(traceback.format_exc())
        raise HTTPException(status_code=400, detail=str(exc))

    return model.Transaction(entries=entries, **tx)
