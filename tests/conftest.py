import json

import psycopg_pool
import pytest
from fastapi.testclient import TestClient
from sqly import SQL

from blackledger.domain import model, types
from blackledger.http import app
from blackledger.settings import DatabaseSettings


@pytest.fixture(scope="session")
def client():
    # "You should use TestClient as a context manager, to ensure that the lifespan is
    # called." --<https://www.starlette.io/lifespan/#running-lifespan-in-tests>
    with TestClient(app) as testclient:
        yield testclient


@pytest.fixture(scope="session")
def dbpool():
    settings = DatabaseSettings()
    with psycopg_pool.ConnectionPool(
        conninfo=settings.url.get_secret_value(), timeout=1.00
    ) as pool:
        yield pool


@pytest.fixture(scope="session")
def sql():
    settings = DatabaseSettings()
    return SQL(dialect=settings.dialect)


@pytest.fixture(scope="session")
def json_dumps():
    class TestJsonEncoder(json.JSONEncoder):
        def default(self, obj):
            if isinstance(obj, (types.ID,)):
                return str(obj)
            return super().default(obj)

    return TestJsonEncoder().encode


# == DATA FIXTURES ==


@pytest.fixture(scope="session", autouse=True)
def base_currencies(dbpool):
    with dbpool.connection() as conn:
        conn.execute("insert into currency (code) values ('USD'), ('CAD'), ('MSFT')")


@pytest.fixture(scope="function")
def base_accounts(dbpool, sql):
    with dbpool.connection() as conn:
        # select base parent accounts, map by name
        accounts = {
            acct.name: acct
            for acct in sql.select_all(
                conn, sql.queries.SELECT("account"), Constructor=model.Account
            )
        }
        return accounts


# -- Balance fixtures


@pytest.fixture(scope="function")
def balance_accounts(dbpool, sql):
    """
    Provide a base set of accounts that is scoped to this particular test function.
    """
    accounts = {}
    run_id = types.ID()
    with dbpool.connection() as conn:
        # map accounts by basename
        for basename, normal in [
            ("Asset", "DR"),
            ("Expense", "DR"),
            ("Liability", "CR"),
            ("Income", "CR"),
            ("Equity", "CR"),
        ]:
            name = f"{basename}-{run_id}"
            accounts |= {
                basename: acct
                for acct in sql.select_all(
                    conn,
                    sql.queries.INSERT("account", ["name", "normal"], returning=True),
                    {"name": name, "normal": normal},
                    Constructor=model.Account,
                )
            }

    yield accounts


@pytest.fixture(scope="function")
def balance_transactions(dbpool, sql, balance_accounts):
    accts = balance_accounts
    transactions = []
    with dbpool.connection() as conn:
        for item in [
            # Earn some income from 2 sources
            {
                "memo": "client1",
                "entries": [
                    {"acct": accts["Asset"].id, "dr": "1000", "curr": "USD"},
                    {"acct": accts["Income"].id, "cr": "1000", "curr": "USD"},
                ],
            },
            {
                "memo": "client2",
                "entries": [
                    {"acct": accts["Asset"].id, "dr": "1500", "curr": "USD"},
                    {"acct": accts["Income"].id, "cr": "1500", "curr": "USD"},
                ],
            },
            # Pay for lunch out in Canada
            {
                "memo": "lunch",
                "entries": [
                    {"acct": accts["Expense"].id, "dr": "20", "curr": "CAD"},
                    {"acct": accts["Asset"].id, "cr": "15", "curr": "USD"},
                    {"acct": accts["Equity"].id, "dr": "15", "curr": "USD"},
                    {"acct": accts["Equity"].id, "cr": "20", "curr": "CAD"},
                ],
            },
            # Pay for dinner out in Canada
            {
                "memo": "dinner",
                "entries": [
                    {"acct": accts["Expense"].id, "dr": "28", "curr": "CAD"},
                    {"acct": accts["Asset"].id, "cr": "21", "curr": "USD"},
                    {"acct": accts["Equity"].id, "dr": "21", "curr": "USD"},
                    {"acct": accts["Equity"].id, "cr": "28", "curr": "CAD"},
                ],
            },
            # Buy some MSFT stock
            {
                "memo": "5 MSFT @ 377.43 USD",
                "entries": [
                    {"acct": accts["Asset"].id, "dr": "5", "curr": "MSFT"},
                    {"acct": accts["Asset"].id, "cr": "1887", "curr": "USD"},
                    {"acct": accts["Equity"].id, "cr": "5", "curr": "MSFT"},
                    {"acct": accts["Equity"].id, "dr": "1887", "curr": "USD"},
                ],
            },
        ]:
            tx = sql.select_one(
                conn,
                sql.queries.INSERT("transaction", ["memo"], returning=True),
                {"memo": item["memo"]},
            )
            for new_entry in item["entries"]:
                new_entry["tx"] = tx["id"]
                entry = sql.select_one(
                    conn,
                    sql.queries.INSERT("entry", new_entry, returning=True),
                    new_entry,
                )
                sql.execute(
                    conn,
                    sql.queries.UPDATE("account", ["version"], [sql.Q.filter("id")]),
                    {"id": entry["acct"], "version": entry["id"]},
                )
            transactions.append(tx)

    yield transactions
