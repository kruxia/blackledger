from http import HTTPStatus

import pytest

from blackledger.domain import model, types


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


@pytest.mark.parametrize(
    "query, expected_names",
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
def test_search_accounts(client, base_accounts, query, expected_names):
    response = client.get(f"/api/accounts{query}")
    assert response.status_code == HTTPStatus.OK
    data = response.json()
    response_names = [a["name"] for a in data]
    assert response_names == expected_names


@pytest.mark.parametrize(
    "name, updates",
    [
        # number can be changed
        ("Asset", {"number": 100}),
        # number can be int-castable
        ("Asset", {"number": "100"}),
        # name can be changed
        ("Asset", {"name": "Assets"}),
        # change it back
        ("Assets", {"name": "Asset"}),
    ],
)
def test_update_account_ok(client, json_dumps, base_accounts, name, updates):
    account = base_accounts[name]
    data = account.model_dump(exclude_none=True) | updates

    response = client.post("/api/accounts", data=json_dumps(data))
    assert response.status_code == HTTPStatus.OK


@pytest.mark.parametrize(
    "name, updates",
    [
        # name must be a NameString -- not empty
        ("Asset", {"name": ""}),
        # name must be a NameString -- pattern must match
        ("Asset", {"name": "Foo,Bar"}),
        # number must be an int or int-castable
        ("Asset", {"number": "34.5"}),
        ("Asset", {"number": 34.5}),
    ],
)
def test_update_account_unprocessable(client, json_dumps, base_accounts, name, updates):
    account = base_accounts[name]
    data = account.model_dump(exclude_none=True) | updates

    response = client.post("/api/accounts", data=json_dumps(data))
    assert response.status_code == HTTPStatus.UNPROCESSABLE_ENTITY


@pytest.mark.parametrize(
    "name, updates",
    [
        # id cannot be changed
        ("Asset", {"id": types.ID()}),
        # normal cannot be changed
        ("Asset", {"normal": "CR"}),
        # parent_id must exist if given
        ("Asset", {"parent_id": types.ID()}),
    ],
)
def test_update_account_conflict(client, json_dumps, base_accounts, name, updates):
    account = base_accounts[name]
    data = account.model_dump(exclude_none=True) | updates

    response = client.post("/api/accounts", data=json_dumps(data))
    assert response.status_code == HTTPStatus.CONFLICT


@pytest.mark.parametrize(
    "item",
    [
        # id must be types.ID
        {"id": "abc", "name": "Foo", "normal": "DR"},
        {"id": 123, "name": "Foo", "normal": "DR"},
        # name is required and must be model.NameString
        {"name": "", "normal": "CR"},
        {"name": "Foo,Bar", "normal": "CR"},
        # number must be an int or int-castable
        {"number": "34.5", "normal": "CR"},
        {"number": 34.5, "normal": "CR"},
        # normal must be CR or DR
        {"name": "Foo", "normal": "FR"},
    ],
)
def test_create_account_invalid(client, json_dumps, base_accounts, item):
    response = client.post("/api/accounts", data=json_dumps(item))
    assert response.status_code == HTTPStatus.UNPROCESSABLE_ENTITY
