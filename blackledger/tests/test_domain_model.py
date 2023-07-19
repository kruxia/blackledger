import pytest
from blackledger.domain import model, types
from blackledger.tests import parameters

def test_account_initialize_currency():
    """
    Initializing an Account with a currency string is automatically converted to a
    Currency.
    """
    account = model.Account(name="bank", normal=types.Normal.CR, currency="USD")
    assert isinstance(account.currency, types.Currency)

def test_amount_initialize_currency():
    """
    Initializing an Amount with a currency string is automatically converted to a
    Currency.
    """
    amount = model.Amount(decimal="3.50", currency="USD")
    assert isinstance(amount.currency, types.Currency)


@pytest.mark.parametrize('entries', parameters.valid_transaction_entries)
def test_transaction_currency(entries):
    """
    + A Transaction's currency is the currency of the first entry.
    + A Transaction with no entries has no currency
    """
    tx = model.Transaction(entries=entries)
    assert tx.currency() == tx.entries[0].amount.currency

    tx = model.Transaction(entries=[])
    assert tx.currency() is None
