from http import HTTPStatus
from typing import Annotated, Optional

from fastapi import APIRouter, Depends, HTTPException, Request
from pydantic import Field
from sqly import Q

from blackledger import model, types
from blackledger.db import queries

from ..search import SearchParams

router = APIRouter(prefix="/transactions", tags=["transactions"])


class TransactionParams(SearchParams):
    # entry fields
    tx: Optional[model.BigIDSearchField] = Field(default=None)
    ledger_id: Optional[model.BigIDSearchField] = Field(
        default=None, validation_alias="ledger"
    )
    acct: Optional[model.BigIDSearchField] = Field(default=None)
    curr: Optional[types.CurrencyCode] = Field(default=None)
    # transaction fields
    memo: Optional[str] = Field(default=None)

    def select_filters(self):
        """
        Specify transaction query filters by table to resolve ambiguity.
        """
        data = self.query_data()
        entry_bigid_search = ["tx", "ledger_id", "acct"]
        entry_str = ["curr"]
        transaction_str = ["memo"]
        return (
            [
                f"entry.{field} = ANY(:{field})"
                for field in entry_bigid_search
                if field in data
            ]
            + [f"entry.{field} ~* :{field}" for field in entry_str if field in data]
            + [
                f"transaction.{field} ~* :{field}"
                for field in transaction_str
                if field in data
            ]
        )


@router.get("", response_model=list[model.Transaction])
async def search_transactions(
    req: Request, params: Annotated[TransactionParams, Depends(TransactionParams)]
):
    """
    Search for and list transactions.
    """
    async with req.app.pool.connection() as conn:
        results = await queries.select_transactions(conn, req.app.sql, params)

    return results


@router.post("", status_code=HTTPStatus.CREATED, response_model=model.Transaction)
async def post_transaction(req: Request, item: model.NewTransaction):
    """
    Post a transaction. (Once a transaction has been posted, BlackLedger provides no
    means to alter it: Transaction and Entry records are immutable in the database. To
    change a transaction, issue a new transaction that adjusts or reverses it.)
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

            # if we've updated the version in this transaction, use it (we know the
            # earlier account version was correct because it has been updated.)
            if entry_item.acct in entry_accts_versions:
                entry_item.version = entry_accts_versions[entry_item.acct]

            # ensure that each entry's account.version, if given, is equal to the latest
            # entry for that account (OPTIONAL optimistic locking / concurrency control)
            if entry_item.version and acct["version"] != entry_item.version:
                raise HTTPException(
                    status_code=HTTPStatus.PRECONDITION_FAILED,
                    detail="Entry account_version is out of date",
                )

            entry_item.tx = tx["id"]
            entry_data = entry_item.model_dump(
                exclude=["version"], exclude_none=True
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
