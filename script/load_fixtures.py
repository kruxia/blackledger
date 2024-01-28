#!/usr/bin/env python
import psycopg_pool

from blackledger.domain import types
from blackledger.settings import DatabaseSettings
from blackledger.tests import fixtures

if __name__ == "__main__":
    settings = DatabaseSettings()
    sql = fixtures.sql()
    with psycopg_pool.ConnectionPool(
        conninfo=settings.url.get_secret_value()
    ) as dbpool:
        fixtures.base_currencies(dbpool)
        tenant_name = f"Sample {types.ID()}"
        tenant = fixtures.base_tenant(dbpool, sql, tenant_name)
        print(tenant)
        accounts = next(fixtures.test_accounts(dbpool, sql, tenant, ""))
        transactions = next(fixtures.test_transactions(dbpool, sql, accounts, tenant))
