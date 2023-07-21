import pytest

from blackledger.domain import model, types
from blackledger.tests import parameters


@pytest.mark.skip("need a better approach to this problem")
@pytest.mark.parametrize("entries", parameters.valid_transaction_entries)
def test_model_de_serialize(entries):
    """
    Deserializing and Serializing a model (json) should roundtrip.
    """
    item = model.Transaction(entries=entries)
    item_entries = item.dict()["entries"]
    for entry, item_entry in zip(entries, item_entries):
        item_entry = {k: v for k, v in item_entry.items() if k in entry}
        item_entry["account"] = {
            k: v for k, v in item_entry["account"].items() if k in entry["account"]
        }
        print(f"{entry=}")
        print(f"{item_entry=}")
        assert entry == item_entry


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

    amount = model.Amount(decimal="3.50", currency=types.Currency("USD"))
    assert isinstance(amount.currency, types.Currency)


@pytest.mark.parametrize("entries", parameters.valid_transaction_entries)
def test_transaction_currency(entries):
    """
    + A Transaction's currency is the currency of the first entry.
    + A Transaction with no entries has no currency
    """
    tx = model.Transaction(entries=entries)
    assert tx.currency() == tx.entries[0].amount.currency

    tx = model.Transaction(entries=[])
    assert tx.currency() is None
