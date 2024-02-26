import json
from datetime import datetime

import psycopg_pool
from fastapi.testclient import TestClient
from sqly import SQL

from blackledger import model, types
from blackledger.http import app
from blackledger.settings import DatabaseSettings


def client():
    # "You should use TestClient as a context manager, to ensure that the lifespan is
    # called." --<https://www.starlette.io/lifespan/#running-lifespan-in-tests>
    app.settings.auth.disabled = True
    with TestClient(app) as testclient:
        yield testclient


def auth_client(client):
    app.settings.auth.disabled = False
    yield client


def dbpool():
    settings = DatabaseSettings()
    with psycopg_pool.ConnectionPool(
        conninfo=settings.url.get_secret_value(), timeout=1.00
    ) as pool:
        yield pool


def sql():
    settings = DatabaseSettings()
    return SQL(dialect=settings.dialect)


def json_dumps():
    class TestJsonEncoder(json.JSONEncoder):
        def default(self, obj):
            if isinstance(obj, (types.ID,)):
                return str(obj)
            if isinstance(obj, datetime):
                return obj.isoformat()
            return super().default(obj)

    return TestJsonEncoder().encode


def base_currencies(dbpool):
    with dbpool.connection() as conn:
        conn.execute(
            """
            insert into currency (code) values ('USD'), ('CAD'), ('MSFT')
            on conflict (code) do nothing
            """
        )


def base_tenant_name():
    return "test"


def base_tenant(dbpool, sql, base_tenant_name):
    with dbpool.connection() as conn:
        return sql.select_one(
            conn,
            sql.queries.INSERT("tenant", ["name"], returning=True),
            {"name": base_tenant_name},
            Constructor=model.Tenant,
        )


def run_id():
    return types.ID()


def test_accounts(dbpool, sql, base_tenant, run_id):
    """
    Provide a set of accounts that is scoped to this particular test function.
    """
    accounts = {}
    with dbpool.connection() as conn:
        # map accounts by basename
        for basename, normal in [
            ("Asset", "DR"),
            ("Expense", "DR"),
            ("Liability", "CR"),
            ("Income", "CR"),
            ("Equity", "CR"),
        ]:
            name = f"{basename}{run_id and '-' or ''}{run_id}"
            acct = sql.select_one(
                conn,
                sql.queries.INSERT(
                    "account", ["tenant_id", "name", "normal"], returning=True
                ),
                {"tenant_id": base_tenant.id, "name": name, "normal": normal},
                Constructor=model.Account,
            )
            accounts[basename] = acct

    yield accounts


def test_transactions(dbpool, sql, test_accounts, base_tenant):
    accts = test_accounts
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
                sql.queries.INSERT(
                    "transaction", ["memo", "tenant_id"], returning=True
                ),
                {"memo": item["memo"], "tenant_id": base_tenant.id},
            )
            tx["entries"] = []
            for new_entry in item["entries"]:
                new_entry |= {"tx": tx["id"], "tenant_id": base_tenant.id}
                entry = sql.select_one(
                    conn,
                    sql.queries.INSERT("entry", new_entry, returning=True),
                    new_entry,
                )
                tx["entries"].append(entry)
                sql.execute(
                    conn,
                    sql.queries.UPDATE("account", ["version"], [sql.Q.filter("id")]),
                    {"id": entry["acct"], "version": entry["id"]},
                )
            transactions.append(tx)

    yield transactions
