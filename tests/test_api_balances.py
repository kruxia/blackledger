from http import HTTPStatus
import pytest
from blackledger.domain import model, types

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


def test_get_balances_ok(client, balance_transactions):
    # balances expected from balance_transactions
    expected_balances = {
        "Asset": {"MSFT": "5", "USD": "577"},
        "Expense": {"CAD": "48"},
        "Income": {"USD": "2500"},
        "Equity": {"CAD": "48", "MSFT": "5", "USD": "-1923"},
    }
    response = client.get("/api/accounts/balances")
    response_accounts = {
        # key is basename
        item["account"]["name"].split("-")[0]: item
        for item in response.json()
    }
    assert response.status_code == HTTPStatus.OK
    for name, item in response_accounts.items():
        assert expected_balances[name] == item["balances"]
