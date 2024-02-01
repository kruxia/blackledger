from http import HTTPStatus
from typing import Optional

from fastapi import APIRouter, HTTPException, Request
from pydantic import ConfigDict, Field, field_serializer, field_validator
from sqly import Q

from blackledger.domain import model, types

from ._search import SearchFilters, SearchParams

router = APIRouter(prefix="/transactions", tags=["transactions"])


class TransactionFilters(SearchFilters):
    tenant_id: Optional[model.IDField] = Field(default=None, alias="tenant")
    tx: Optional[list[model.IDField]] = None
    acct: Optional[list[model.IDField]] = None
    curr: Optional[list[types.CurrencyCode]] = None
    memo: Optional[str] = None

    model_config = ConfigDict(arbitrary_types_allowed=True)

    @field_validator("tx", "acct", "curr", mode="before")
    def convert_list_fields(cls, val):
        if isinstance(val, str):
            return [v.strip() for v in val.split(",")]

    @field_serializer("tx", "acct")
    def serialize_ids(self, val: list[types.ID]):
        return [str(i.to_uuid()) for i in val] if val else None

    @field_serializer("tenant_id")
    def serialize_tenant_id(self, val: types.ID):
        return val.to_uuid()

    def select_filters(self):
        """
        Specify transaction.tenant_id to resolve ambiguity in where clause between
        transaction and entry tables.
        """
        return [
            "transaction.tenant_id = :tenant_id" if "tenant_id" in filter else filter
            for filter in super().select_filters()
        ]


@router.get("")
async def search_transactions(req: Request):
    """
    Search for and list transactions.
    """
    filters = TransactionFilters.from_query(req.query_params)
    search_params = SearchParams.from_query(req.query_params)
    select_params = search_params.select_params()
    tx_query = [
        # filter transaction ids based on transaction and entry fields
        "WITH filtered_tr AS (",
        "  SELECT distinct(transaction.id)",
        "  FROM transaction",
        "  JOIN entry",
        "    ON transaction.id = entry.tx",
    ]
    if filters.query_data():
        tx_query += [
            "  WHERE",
            "\n    AND ".join(filters.select_filters()),
        ]
    tx_query += [
        ")",
        # select matching transactions (only -- entries are separate)
        "SELECT",
        "  DISTINCT tx.*",
        "FROM transaction tx",
        "JOIN filtered_tr",
        "  ON filtered_tr.id = tx.id",
    ]

    if select_params.get("orderby"):
        tx_query.append(f"ORDER BY {select_params['orderby']}")
    if select_params.get("limit"):
        tx_query.append(f"LIMIT {select_params['limit']}")
    if select_params.get("offset"):
        tx_query.append(f"OFFSET {select_params['offset']}")

    # select corresponding entries
    entries_query = """
        SELECT e.*, a.name acct_name FROM entry e
        JOIN transaction t ON e.tx = t.id
        JOIN account a ON e.acct = a.id
        WHERE t.id = ANY(:tx)
    """

    async with req.app.pool.connection() as conn:
        tx_results = await req.app.sql.select_all(
            conn, tx_query, filters.query_data(), Constructor=model.Transaction
        )
        transactions = {tx.id: tx for tx in tx_results}
        entries_params = {"tx": [str(tx.to_uuid()) for tx in transactions.keys()]}
        entries_results = await req.app.sql.select_all(
            conn, entries_query, entries_params, Constructor=model.Entry
        )

    for entry in entries_results:
        transactions[entry.tx].entries.append(entry)

    return list(transactions.values())


@router.post("", status_code=HTTPStatus.CREATED)
async def post_transaction(req: Request, item: model.NewTransaction):
    """
    Post transaction.
    """
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
            acct = await sql.select_one(
                conn,
                sql.queries.SELECT(
                    "account", fields=["version"], filters=[Q.filter("id")]
                ),
                {"id": entry_item.acct},
            )
            if acct is None:
                raise HTTPException(
                    status_code=HTTPStatus.NOT_FOUND,
                    detail=f"Account not found: {entry_item.acct}",
                )

            # if we've updated the acct_version in this transaction, use it (we know the
            # earlier account version was correct because it has been updated.)
            if entry_item.acct in entry_accts_versions:
                entry_item.acct_version = entry_accts_versions[entry_item.acct]

            # ensure that each entry's account.version, if given, is equal to the latest
            # entry for that account (OPTIONAL optimistic locking / concurrency control)
            if entry_item.acct_version and acct["version"] != entry_item.acct_version:
                raise HTTPException(
                    status_code=HTTPStatus.PRECONDITION_FAILED,
                    detail="Entry account_version is out of date",
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
