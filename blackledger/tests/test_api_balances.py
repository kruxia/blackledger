from http import HTTPStatus

import pytest

from blackledger import types

# -- GET BALANCES --


def test_get_balances_ok(client, test_transactions):
    # balances expected from test_transactions
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


@pytest.mark.parametrize(
    "query, keys",
    [
        ("?_orderby=id", {"Asset", "Expense", "Income", "Equity"}),
        ("?normal=DR", {"Asset", "Expense"}),
        # NOTE: the following succeed because we are ordering the ids desc, which selects
        # this test_transactions accounts first. See TODO below.
        ("?_orderby=-id&_limit=1&normal=CR", {"Equity"}),
        (f"?ledger={types.new_bigid()}", set()),
        # NOTE: the following still fails - offset is unreliable with other
        # test_transactions.
        # ("?_orderby=-id&_offset=1&_limit=1&normal=CR", {"Income"}),
    ],
)
def test_get_balances_filters_ok(client, test_transactions, query, keys):
    # TODO: As it stands, the filter will return accounts from other test cases. This is
    # fine as long as the test account names are consistent. It would be better to
    # filter the results to only the accounts in this run of test_transactions.

    # # Add to the query a filter for the account ids (to avoid conflict w/other tests)
    # transaction_account_ids = set(
    #     str(e["acct"]) for t in test_transactions for e in t["entries"]
    # )
    # print(transaction_account_ids)
    # query += f"&id={','.join(transaction_account_ids)}"

    print(f"{query=}")
    response = client.get(f"/api/accounts/balances{query}")
    print(response.json())
    response_account_keys = {
        item["account"]["name"].split("-")[0] for item in response.json()
    }
    assert response.status_code == HTTPStatus.OK
    assert response_account_keys == keys
