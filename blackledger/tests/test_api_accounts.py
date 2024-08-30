import logging
from http import HTTPStatus

import pytest

from blackledger import model, types

LOG = logging.getLogger(__name__)


@pytest.fixture(scope="session")
def base_accounts(dbpool, sql, base_ledger):
    accounts = {}
    with dbpool.connection() as conn:
        # map accounts by basename
        for name, number, normal in [
            ("Asset", 1000, "DR"),
            ("Expense", 5000, "DR"),
            ("Liability", 2000, "CR"),
            ("Income", 4000, "CR"),
            ("Equity", 3000, "CR"),
        ]:
            accounts |= {
                name: sql.select_one(
                    conn,
                    sql.queries.INSERT(
                        "account",
                        ["ledger_id", "name", "number", "normal"],
                        returning=True,
                    ),
                    {
                        "ledger_id": base_ledger.id,
                        "name": name,
                        "number": number,
                        "normal": normal,
                    },
                    Constructor=model.Account,
                )
            }
    yield accounts


# -- SEARCH ACCOUNTS --


@pytest.mark.parametrize(
    "query, expected_names",
    [
        ("", ["Asset", "Expense", "Liability", "Income", "Equity"]),
        # -- FILTERS --
        ("?normal=DR", ["Asset", "Expense"]),
        ("?normal=CR", ["Liability", "Income", "Equity"]),
        ("?name=e", ["Asset", "Expense", "Income", "Equity"]),
        ("?name=^E", ["Expense", "Equity"]),
        (f"?ledger={types.new_bigid()}", []),
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
def testsearch_accounts(client, base_accounts, query, expected_names):
    response = client.get(f"/api/accounts{query}")
    print(f"{response.json()=}")
    assert response.status_code == HTTPStatus.OK
    data = response.json()
    response_names = [a["name"] for a in data]

    assert response_names == expected_names


def testsearch_accounts_unmatched_ledger(client, base_accounts):
    response = client.get(f"/api/accounts?ledger={types.new_bigid()}")
    print(f"{response.json()=}")
    assert response.status_code == HTTPStatus.OK
    items = response.json()
    assert len(items) == 0


@pytest.mark.parametrize(
    "query, status_code",
    [
        ("?normal=FR", HTTPStatus.UNPROCESSABLE_ENTITY),
        ("?version=NOT_AN_ID", HTTPStatus.UNPROCESSABLE_ENTITY),
        ("?name=No@punctuation$allowed", HTTPStatus.UNPROCESSABLE_ENTITY),
        ("?ledger=NOT_AN_ID", HTTPStatus.UNPROCESSABLE_ENTITY),
        ("?parent=NOT_AN_ID", HTTPStatus.UNPROCESSABLE_ENTITY),
        ("?number=INTS_ONLY", HTTPStatus.UNPROCESSABLE_ENTITY),
    ],
)
def testsearch_accounts_fail(client, base_accounts, query, status_code):
    response = client.get(f"/api/accounts{query}")
    print(response.status_code, response.json())
    assert response.status_code == status_code


# -- EDIT ACCOUNT --


@pytest.mark.parametrize(
    "name, updates",
    [
        # number can be changed
        ("Asset", {"number": 100}),
        # number can be int-castable
        ("Asset", {"number": "100"}),
        # name can be changed
        ("Asset", {"name": "Assets"}),
    ],
)
def test_update_account_ok(client, json_dumps, base_accounts, name, updates):
    account = base_accounts[name]
    data = account.model_dump(exclude_none=True) | updates

    response = client.post("/api/accounts", content=json_dumps(data))
    assert response.status_code == HTTPStatus.OK


@pytest.mark.parametrize(
    "name, updates",
    [
        # name must be a Name -- not empty
        ("Asset", {"name": ""}),
        # name must be a Name -- pattern must match
        ("Asset", {"name": "Foo,Bar"}),
        # number must be an int or int-castable
        ("Asset", {"number": "34.5"}),
        ("Asset", {"number": 34.5}),
        # normal must be DR or CR
        ("Asset", {"normal": "FR"}),
    ],
)
def test_update_account_unprocessable(client, json_dumps, base_accounts, name, updates):
    account = base_accounts[name]
    data = account.model_dump(exclude_none=True) | updates

    response = client.post("/api/accounts", content=json_dumps(data))
    print(response.json())
    assert response.status_code == HTTPStatus.UNPROCESSABLE_ENTITY


@pytest.mark.parametrize(
    "name, updates",
    [
        # id cannot be changed
        ("Asset", {"id": types.new_bigid()}),
        # normal cannot be changed
        ("Asset", {"normal": "CR"}),
        # parent_id must exist if given
        ("Asset", {"parent_id": types.new_bigid()}),
    ],
)
def test_update_account_conflict(client, json_dumps, base_accounts, name, updates):
    account = base_accounts[name]
    data = account.model_dump(exclude_none=True) | updates

    response = client.post("/api/accounts", content=json_dumps(data))
    assert response.status_code == HTTPStatus.CONFLICT


# -- CREATE ACCOUNT --


@pytest.mark.parametrize(
    "item",
    [
        # id must be int
        {"id": "abc", "name": "Foo", "normal": "DR"},
        {"id": 123, "name": "Foo", "normal": "DR"},
        # name is required and must be model.Name
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
    response = client.post("/api/accounts", content=json_dumps(item))
    assert response.status_code == HTTPStatus.UNPROCESSABLE_ENTITY
