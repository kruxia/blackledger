from typing import Optional

from fastapi import APIRouter, HTTPException, Request

# from pydantic import model_validator
from sqly import Q

from blackledger.domain import model, types

from ._search import SearchFilters, SearchParams

router = APIRouter(prefix="/transactions")


class TransactionFilters(SearchFilters):
    tx: Optional[model.ID] = None
    acct: Optional[model.ID] = None
    curr: Optional[types.CurrencyCode] = None
    memo: Optional[str] = None

    class Config:
        arbitrary_types_allowed = True


@router.get("")
async def search_transactions(req: Request):
    """
    Search for and list transactions.
    """
    filters = TransactionFilters.from_query(req.query_params)
    query = [
        # filter transaction ids based on transaction and entry fields
        "WITH filtered_tr AS (",
        "  SELECT distinct(transaction.id)",
        "  FROM transaction",
        "  JOIN entry",
        "    ON transaction.id = entry.tx",
    ]
    if filters.query_data():
        query += [
            "  WHERE",
            "\n    AND ".join(filters.select_filters()),
        ]
    query += [
        ")",
        # select matching transactions and all associated entries
        "SELECT",
        "  tx.id tx_id, tx.posted, tx.effective, tx.memo, tx.meta,",
        "  e.*,",
        "  a.version acct_version",
        "FROM transaction tx",
        "JOIN filtered_tr",
        "  ON filtered_tr.id = tx.id",
        "JOIN entry e",
        "  ON tx.id = e.tx",
        "JOIN account a",
        "  ON a.id = e.acct",
    ]
    select_params = SearchParams.from_query(req.query_params).select_params()
    if select_params.get("orderby"):
        query.append(f"ORDER BY {select_params['orderby']}")
    if select_params.get("limit"):
        query.append(f"LIMIT {select_params['limit']}")
    if select_params.get("offset"):
        query.append(f"OFFSET {select_params['offset']}")

    async with req.app.pool.connection() as conn:
        results = req.app.sql.select(conn, query, filters.query_data())

    # build data with Transaction.id as key
    tx_map = {}
    async for item in results:
        tx_id = item["tx_id"]
        if tx_id not in tx_map:
            tx_map[tx_id] = {
                "id": tx_id,
                "posted": item["posted"],
                "effective": item["effective"],
                "memo": item["memo"],
                "meta": item["meta"],
                "entries": [],
            }
        entry = model.Entry(**item)
        tx_map[tx_id]["entries"].append(entry)

    return list(tx_map.values())


@router.post("")
async def post_transaction(req: Request):
    """
    Post transaction.
    """
    item = model.NewTransaction(**(await req.json()))
    # the input item has been validated -- just post it
    sql = req.app.sql
    async with req.app.pool.connection() as conn:  # (creates a db tx context)
        # create the transaction
        tx_data = item.dict(exclude=["entries"], exclude_none=True)
        tx = await sql.select_one(
            conn,
            sql.queries.INSERT("transaction", tx_data, returning=True),
            tx_data,
        )

        # create each entry
        entries = []
        entry_accts_versions = {}
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
            if acct is None:
                raise HTTPException(
                    status_code=404, detail=f"Account not found: {entry_item.acct}"
                )

            # if we've updated the acct_version in this transaction, use it
            if entry_item.acct in entry_accts_versions:
                entry_item.acct_version = entry_accts_versions[entry_item.acct]

            if acct["version"] != entry_item.acct_version:
                raise HTTPException(
                    status_code=409, detail="Entry account_version is out of date"
                )

            entry_item.tx = tx["id"]
            entry_data = entry_item.dict(exclude=["acct_version"], exclude_none=True)
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
            # update the local cache for use later in the transaction
            entry_accts_versions[entry.acct] = entry.id

    return model.Transaction(entries=entries, **tx)
