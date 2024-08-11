#!/usr/bin/env python
import psycopg_pool

from blackledger import types
from blackledger.settings import DatabaseSettings
from blackledger.tests import fixtures

if __name__ == "__main__":
    settings = DatabaseSettings()
    sql = fixtures.sql()
    with psycopg_pool.ConnectionPool(
        conninfo=settings.url.get_secret_value()
    ) as dbpool:
        fixtures.base_currencies(dbpool)
        ledger_name = f"Sample {types.make_bigid()}"
        ledger = fixtures.base_ledger(dbpool, sql, ledger_name)
        print(ledger)
        accounts = next(fixtures.test_accounts(dbpool, sql, ledger, ""))
        transactions = next(fixtures.test_transactions(dbpool, sql, accounts, ledger))
