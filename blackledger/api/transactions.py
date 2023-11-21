from typing import Optional

from fastapi import APIRouter, Request
from pydantic import model_validator
from sqly import Q

from blackledger.domain import model, types

from .search import SearchFilters, SearchParams

router = APIRouter(prefix="/transactions")


class TransactionFilters(SearchFilters):
    tx: Optional[model.ModelID] = None
    acct: Optional[model.ModelID] = None
    curr: Optional[types.CurrencyCode] = None
    memo: Optional[str] = None

    class Config:
        arbitrary_types_allowed = True

    @model_validator(mode="after")
    def validate_some_filter(self):
        if not self.query_data():
            raise ValueError("at least one transaction filter must be defined")


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
        "  WHERE",
        "\n    AND ".join(filters.select_filters()),
        ")",
        # select matching transactions and all associated entries
        "SELECT",
        "  tx.id tx_id, tx.posted, tx.memo,",
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
    params = SearchParams.from_query(req.query_params).select_params()
    if params.get("orderby"):
        query.append(f"ORDER BY {params['orderby']}")
    if params.get("limit"):
        query.append(f"LIMIT {params['limit']}")
    if params.get("offset"):
        query.append(f"OFFSET {params['offset']}")

    async with req.app.pool.connection() as conn:
        results = req.app.sql.select(conn, query, filters.query_data())

    # build data with Transaction.id as key
    tx_map = {}
    async for item in results:
        tx_id = types.ID.from_uuid(item["tx_id"])
        if tx_id not in tx_map:
            tx_map[tx_id] = {
                "id": tx_id,
                "posted": item["posted"],
                "memo": item["memo"],
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

    return model.Transaction(entries=entries, **tx)
