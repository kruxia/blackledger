from http import HTTPStatus
from typing import Annotated, Optional

from fastapi import APIRouter, Depends, HTTPException, Request
from pydantic import Field
from sqly import Q

from blackledger import model, types
from blackledger.db import queries

from ._search import SearchFilters, SearchParams

router = APIRouter(prefix="/transactions", tags=["transactions"])


class TransactionFilters(SearchFilters):
    tenant_id: Optional[model.IDSearchField] = Field(default=None, alias="tenant")
    tx: Optional[model.IDSearchField] = None
    acct: Optional[model.IDSearchField] = None
    curr: Optional[types.CurrencyCode] = None
    memo: Optional[str] = None

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
async def search_transactions(
    req: Request, filters: Annotated[TransactionFilters, Depends(TransactionFilters)]
):
    """
    Search for and list transactions.
    """
    # filters = TransactionFilters.from_query(req.query_params)
    params = SearchParams.from_query(req.query_params)

    async with req.app.pool.connection() as conn:
        results = await queries.select_transactions(conn, req.app.sql, filters, params)

    return results


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
