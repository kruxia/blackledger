from http import HTTPStatus

import pytest

from blackledger.domain import model


@pytest.fixture
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


@pytest.mark.parametrize(
    "query, names",
    [
        ("", ["Asset", "Expense", "Liability", "Income", "Equity"]),
        # -- FILTERS --
        ("?normal=DR", ["Asset", "Expense"]),
        ("?normal=CR", ["Liability", "Income", "Equity"]),
        ("?name=e", ["Asset", "Expense", "Income", "Equity"]),
        ("?name=^E", ["Expense", "Equity"]),
        # -- SEARCH PARAMS --
        ("?_orderby=number", ["Asset", "Liability", "Equity", "Income", "Expense"]),
        ("?_orderby=-number", ["Expense", "Income", "Equity", "Liability", "Asset"]),
        (
            "?_orderby=normal,-name",
            ["Liability", "Income", "Equity", "Expense", "Asset"],
        ),
        ("?_orderby=name", ["Asset", "Equity", "Expense", "Income", "Liability"]),
        ("?_orderby=name&_limit=2", ["Asset", "Equity"]),
        ("?_orderby=name&_offset=3", ["Income", "Liability"]),
        ("?_orderby=name&_offset=3&_limit=1", ["Income"]),
    ],
)
def test_search_accounts(client, base_accounts, query, names):
    response = client.get(f"/api/accounts{query}")
    assert response.status_code == HTTPStatus.OK
    data = response.json()
    response_names = [a["name"] for a in data]
    assert response_names == names


@pytest.mark.parametrize("name, updates", [("Asset", {"number": 100})])
def test_update_accounts_ok(client, base_accounts, name, updates):
    account = base_accounts[name]
    data = account.model_dump(exclude_none=True) | updates
    print(f"{data=}")
    response = client.post("/api/accounts", json=data)
    print(f"{response.json()=}")
    assert response.status_code == HTTPStatus.OK
