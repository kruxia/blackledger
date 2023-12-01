from typing import Optional

from fastapi import APIRouter, HTTPException, Request
from pydantic import ConfigDict, field_serializer, field_validator
from sqly import Q

from blackledger.domain import model, types

from ._search import SearchFilters, SearchParams

router = APIRouter(prefix="/transactions")


class TransactionFilters(SearchFilters):
    tx: Optional[list[model.ID]] = None
    acct: Optional[list[model.ID]] = None
    curr: Optional[list[types.CurrencyCode]] = None
    memo: Optional[str] = None

    model_config = ConfigDict(arbitrary_types_allowed=True)

    @field_validator("tx", "acct", "curr", mode="before")
    def convert_list_fields(cls, val):
        if isinstance(val, str):
            return [v.strip() for v in val.split(",")]

    @field_serializer("tx", "acct")
    def serialize_ids(self, val: types.ID):
        return [str(i.to_uuid()) for i in val] if val else None


@router.get("")
async def search_transactions(req: Request):
    """
    Search for and list transactions.
    """
    filters = TransactionFilters.from_query(req.query_params)
    search_params = SearchParams.from_query(req.query_params)
    select_params = search_params.select_params()
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
        # select matching transactions (only -- entries are separate)
        "SELECT",
        "  DISTINCT tx.*",
        "FROM transaction tx",
        "JOIN filtered_tr",
        "  ON filtered_tr.id = tx.id",
    ]

    if select_params.get("orderby"):
        query.append(f"ORDER BY {select_params['orderby']}")
    if select_params.get("limit"):
        query.append(f"LIMIT {select_params['limit']}")
    if select_params.get("offset"):
        query.append(f"OFFSET {select_params['offset']}")

    async with req.app.pool.connection() as conn:
        results = await req.app.sql.select_all(
            conn, query, filters.query_data(), Constructor=model.Transaction
        )
        transactions = {tx.id: tx for tx in results}
        print(f"{transactions=}")

    # select corresponding entries
    entries_query = """
        SELECT e.* FROM entry e
        JOIN transaction t ON e.tx = t.id
        WHERE t.id = ANY(:tx)
    """
    entries_query_data = {"tx": [str(tx.to_uuid()) for tx in transactions.keys()]}

    async with req.app.pool.connection() as conn:
        entries = await req.app.sql.select_all(
            conn, entries_query, entries_query_data, Constructor=model.Entry
        )

    for entry in entries:
        transactions[entry.tx].entries.append(entry)

    return list(transactions.values())


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
        tx_data = item.model_dump(exclude=["entries"], exclude_none=True)
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
            entry_data = entry_item.model_dump(
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
            # update the local cache for use later in the transaction
            entry_accts_versions[entry.acct] = entry.id

    return model.Transaction(entries=entries, **tx)
