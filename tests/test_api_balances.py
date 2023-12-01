from http import HTTPStatus


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
